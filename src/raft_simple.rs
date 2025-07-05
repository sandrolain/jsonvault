use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use std::collections::HashMap;
use tokio::time::{interval, Duration, Instant};
use log::{debug, info, warn, error};

use crate::protocol::{Command, Response};
use crate::Database;

pub type NodeId = u64;
pub type Term = u64;
pub type LogIndex = u64;

/// Stato del nodo Raft
#[derive(Clone, Debug, PartialEq)]
pub enum RaftState {
    Follower,
    Candidate,
    Leader,
}

/// Entry del log Raft
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub term: Term,
    pub index: LogIndex,
    pub command: Command,
    pub id: Uuid,
}

/// Request per AppendEntries RPC
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    pub term: Term,
    pub leader_id: NodeId,
    pub prev_log_index: LogIndex,
    pub prev_log_term: Term,
    pub entries: Vec<LogEntry>,
    pub leader_commit: LogIndex,
}

/// Response per AppendEntries RPC
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    pub term: Term,
    pub success: bool,
    pub match_index: Option<LogIndex>,
}

/// Request per RequestVote RPC
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoteRequest {
    pub term: Term,
    pub candidate_id: NodeId,
    pub last_log_index: LogIndex,
    pub last_log_term: Term,
}

/// Response per RequestVote RPC
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoteResponse {
    pub term: Term,
    pub vote_granted: bool,
}

/// Manager Raft semplificato
pub struct SimpleRaftManager {
    /// ID univoco del nodo
    node_id: NodeId,
    
    /// Database condiviso
    database: Arc<Database>,
    
    /// Stato corrente del nodo
    state: Arc<RwLock<RaftState>>,
    
    /// Term corrente
    current_term: Arc<RwLock<Term>>,
    
    /// Candidato per cui ho votato in questo term
    voted_for: Arc<RwLock<Option<NodeId>>>,
    
    /// Log delle entries
    log: Arc<RwLock<Vec<LogEntry>>>,
    
    /// Index dell'ultima entry applicata allo state machine
    last_applied: Arc<RwLock<LogIndex>>,
    
    /// Index dell'ultima entry committed
    commit_index: Arc<RwLock<LogIndex>>,
    
    /// Nodi del cluster
    cluster_nodes: Arc<RwLock<Vec<NodeId>>>,
    
    /// ID del leader corrente
    current_leader: Arc<RwLock<Option<NodeId>>>,
    
    /// Timeout per election
    election_timeout: Duration,
    
    /// Timestamp dell'ultimo heartbeat ricevuto
    last_heartbeat: Arc<RwLock<Instant>>,
}

impl SimpleRaftManager {
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

    /// Inizializza il cluster
    pub async fn initialize_cluster(&mut self, members: Vec<NodeId>) -> Result<(), String> {
        *self.cluster_nodes.write().await = members.clone();
        
        // Se siamo l'unico nodo, diventiamo leader
        if members.len() == 1 && members[0] == self.node_id {
            *self.state.write().await = RaftState::Leader;
            *self.current_leader.write().await = Some(self.node_id);
            info!("Node {} initialized as single-node leader", self.node_id);
        } else {
            // Avvia il processo di election
            self.start_election_timer().await;
            info!("Node {} initialized in cluster of {} nodes", self.node_id, members.len());
        }
        
        Ok(())
    }

    /// Sottometti un comando
    pub async fn submit_command(&self, command: Command) -> Result<Response, String> {
        if !self.is_leader().await {
            return Err("Not the leader".to_string());
        }

        let entry = LogEntry {
            term: *self.current_term.read().await,
            index: self.log.read().await.len() as LogIndex + 1,
            command: command.clone(),
            id: Uuid::new_v4(),
        };

        // Aggiungi al log
        self.log.write().await.push(entry.clone());
        
        // Per ora, applica immediatamente (semplificazione)
        let response = self.database.execute_command(command).await;
        *self.last_applied.write().await = entry.index;
        *self.commit_index.write().await = entry.index;

        // In una implementazione completa, dovremmo:
        // 1. Replicare sui follower
        // 2. Aspettare la maggioranza
        // 3. Poi applicare

        Ok(response)
    }

    /// Verifica se questo nodo è il leader
    pub async fn is_leader(&self) -> bool {
        matches!(*self.state.read().await, RaftState::Leader)
    }

    /// Ottieni l'ID del leader corrente
    pub async fn leader_id(&self) -> Option<NodeId> {
        *self.current_leader.read().await
    }

    /// Ottieni le metriche del cluster
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

    /// Aggiungi un nodo al cluster
    pub async fn add_node(&mut self, new_node_id: NodeId) -> Result<(), String> {
        let mut nodes = self.cluster_nodes.write().await;
        if !nodes.contains(&new_node_id) {
            nodes.push(new_node_id);
            info!("Added node {} to cluster", new_node_id);
        }
        Ok(())
    }

    /// Avvia il timer per le elezioni
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
                
                // Se siamo leader, non facciamo nulla
                if matches!(*state.read().await, RaftState::Leader) {
                    continue;
                }

                // Controlla se è scaduto il timeout
                let last_hb = *last_heartbeat.read().await;
                if last_hb.elapsed() > election_timeout {
                    info!("Election timeout for node {}, starting election", node_id);
                    
                    // Inizia election
                    let mut term = current_term.write().await;
                    *term += 1;
                    *voted_for.write().await = Some(node_id);
                    *state.write().await = RaftState::Candidate;
                    *current_leader.write().await = None;

                    // Per semplicità, in un cluster single-node diventiamo leader
                    let nodes = cluster_nodes.read().await.clone();
                    if nodes.len() == 1 {
                        *state.write().await = RaftState::Leader;
                        *current_leader.write().await = Some(node_id);
                        info!("Node {} became leader for term {}", node_id, *term);
                    } else {
                        // In un cluster multi-node, dovremmo inviare VoteRequest
                        // Per ora, assumiamo di non vincere
                        warn!("Multi-node election not fully implemented");
                        *state.write().await = RaftState::Follower;
                    }

                    *last_heartbeat.write().await = Instant::now();
                }
            }
        });
    }

    /// Gestisce AppendEntries RPC
    pub async fn handle_append_entries(&self, request: AppendEntriesRequest) -> AppendEntriesResponse {
        let mut current_term = self.current_term.write().await;
        
        // Se il term del request è più vecchio, rifiuta
        if request.term < *current_term {
            return AppendEntriesResponse {
                term: *current_term,
                success: false,
                match_index: None,
            };
        }

        // Se il term è più nuovo, aggiorna
        if request.term > *current_term {
            *current_term = request.term;
            *self.voted_for.write().await = None;
            *self.state.write().await = RaftState::Follower;
        }

        // Aggiorna leader e heartbeat
        *self.current_leader.write().await = Some(request.leader_id);
        *self.last_heartbeat.write().await = Instant::now();

        // Per semplicità, accetta sempre (in produzione dovremmo verificare il log)
        AppendEntriesResponse {
            term: *current_term,
            success: true,
            match_index: Some(request.prev_log_index + request.entries.len() as LogIndex),
        }
    }

    /// Gestisce RequestVote RPC
    pub async fn handle_vote_request(&self, request: VoteRequest) -> VoteResponse {
        let mut current_term = self.current_term.write().await;
        let mut voted_for = self.voted_for.write().await;

        // Se il term del request è più vecchio, rifiuta
        if request.term < *current_term {
            return VoteResponse {
                term: *current_term,
                vote_granted: false,
            };
        }

        // Se il term è più nuovo, aggiorna
        if request.term > *current_term {
            *current_term = request.term;
            *voted_for = None;
            *self.state.write().await = RaftState::Follower;
        }

        // Vota se non abbiamo ancora votato o se abbiamo votato per questo candidato
        let vote_granted = voted_for.is_none() || *voted_for == Some(request.candidate_id);
        
        if vote_granted {
            *voted_for = Some(request.candidate_id);
        }

        VoteResponse {
            term: *current_term,
            vote_granted,
        }
    }

    /// Shutdown del manager
    pub async fn shutdown(self) -> Result<(), String> {
        info!("Shutting down Simple Raft manager for node {}", self.node_id);
        Ok(())
    }
}

/// Metriche del cluster
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
    async fn test_simple_raft_creation() {
        let database = Arc::new(Database::new());
        let manager = SimpleRaftManager::new(1, database).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_single_node_cluster() {
        let database = Arc::new(Database::new());
        let mut manager = SimpleRaftManager::new(1, database).await.unwrap();
        
        manager.initialize_cluster(vec![1]).await.unwrap();
        
        // Aspetta un momento per l'inizializzazione
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        assert!(manager.is_leader().await);
        
        let command = Command::Set {
            key: "test".to_string(),
            value: serde_json::json!({"test": true}),
        };
        
        let result = manager.submit_command(command).await;
        assert!(result.is_ok());
    }
}
