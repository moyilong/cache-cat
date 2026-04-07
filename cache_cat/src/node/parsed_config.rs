use crate::config::config::Config;
use crate::error::Result;
use crate::raft::types::endpoint::Endpoint;

pub struct ParsedConfig {
    pub node_id: u64,

    pub raft_endpoint: Endpoint,

    pub raft_advertise_endpoint: Endpoint,

    pub raft_single: bool,

    pub raft_join: Vec<String>,

    #[allow(dead_code)]
    pub rocksdb_data_path: String,
}

impl ParsedConfig {
    pub fn from(config: &Config) -> Result<Self> {
        let raft_endpoint = Endpoint::parse(&config.raft.address)?;
        let raft_advertise_endpoint =
            Endpoint::new(&config.raft.advertise_host, raft_endpoint.port());

        Ok(ParsedConfig {
            node_id: config.node_id,
            raft_endpoint,
            raft_advertise_endpoint,
            raft_single: config.raft.single,
            raft_join: config.raft.join.clone(),
            rocksdb_data_path: config.raft.data_path.clone(),
        })
    }
}
