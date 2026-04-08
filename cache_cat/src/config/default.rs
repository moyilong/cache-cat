use crate::config::config::RaftConfig;

pub fn default_raft_config() -> RaftConfig {
    RaftConfig {
        log_path: ".data".to_string(),
        address: "127.0.0.1:6682".to_string(),
        advertise_host: "localhost".to_string(),
        single: true,
        join: vec![],
    }
}
