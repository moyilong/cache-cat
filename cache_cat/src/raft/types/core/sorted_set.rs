use std::collections::{BTreeSet, HashMap};
use bytes::Bytes;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use crate::protocol::zset::zadd::ZAddReq;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SortedSet {
    /// 按 (score, member) 排序。
    ///
    /// Redis Sorted Set 在 score 相同时，按照 member 的字典序排列。
    tree: BTreeSet<(OrderedFloat<f64>, Bytes)>,

    /// member -> score
    ///
    /// 用于 O(1) 查询 member 是否存在以及它当前的 score。
    hash: HashMap<Bytes, f64>,
}

impl SortedSet {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn zadd(&mut self, req: ZAddReq) -> i64 {
        let mut added = 0;
        let mut changed = 0;

        for (member, score) in req.members {
            let old_score = self.hash.get(&member).copied();
            let exists = old_score.is_some();

            // NX: 只添加不存在的 member
            if req.nx && exists {
                continue;
            }

            // XX: 只更新已经存在的 member
            if req.xx && !exists {
                continue;
            }

            if let Some(old_score) = old_score {
                // GT: 新 score 必须大于旧 score
                if req.gt && score <= old_score {
                    continue;
                }

                // LT: 新 score 必须小于旧 score
                if req.lt && score >= old_score {
                    continue;
                }

                // score 真正发生改变才需要修改 tree/hash
                if old_score != score {
                    self.tree
                        .remove(&(OrderedFloat(old_score), member.clone()));

                    self.tree
                        .insert((OrderedFloat(score), member.clone()));

                    self.hash.insert(member, score);

                    changed += 1;
                }
            } else {
                // 新 member
                self.tree
                    .insert((OrderedFloat(score), member.clone()));

                self.hash.insert(member, score);

                added += 1;
                changed += 1;
            }
        }

        if req.ch {
            changed
        } else {
            added
        }
    }

    /// 按 rank 返回成员。
    ///
    /// start/stop 都是闭区间，并支持 Redis 风格负数索引：
    ///
    /// - 0 = 第一个
    /// - 1 = 第二个
    /// - -1 = 最后一个
    /// - -2 = 倒数第二个
    pub fn zrange(&self, start: i64, stop: i64) -> Vec<(Bytes, f64)> {
        let len = self.tree.len() as i64;

        if len == 0 {
            return Vec::new();
        }

        let mut start_idx = if start < 0 {
            len + start
        } else {
            start
        };

        let mut stop_idx = if stop < 0 {
            len + stop
        } else {
            stop
        };

        // Redis 语义：
        // start 过小则修正到 0
        if start_idx < 0 {
            start_idx = 0;
        }

        // stop 超出末尾则修正到 len - 1
        if stop_idx >= len {
            stop_idx = len - 1;
        }

        // stop 仍然 < 0，说明整个范围都在集合之前
        if stop_idx < 0 {
            return Vec::new();
        }

        if start_idx >= len || start_idx > stop_idx {
            return Vec::new();
        }

        let count = (stop_idx - start_idx + 1) as usize;

        self.tree
            .iter()
            .skip(start_idx as usize)
            .take(count)
            .map(|(score, member)| (member.clone(), score.0))
            .collect()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.hash.len()
    }

    /// 统计 score 在指定范围中的 member 数量。
    ///
    /// min_exclusive:
    /// - false => score >= min
    /// - true  => score > min
    ///
    /// max_exclusive:
    /// - false => score <= max
    /// - true  => score < max
    pub fn zcount(
        &self,
        min: f64,
        max: f64,
        min_exclusive: bool,
        max_exclusive: bool,
    ) -> i64 {
        if self.tree.is_empty() {
            return 0;
        }

        if min > max {
            return 0;
        }

        self.tree
            .iter()
            .filter(|(score, _)| {
                let score = score.0;

                let min_ok = if min_exclusive {
                    score > min
                } else {
                    score >= min
                };

                let max_ok = if max_exclusive {
                    score < max
                } else {
                    score <= max
                };

                min_ok && max_ok
            })
            .count() as i64
    }

    /// ZRANGEBYSCORE
    ///
    /// min/max 当前为闭区间：
    ///
    ///     min <= score <= max
    ///
    /// limit:
    ///
    ///     Some((offset, count))
    pub fn zrangebyscore(
        &self,
        min: f64,
        max: f64,
        limit: Option<(usize, usize)>,
    ) -> Vec<(Bytes, f64)> {
        if self.tree.is_empty() {
            return Vec::new();
        }

        if min > max {
            return Vec::new();
        }

        let skip_count = limit
            .map(|(offset, _)| offset)
            .unwrap_or(0);

        let take_count = limit
            .map(|(_, count)| count)
            .unwrap_or(usize::MAX);

        if take_count == 0 {
            return Vec::new();
        }

        /*
         * BTreeSet 的 key 是：
         *
         *     (score, member)
         *
         * Bytes::new() 是所有非空 Bytes 的最小值，因此可以从：
         *
         *     (min, "")
         *
         * 开始 range。
         *
         * 不能写：
         *
         *     ..=(max, Bytes::new())
         *
         * 因为这会漏掉：
         *
         *     (max, "abc")
         *     (max, "xyz")
         *
         * 所以这里只设置下界，然后通过 take_while 控制 score 上界。
         */
        let start = (OrderedFloat(min), Bytes::new());

        self.tree
            .range(start..)
            .take_while(|(score, _)| score.0 <= max)
            .skip(skip_count)
            .take(take_count)
            .map(|(score, member)| (member.clone(), score.0))
            .collect()
    }

    /// 删除指定成员。
    ///
    /// 时间复杂度：
    ///
    /// O(M * log N)
    ///
    /// M = members 数量
    pub fn zrem(&mut self, members: &[Bytes]) -> i64 {
        let mut removed = 0i64;

        for member in members {
            if let Some(score) = self.hash.remove(member) {
                self.tree
                    .remove(&(OrderedFloat(score), member.clone()));

                removed += 1;
            }
        }

        removed
    }

    /// ZINCRBY
    ///
    /// member 不存在时，相当于从 0 开始增加。
    pub fn zincrby(
        &mut self,
        member: Bytes,
        increment: f64,
    ) -> Option<f64> {
        let old_score = self.hash.get(&member).copied();

        let new_score = old_score.unwrap_or(0.0) + increment;

        // Redis 不允许 NaN score。
        if new_score.is_nan() {
            return None;
        }

        if let Some(old_score) = old_score {
            // 如果 score 没变化，就不需要重新插入 tree。
            if old_score == new_score {
                return Some(new_score);
            }

            self.tree
                .remove(&(OrderedFloat(old_score), member.clone()));
        }

        self.tree
            .insert((OrderedFloat(new_score), member.clone()));

        self.hash.insert(member, new_score);

        Some(new_score)
    }

    /// ZSCORE
    #[inline]
    pub fn zscore(&self, member: &Bytes) -> Option<f64> {
        self.hash.get(member).copied()
    }

    /// ZRANK
    ///
    /// 当前复杂度仍然是 O(N)。
    pub fn zrank(&self, member: &Bytes) -> Option<i64> {
        let score = self.hash.get(member).copied()?;

        /*
         * 已经知道 score，所以不需要像原代码一样：
         *
         * self.tree.iter().position(|(_, m)| m == member)
         *
         * 从逻辑上搜索 member。
         *
         * 不过 std::collections::BTreeSet 不提供 order-statistics，
         * 所以获取 rank 本身仍需要迭代，复杂度 O(N)。
         */
        let key = (OrderedFloat(score), member.clone());

        self.tree
            .iter()
            .position(|item| item == &key)
            .map(|rank| rank as i64)
    }

    /// ZREVRANK
    ///
    /// 当前复杂度 O(N)。
    pub fn zrevrank(&self, member: &Bytes) -> Option<i64> {
        let score = self.hash.get(member).copied()?;

        let key = (OrderedFloat(score), member.clone());

        self.tree
            .iter()
            .rev()
            .position(|item| item == &key)
            .map(|rank| rank as i64)
    }

    /// 检查集合是否为空。
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.hash.is_empty()
    }

    /// ZPOPMIN
    pub fn zpop_min(
        &mut self,
        count: Option<usize>,
    ) -> Vec<(Bytes, f64)> {
        let count = match count {
            None => 1,
            Some(0) => return Vec::new(),
            Some(count) => count,
        };

        let mut values = Vec::with_capacity(
            count.min(self.tree.len())
        );

        for _ in 0..count {
            let (score, member) = match self.tree.pop_first() {
                Some(value) => value,
                None => break,
            };

            self.hash.remove(&member);

            values.push((member, score.0));
        }

        values
    }

    /// 估算 SortedSet 额外占用的 heap 内存。
    ///
    /// 注意：
    /// - 不包含 `size_of::<SortedSet>()`
    /// - `samples == 0` 表示统计全部元素
    /// - `samples > 0` 表示采样指定数量，然后估算整体 payload
    ///
    /// 当前 SortedSet 结构：
    ///
    /// tree: BTreeSet<(OrderedFloat<f64>, Bytes)>
    /// hash: HashMap<Bytes, f64>
    ///
    /// 同一个 member 在 tree/hash 中的 Bytes 是 clone 关系，
    /// Bytes::clone() 通常共享 backing storage，
    /// 所以 member 的实际字节 payload 只计算一次。
    pub fn estimated_heap_usage(&self, samples: usize) -> usize {
        let len = self.len();

        /*
         * ------------------------------------------------------------
         * HashMap heap
         * ------------------------------------------------------------
         *
         * HashMap 的 struct 本身属于 SortedSet inline 内存，
         * 这里计算它动态分配的 buckets。
         */
        let hash_bucket_size = size_of::<(Bytes, f64)>()
            .saturating_add(1);

        let hash_memory = self
            .hash
            .capacity()
            .saturating_mul(hash_bucket_size);

        if len == 0 {
            return hash_memory;
        }

        /*
         * ------------------------------------------------------------
         * BTreeSet heap
         * ------------------------------------------------------------
         *
         * std::collections::BTreeSet 没有公开 capacity，
         * 也没有公开节点真实 allocation size。
         *
         * 因此只能近似：
         *
         * 每个 entry：
         *
         *     OrderedFloat<f64>
         *     Bytes
         *
         * 再加一些 BTree node 指针/metadata 开销。
         */
        let tree_entry_size =
            size_of::<(OrderedFloat<f64>, Bytes)>();

        /*
         * BTree 内部还有 node pointer / metadata。
         *
         * 这里每个元素额外按两个 usize 估算。
         *
         * 这是近似值，不是 allocator 精确值。
         */
        let tree_overhead_per_entry =
            size_of::<usize>().saturating_mul(2);

        let tree_memory = len.saturating_mul(
            tree_entry_size
                .saturating_add(tree_overhead_per_entry),
        );

        /*
         * ------------------------------------------------------------
         * Bytes payload
         * ------------------------------------------------------------
         *
         * tree:
         *
         *     member.clone()
         *
         * hash:
         *
         *     member
         *
         * Bytes handle 有两份，但是 backing bytes 通常共享，
         * 所以真正 member.len() 只计算一次。
         */
        let sample_count = if samples == 0 {
            len
        } else {
            samples.min(len)
        };

        let member_payload = if sample_count == 0 {
            0
        } else {
            let sampled_bytes = self
                .tree
                .iter()
                .take(sample_count)
                .fold(
                    0usize,
                    |total, (_, member)| {
                        total.saturating_add(member.len())
                    },
                );

            if sample_count >= len {
                sampled_bytes
            } else {
                /*
                 * sampled average * total count
                 *
                 * 用 u128 防止：
                 *
                 * sampled_bytes * len
                 *
                 * 在 usize 上溢出。
                 */
                let estimated =
                    (sampled_bytes as u128)
                        .saturating_mul(len as u128)
                        / sample_count as u128;

                estimated.min(usize::MAX as u128) as usize
            }
        };

        hash_memory
            .saturating_add(tree_memory)
            .saturating_add(member_payload)
    }
}