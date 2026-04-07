use crate::config::config::Config;
use crate::error::Result;
use crate::node::node::RaftNode;

use std::sync::Arc;

pub struct RaftNodeBuilder;

impl RaftNodeBuilder {
    /// Build a new RaftNode with the given configuration
    pub async fn build(config: &Config) -> Result<Arc<RaftNode>> {
        config.validate()?;
        let raft_node = RaftNode::create(config).await?;
        let arc = Arc::from(raft_node);
        RaftNode::start(arc.clone()).await?;
        Ok(arc)
    }
}
