use crate::error::{CacheCatError, ProtocolError};
use crate::mocha::{EntrySnapshot, MochaOperation};
use crate::protocol::command::{Client, Command};
use crate::protocol::raft_command::RaftCommand;
use crate::raft::network::redis_server::RedisServer;
use crate::raft::types::core::mocha::cas::ComputeCommand;
use crate::raft::types::core::mocha::mocha::MyValue;
use crate::raft::types::core::response_value::Value;
use crate::raft::types::core::value_object::ValueObject;
use crate::raft::types::entry::bae_operation::BaseOperation;
use crate::raft::types::entry::request::Operation;
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZPopMinCommand;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZPopMinParams {
    pub key: Bytes,
    pub count: Option<usize>,
}

impl Display for ZPopMinParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ZPopMinParams {{ key: {}, count: {:?} }}",
            String::from_utf8_lossy(&self.key),
            self.count
        )
    }
}

impl ZPopMinCommand {
    fn parse_params(items: &[Value]) -> Result<ZPopMinParams, ProtocolError> {
        let count = match items.len() {
            2 => None,
            3 => Some(items[2].try_parse_usize()?),
            _ => return Err(ProtocolError::WrongArgCount("zpopmin")),
        };

        let key = items[1]
            .string_bytes_clone()
            .ok_or(ProtocolError::InvalidArgument("key"))?;

        Ok(ZPopMinParams { key, count })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZPopMinReq {
    pub key: Bytes,
    pub count: Option<usize>,
}

impl Display for ZPopMinReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ZPopMinReq {{ key: {}, count: {:?} }}",
            String::from_utf8_lossy(&self.key),
            self.count
        )
    }
}

impl RaftCommand for ZPopMinCommand {
    fn raft_request(&self, items: &[Value]) -> Result<Operation, ProtocolError> {
        let ZPopMinParams { key, count } = Self::parse_params(items)?;

        Ok(Operation::Base(BaseOperation::ZPopMin(ZPopMinReq {
            key,
            count,
        })))
    }
}

#[async_trait]
impl Command for ZPopMinCommand {
    async fn execute(
        &self,
        client: &mut Client,
        items: &[Value],
        server: &RedisServer,
    ) -> Result<Value, CacheCatError> {
        // Parse arguments
        let operation = self.raft_request(items)?;
        let value = server.app.write(operation, client.db_number).await?;
        Ok(value)
    }
}

impl ComputeCommand for ZPopMinReq {
    #[inline]
    fn key(&self) -> &Bytes {
        &self.key
    }

    #[inline]
    fn into_base_op(self) -> BaseOperation {
        BaseOperation::ZPopMin(self)
    }

    fn mutate(
        self,
        entry: EntrySnapshot<MyValue>,
        _write_clock: u64,
    ) -> (MochaOperation<MyValue>, Value) {
        match &entry.value.data {
            ValueObject::ZSet(zset) => {
                let values = zset.lock().zpop_min(self.count);
                let values = values
                    .into_iter()
                    .flat_map(|(value, score)| {
                        [
                            Value::BulkString(Some(value)),
                            Value::SimpleString(score.to_string()),
                        ]
                    })
                    .collect();

                (
                    MochaOperation::Insert {
                        value: entry.value.clone(),
                        expire: entry.get_expire_policy(),
                    },
                    Value::Array(Some(values)),
                )
            }

            _ => (
                MochaOperation::Abort,
                Value::Error("zadd: key is not a zset".to_string()),
            ),
        }
    }

    #[inline]
    fn init(self) -> (MochaOperation<MyValue>, Value) {
        (MochaOperation::Abort, Value::Array(None))
    }
}
