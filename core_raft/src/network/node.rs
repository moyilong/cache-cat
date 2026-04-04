use crate::network::raft_type::{CacheCatApp, GroupId, NodeId, Raft};
use crate::network::router::{MultiNetworkFactory, Router};
use crate::server::core::config::{Config, GROUP_NUM};
use crate::store::log_store::LogStore;
use crate::store::raft_engine::create_raft_engine;
use crate::store::store::StateMachineStore;
use openraft::SnapshotPolicy::Never;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Node {
    pub config: Config,
    pub groups: HashMap<GroupId, CacheCatApp>,
}
impl Node {
    pub async fn create_node(app_config: &Config) -> Node {
        let dir = Path::new(&app_config.raft.log_path);
        let path = dir.join("");
        let mut node = Node {
            config: app_config.clone(),
            groups: HashMap::new(),
        };
        let raft_engine = dir.join("raft-engine");
        let engine = create_raft_engine(raft_engine.clone());
        let config = Arc::new(openraft::Config {
            heartbeat_interval: 250,
            election_timeout_min: 299,
            election_timeout_max: 599, // 添加最大选举超时时间
            purge_batch_size: 1,
            max_in_snapshot_log_to_keep: 500, //生成快照后要保留的日志数量（以供从节点同步数据）需要大于等于replication_lag_threshold,该参数会影响快照逻辑
            max_append_entries: Some(5000000),
            max_payload_entries: 5000000,
            snapshot_policy: Never,         //LogsSinceLast(100),
            replication_lag_threshold: 200, //需要大于snapshot_policy
            ..Default::default()
        });
        for i in 0..GROUP_NUM {
            let group_id = i as GroupId;
            // let engine_path = dir.as_ref().join(format!("raft-engine-{}", group_id));
            // let engine = create_raft_engine(engine_path);
            let router = Router::new(
                app_config.raft.address.clone(),
                dir.join(""),
                app_config.node_id,
            );
            let network = MultiNetworkFactory::new(router, group_id);
            let log_store = LogStore::new(group_id, engine.clone());
            let sm_store = StateMachineStore::new(path.clone(), group_id, app_config.node_id)
                .await
                .unwrap();
            let raft = openraft::Raft::new(
                app_config.node_id,
                config.clone(),
                network,
                log_store,
                sm_store.clone(),
            )
            .await
            .unwrap();
            node.add_group(
                &*app_config.raft.address,
                group_id,
                raft,
                sm_store,
                dir.join(""),
            )
        }
        node
    }

    pub fn add_group(
        &mut self,
        addr: &str,
        group_id: GroupId,
        raft: Raft,
        state_machine: StateMachineStore,
        path: PathBuf,
    ) {
        let app = CacheCatApp {
            node_id: self.config.node_id,
            addr: addr.to_string(),
            raft,
            group_id,
            state_machine,
            path,
        };
        self.groups.insert(group_id, app);
    }
}
