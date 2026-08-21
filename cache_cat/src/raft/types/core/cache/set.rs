use crate::error::ProtocolError;
use crate::protocol::key::del::DelReq;
use crate::protocol::set::sadd::SAddReq;
use crate::protocol::set::sdiffstore::SDiffStoreParams;
use crate::protocol::set::sinterstore::SInterStoreParams;
use crate::protocol::set::srem::SRemReq;
use crate::protocol::set::sunionstore::SUnionStoreParams;
use crate::raft::types::core::mocha::core::{MyCache, Update};
use crate::raft::types::core::response_value::Value;
use crate::raft::types::core::value_object::ValueObject;
use bytes::Bytes;
use std::collections::HashSet;

impl MyCache {
    pub fn s_rem(&self, param: SRemReq, update: &mut Update) -> Value {
        self.execute_compute(param, update)
    }

    pub fn s_add(&self, param: SAddReq, update: &mut Update) -> Value {
        self.execute_compute(param, update)
    }

    pub fn redis_sinterstore(
        &self,
        param: SInterStoreParams,
        update: &mut Update<'_>,
        external: bool,
    ) -> Value {
        let _exclusive_lock = if external {
            Some(self.read_lock.write())
        } else {
            None
        };

        let cache = match self.get_cache(update.db_number) {
            Err(err) => return err,
            Ok(cache) => cache,
        };

        // use `HashSet` for quickly remove
        let mut results: Option<HashSet<Bytes>> = None;
        for key in param.keys {
            let Some(value) = cache.mocha.get_entry(&key) else {
                // the key not exists, set empty.
                match results {
                    None => results = Some(Default::default()),
                    Some(ref mut set) => set.clear(),
                }

                continue;
            };

            let ValueObject::Set(data) = value.value.data else {
                return ProtocolError::InvalidArgument("There is a value that is not a set").into();
            };

            match results {
                // init the result set
                None => results = Some(data.lock().clone()),

                // continue for check the value types
                // and skip the lock
                Some(ref set) if set.is_empty() => continue,

                Some(ref mut set) => {
                    let data = data.lock();
                    set.retain(|v| data.contains(v));
                }
            }
        }

        let elements = results
            .map(|res| res.into_iter().collect())
            .unwrap_or_default();

        let key = param.key;

        if cache.mocha.get_entry(&key).is_some() {
            let del = DelReq { key: key.clone() };

            self.del(del, update);
        }

        let add = SAddReq { key, elements };
        self.s_add(add, update)
    }

    pub fn redis_sunionstore(
        &self,
        param: SUnionStoreParams,
        update: &mut Update<'_>,
        external: bool,
    ) -> Value {
        let _exclusive_lock = if external {
            Some(self.read_lock.write())
        } else {
            None
        };

        let cache = match self.get_cache(update.db_number) {
            Err(err) => return err,
            Ok(cache) => cache,
        };

        // use `HashSet` for quickly insert
        let mut results: HashSet<Bytes> = Default::default();
        for key in param.keys {
            let Some(value) = cache.mocha.get_entry(&key) else {
                continue;
            };

            let ValueObject::Set(data) = value.value.data else {
                return ProtocolError::InvalidArgument("There is a value that is not a set").into();
            };

            results.extend(data.lock().iter().cloned());
        }

        let elements = results.into_iter().collect();

        let key = param.key;

        if cache.mocha.get_entry(&key).is_some() {
            let del = DelReq { key: key.clone() };

            self.del(del, update);
        }

        let add = SAddReq { key, elements };
        self.s_add(add, update)
    }

    pub fn redis_sdiffstore(
        &self,
        param: SDiffStoreParams,
        update: &mut Update<'_>,
        external: bool,
    ) -> Value {
        let _exclusive_lock = if external {
            Some(self.read_lock.write())
        } else {
            None
        };

        let cache = match self.get_cache(update.db_number) {
            Err(err) => return err,
            Ok(cache) => cache,
        };

        let mut keys = param.keys.into_iter();

        // use `HashSet` for quickly remove
        let mut results = if let Some(key) = keys.next() {
            if let Some(value) = cache.mocha.get_entry(&key) {
                let ValueObject::Set(data) = value.value.data else {
                    return ProtocolError::InvalidArgument("There is a value that is not a set")
                        .into();
                };

                Some(data.lock().clone())
            } else {
                None
            }
        } else {
            return ProtocolError::WrongArgCount("wrong number of arguments for command").into();
        };

        for key in keys {
            let Some(value) = cache.mocha.get_entry(&key) else {
                continue;
            };

            let ValueObject::Set(data) = value.value.data else {
                return ProtocolError::InvalidArgument("There is a value that is not a set").into();
            };

            let Some(ref mut results) = results else {
                continue;
            };

            let data = data.lock();
            results.retain(|v| !data.contains(v));
        }

        let elements = results
            .map(|res| res.into_iter().collect())
            .unwrap_or_default();

        let key = param.key;

        if cache.mocha.get_entry(&key).is_some() {
            let del = DelReq { key: key.clone() };

            self.del(del, update);
        }

        let add = SAddReq { key, elements };
        self.s_add(add, update)
    }
}
