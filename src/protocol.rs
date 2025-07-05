use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Commands supported by the protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// SET key value - Set a value for a key
    Set { key: String, value: Value },
    /// GET key - Read a value for a key
    Get { key: String },
    /// DELETE key - Delete a value for a key
    Delete { key: String },
    /// QGET key query - Execute a JSONPath query on a value
    QGet { key: String, query: String },
    /// QSET key path value - Set a sub-property using JSONPath
    QSet {
        key: String,
        path: String,
        value: Value,
    },
    /// MERGE key value - Merge a JSON value with an existing one
    Merge { key: String, value: Value },
    /// PING - Health check
    Ping,
    /// REPLICATE - Commands for replication
    Replicate { data: ReplicationData },
}

/// Server response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// Operation completed successfully
    Ok(Option<Value>),
    /// Operation error
    Error(String),
    /// Response to PING
    Pong,
    /// Response to replication
    ReplicationAck,
}

/// Replication data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicationData {
    /// Full database synchronization
    FullSync(Vec<(String, Value)>),
    /// Single operation to replicate
    Operation {
        op_type: OperationType,
        key: String,
        value: Option<Value>,
    },
}

/// Operation type for replication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Set,
    Delete,
    Merge,
    QSet,
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Set { key, .. } => write!(f, "SET {}", key),
            Command::Get { key } => write!(f, "GET {}", key),
            Command::Delete { key } => write!(f, "DELETE {}", key),
            Command::QGet { key, query } => write!(f, "QGET {} {}", key, query),
            Command::QSet { key, path, .. } => write!(f, "QSET {} {}", key, path),
            Command::Merge { key, .. } => write!(f, "MERGE {}", key),
            Command::Ping => write!(f, "PING"),
            Command::Replicate { .. } => write!(f, "REPLICATE"),
        }
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Response::Ok(Some(value)) => write!(f, "OK {}", value),
            Response::Ok(None) => write!(f, "OK"),
            Response::Error(msg) => write!(f, "ERROR {}", msg),
            Response::Pong => write!(f, "PONG"),
            Response::ReplicationAck => write!(f, "REPLICATION_ACK"),
        }
    }
}
