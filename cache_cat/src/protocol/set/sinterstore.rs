use crate::error::{CacheCatError, ProtocolError};
use crate::protocol::command::{Client, Command};
use crate::protocol::raft_command::RaftCommand;
use crate::raft::network::redis_server::RedisServer;
use crate::raft::types::core::response_value::Value;
use crate::raft::types::entry::request::Operation;
use crate::raft::types::entry::request::RedisOperation::RedisSInterStore;
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Parameters for SINTERSTORE command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SInterStoreParams {
    pub key: Bytes,
    pub keys: Vec<Bytes>,
}

impl Display for SInterStoreParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SINTERSTORE")?;
        write!(f, " {}", String::from_utf8_lossy(&self.key))?;
        for key in &self.keys {
            write!(f, " {}", String::from_utf8_lossy(key))?;
        }
        Ok(())
    }
}

/// SINTERSTORE command executor
pub struct SInterStoreCommand;

impl SInterStoreCommand {
    fn parse(items: &[Value]) -> Result<SInterStoreParams, ProtocolError> {
        if items.len() < 3 {
            return Err(ProtocolError::WrongArgCount("SINTERSTORE"));
        }

        let key = items[1]
            .string_bytes_clone()
            .ok_or(ProtocolError::InvalidArgument("key"))?;

        let keys = items
            .iter()
            .skip(2)
            .map_while(Value::string_bytes_clone)
            .collect::<Vec<_>>();

        if keys.len() < items.len() - 2 {
            return Err(ProtocolError::InvalidArgument("key"));
        }

        Ok(SInterStoreParams { key, keys })
    }
}

impl RaftCommand for SInterStoreCommand {
    fn raft_request(&self, items: &[Value]) -> Result<Operation, ProtocolError> {
        let params = Self::parse(items)?;
        Ok(Operation::Redis(RedisSInterStore(params)))
    }
}

#[async_trait]
impl Command for SInterStoreCommand {
    async fn execute(
        &self,
        client: &mut Client,
        items: &[Value],
        server: &RedisServer,
    ) -> Result<Value, CacheCatError> {
        if let Some(vec) = client.transaction_queue.as_mut() {
            vec.push(self.raft_request(items)?);
            return Ok(Value::SimpleString(String::from("QUEUED")));
        }
        let params = Self::parse(items)?;
        server
            .app
            .write(Operation::Redis(RedisSInterStore(params)), client.db_number)
            .await
    }
}
