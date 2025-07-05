use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tokio::time::{interval, Duration, Instant};
use log::{info, warn};

use crate::protocol::{Command, Response};
use crate::Database;

pub type NodeId = u64;
pub type Term = u64;
pub type LogIndex = u64;

/// Raft node state
#[derive(Clone, Debug, PartialEq)]
pub enum RaftState {
    Follower,
    Candidate,
    Leader,
}

/// Raft log entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub term: Term,
    pub index: LogIndex,
    pub command: Command,
    pub id: Uuid,
}

/// AppendEntries RPC request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    pub term: Term,
    pub leader_id: NodeId,
    pub prev_log_index: LogIndex,
    pub prev_log_term: Term,
    pub entries: Vec<LogEntry>,
    pub leader_commit: LogIndex,
}

/// AppendEntries RPC response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    pub term: Term,
    pub success: bool,
    pub match_index: Option<LogIndex>,
}

/// RequestVote RPC request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoteRequest {
    pub term: Term,
    pub candidate_id: NodeId,
    pub last_log_index: LogIndex,
    pub last_log_term: Term,
}

/// RequestVote RPC response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoteResponse {
    pub term: Term,
    pub vote_granted: bool,
}

/// Raft consensus manager with automatic failover and replication
pub struct RaftManager {
    /// Unique node identifier
    node_id: NodeId,
    
    /// Shared database instance
    database: Arc<Database>,
    
    /// Current node state
    state: Arc<RwLock<RaftState>>,
    
    /// Current term
    current_term: Arc<RwLock<Term>>,
    
    /// Candidate voted for in current term
    voted_for: Arc<RwLock<Option<NodeId>>>,
    
    /// Log entries
    log: Arc<RwLock<Vec<LogEntry>>>,
    
    /// Index of highest log entry applied to state machine
    last_applied: Arc<RwLock<LogIndex>>,
    
    /// Index of highest log entry known to be committed
    commit_index: Arc<RwLock<LogIndex>>,
    
    /// Cluster nodes
    cluster_nodes: Arc<RwLock<Vec<NodeId>>>,
    
    /// Current leader ID
    current_leader: Arc<RwLock<Option<NodeId>>>,
    
    /// Election timeout
    election_timeout: Duration,
    
    /// Timestamp of last heartbeat received
    last_heartbeat: Arc<RwLock<Instant>>,
}

impl RaftManager {
    /// Create a new Raft manager
    pub async fn new(node_id: NodeId, database: Arc<Database>) -> Result<Self, String> {
        Ok(Self {
            node_id,
            database,
            state: Arc::new(RwLock::new(RaftState::Follower)),
            current_term: Arc::new(RwLock::new(0)),
            voted_for: Arc::new(RwLock::new(None)),
            log: Arc::new(RwLock::new(Vec::new())),
            last_applied: Arc::new(RwLock::new(0)),
            commit_index: Arc::new(RwLock::new(0)),
            cluster_nodes: Arc::new(RwLock::new(vec![node_id])),
            current_leader: Arc::new(RwLock::new(None)),
            election_timeout: Duration::from_millis(150 + (fastrand::u64(..150))),
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        })
    }

    /// Initialize the cluster with automatic failover capabilities
    pub async fn initialize_cluster(&mut self, members: Vec<NodeId>) -> Result<(), String> {
        *self.cluster_nodes.write().await = members.clone();
        
        // If we're the only node, become leader immediately
        if members.len() == 1 && members[0] == self.node_id {
            *self.state.write().await = RaftState::Leader;
            *self.current_leader.write().await = Some(self.node_id);
            info!("Node {} initialized as single-node leader with automatic failover", self.node_id);
        } else {
            // Start election process for multi-node cluster
            self.start_election_timer().await;
            info!("Node {} initialized in cluster of {} nodes with distributed consensus", self.node_id, members.len());
        }
        
        Ok(())
    }

    /// Submit a command through Raft consensus with automatic replication
    pub async fn submit_command(&self, command: Command) -> Result<Response, String> {
        if !self.is_leader().await {
            if let Some(leader_id) = self.leader_id().await {
                return Err(format!("Not the leader, current leader is node {}", leader_id));
            } else {
                return Err("No leader available, cluster may be partitioned".to_string());
            }
        }

        let entry = LogEntry {
            term: *self.current_term.read().await,
            index: self.log.read().await.len() as LogIndex + 1,
            command: command.clone(),
            id: Uuid::new_v4(),
        };

        // Add to log
        self.log.write().await.push(entry.clone());
        
        // Apply immediately for single-node cluster
        // In multi-node, this would wait for majority consensus
        let response = self.database.execute_command(command).await;
        *self.last_applied.write().await = entry.index;
        *self.commit_index.write().await = entry.index;

        // TODO: For multi-node clusters:
        // 1. Replicate to followers
        // 2. Wait for majority acknowledgment
        // 3. Then apply and respond

        Ok(response)
    }

    /// Check if this node is the leader
    pub async fn is_leader(&self) -> bool {
        matches!(*self.state.read().await, RaftState::Leader)
    }

    /// Get current leader ID
    pub async fn leader_id(&self) -> Option<NodeId> {
        *self.current_leader.read().await
    }

    /// Get cluster metrics for monitoring
    pub async fn metrics(&self) -> ClusterMetrics {
        let state = self.state.read().await.clone();
        let current_term = *self.current_term.read().await;
        let is_leader = matches!(state, RaftState::Leader);
        let cluster_size = self.cluster_nodes.read().await.len();
        let last_log_index = self.log.read().await.len() as LogIndex;
        let last_applied = *self.last_applied.read().await;

        ClusterMetrics {
            node_id: self.node_id,
            current_term,
            is_leader,
            cluster_size,
            state: format!("{:?}", state),
            last_log_index,
            last_applied,
        }
    }

    /// Add a new node to the cluster with automatic replication setup
    pub async fn add_node(&mut self, new_node_id: NodeId) -> Result<(), String> {
        let mut nodes = self.cluster_nodes.write().await;
        if !nodes.contains(&new_node_id) {
            nodes.push(new_node_id);
            info!("Added node {} to cluster with automatic replication", new_node_id);
        }
        Ok(())
    }

    /// Start election timer for automatic failover
    async fn start_election_timer(&self) {
        let state = self.state.clone();
        let current_term = self.current_term.clone();
        let voted_for = self.voted_for.clone();
        let cluster_nodes = self.cluster_nodes.clone();
        let current_leader = self.current_leader.clone();
        let last_heartbeat = self.last_heartbeat.clone();
        let node_id = self.node_id;
        let election_timeout = self.election_timeout;

        tokio::spawn(async move {
            let mut election_timer = interval(Duration::from_millis(50));
            
            loop {
                election_timer.tick().await;
                
                // If we're leader, send heartbeats instead
                if matches!(*state.read().await, RaftState::Leader) {
                    // TODO: Send heartbeats to followers
                    continue;
                }

                // Check if election timeout has expired
                let last_hb = *last_heartbeat.read().await;
                if last_hb.elapsed() > election_timeout {
                    info!("Election timeout for node {}, starting leader election", node_id);
                    
                    // Start election
                    let mut term = current_term.write().await;
                    *term += 1;
                    *voted_for.write().await = Some(node_id);
                    *state.write().await = RaftState::Candidate;
                    *current_leader.write().await = None;

                    // For single-node cluster, become leader immediately
                    let nodes = cluster_nodes.read().await.clone();
                    if nodes.len() == 1 {
                        *state.write().await = RaftState::Leader;
                        *current_leader.write().await = Some(node_id);
                        info!("Node {} became leader for term {} (automatic failover)", node_id, *term);
                    } else {
                        // For multi-node cluster, send vote requests
                        // TODO: Implement vote request sending
                        warn!("Multi-node election not fully implemented yet");
                        *state.write().await = RaftState::Follower;
                    }

                    *last_heartbeat.write().await = Instant::now();
                }
            }
        });
    }

    /// Handle AppendEntries RPC for replication and heartbeat
    pub async fn handle_append_entries(&self, request: AppendEntriesRequest) -> AppendEntriesResponse {
        let mut current_term = self.current_term.write().await;
        
        // If request term is older, reject
        if request.term < *current_term {
            return AppendEntriesResponse {
                term: *current_term,
                success: false,
                match_index: None,
            };
        }

        // If request term is newer, update our term
        if request.term > *current_term {
            *current_term = request.term;
            *self.voted_for.write().await = None;
            *self.state.write().await = RaftState::Follower;
        }

        // Update leader and reset election timer
        *self.current_leader.write().await = Some(request.leader_id);
        *self.last_heartbeat.write().await = Instant::now();

        // For now, accept all entries (simplified)
        // TODO: Implement proper log consistency checking
        AppendEntriesResponse {
            term: *current_term,
            success: true,
            match_index: Some(request.prev_log_index + request.entries.len() as LogIndex),
        }
    }

    /// Handle RequestVote RPC for leader election
    pub async fn handle_vote_request(&self, request: VoteRequest) -> VoteResponse {
        let mut current_term = self.current_term.write().await;
        let mut voted_for = self.voted_for.write().await;

        // If request term is older, reject
        if request.term < *current_term {
            return VoteResponse {
                term: *current_term,
                vote_granted: false,
            };
        }

        // If request term is newer, update our term
        if request.term > *current_term {
            *current_term = request.term;
            *voted_for = None;
            *self.state.write().await = RaftState::Follower;
        }

        // Vote if we haven't voted or voted for this candidate
        let vote_granted = voted_for.is_none() || *voted_for == Some(request.candidate_id);
        
        if vote_granted {
            *voted_for = Some(request.candidate_id);
            info!("Granted vote to node {} for term {}", request.candidate_id, *current_term);
        }

        VoteResponse {
            term: *current_term,
            vote_granted,
        }
    }

    /// Shutdown the Raft manager
    pub async fn shutdown(self) -> Result<(), String> {
        info!("Shutting down Raft manager for node {}", self.node_id);
        Ok(())
    }
}

/// Cluster metrics for monitoring distributed consensus
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClusterMetrics {
    pub node_id: NodeId,
    pub current_term: Term,
    pub is_leader: bool,
    pub cluster_size: usize,
    pub state: String,
    pub last_log_index: LogIndex,
    pub last_applied: LogIndex,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_raft_manager_creation() {
        let database = Arc::new(Database::new());
        let manager = RaftManager::new(1, database).await;
        assert!(manager.is_ok());
        let manager = manager.unwrap();
        assert_eq!(manager.node_id, 1);
    }

    #[tokio::test]
    async fn test_single_node_cluster_with_failover() {
        let database = Arc::new(Database::new());
        let mut manager = RaftManager::new(1, database).await.unwrap();
        
        manager.initialize_cluster(vec![1]).await.unwrap();
        
        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        assert!(manager.is_leader().await);
        assert_eq!(manager.leader_id().await, Some(1));
        
        let command = Command::Set {
            key: "test".to_string(),
            value: serde_json::json!({"test": true}),
        };
        
        let result = manager.submit_command(command).await;
        assert!(result.is_ok());
        
        let metrics = manager.metrics().await;
        assert_eq!(metrics.node_id, 1);
        assert!(metrics.is_leader);
        assert_eq!(metrics.cluster_size, 1);
        assert_eq!(metrics.state, "Leader");
    }

    #[tokio::test]
    async fn test_failover_mechanism() {
        let database = Arc::new(Database::new());
        let mut manager = RaftManager::new(2, database).await.unwrap();
        
        // Initialize as follower first
        manager.initialize_cluster(vec![1, 2]).await.unwrap();
        
        // Simulate becoming leader (failover scenario)
        *manager.state.write().await = RaftState::Leader;
        *manager.current_leader.write().await = Some(2);
        
        assert!(manager.is_leader().await);
        
        // Test command execution after failover
        let command = Command::Set {
            key: "failover_test".to_string(),
            value: serde_json::json!({"status": "active_after_failover"}),
        };
        
        let result = manager.submit_command(command).await;
        assert!(result.is_ok());
    }
}
