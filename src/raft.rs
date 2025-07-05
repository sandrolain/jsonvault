use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use openraft::Config;
use openraft::{BasicNode, Raft, RaftNetwork, RaftNetworkFactory, RaftStorage};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::io::Cursor;

use crate::protocol::{Command, Response};
use crate::Database;

pub type NodeId = u64;
pub type LogIndex = u64;
pub type Term = u64;

/// Configurazione del tipo Raft per JsonVault
#[derive(Clone)]
pub struct JsonVaultTypeConfig;

impl openraft::RaftTypeConfig for JsonVaultTypeConfig {
    type D = JsonVaultRequest;
    type R = JsonVaultResponse;
    type NodeId = NodeId;
    type Node = BasicNode;
    type Entry = openraft::Entry<JsonVaultRequest>;
    type SnapshotData = Cursor<Vec<u8>>;
    type AsyncRuntime = openraft::TokioRuntime;
}

/// Dati applicativi per JsonVault
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonVaultRequest {
    pub id: Uuid,
    pub command: Command,
}

/// Risposta per le operazioni JsonVault
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonVaultResponse {
    pub id: Uuid,
    pub response: Response,
}

/// Stato dell'applicazione JsonVault
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct JsonVaultStateMachine {
    pub data: BTreeMap<String, serde_json::Value>,
    pub last_applied_log: Option<LogIndex>,
}

/// Network layer per OpenRaft
#[derive(Clone)]
pub struct JsonVaultNetwork {
    clients: Arc<RwLock<BTreeMap<NodeId, String>>>,
}

impl JsonVaultNetwork {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
    
    pub async fn add_node(&self, node_id: NodeId, address: String) {
        self.clients.write().await.insert(node_id, address);
    }
}

#[async_trait]
impl RaftNetworkFactory<JsonVaultRequest> for JsonVaultNetwork {
    type Network = JsonVaultNetworkConnection;

    async fn new_client(&mut self, target: NodeId, _node: &BasicNode) -> Self::Network {
        let clients = self.clients.read().await;
        let address = clients.get(&target).cloned().unwrap_or_default();
        JsonVaultNetworkConnection { target, address }
    }
}

/// Connessione di rete per un nodo specifico
pub struct JsonVaultNetworkConnection {
    target: NodeId,
    address: String,
}

#[async_trait]
impl RaftNetwork<JsonVaultRequest> for JsonVaultNetworkConnection {
    async fn send_append_entries(
        &mut self,
        rpc: openraft::raft::AppendEntriesRequest<JsonVaultRequest>,
    ) -> Result<
        openraft::raft::AppendEntriesResponse<NodeId>,
        openraft::error::RPCError<NodeId, openraft::BasicNode, openraft::error::Unreachable>,
    > {
        // Implementazione semplificata - in produzione useremmo HTTP/gRPC
        log::debug!("Sending append_entries to node {}", self.target);
        
        // Per ora restituiamo sempre successo per la demo
        Ok(openraft::raft::AppendEntriesResponse {
            term: rpc.term,
            success: true,
            conflict_opt: None,
        })
    }

    async fn send_install_snapshot(
        &mut self,
        rpc: openraft::raft::InstallSnapshotRequest<NodeId>,
    ) -> Result<
        openraft::raft::InstallSnapshotResponse<NodeId>,
        openraft::error::RPCError<NodeId, openraft::BasicNode, openraft::error::Unreachable>,
    > {
        log::debug!("Sending install_snapshot to node {}", self.target);
        
        Ok(openraft::raft::InstallSnapshotResponse {
            term: rpc.term,
        })
    }

    async fn send_vote(
        &mut self,
        rpc: openraft::raft::VoteRequest<NodeId>,
    ) -> Result<
        openraft::raft::VoteResponse<NodeId>,
        openraft::error::RPCError<NodeId, openraft::BasicNode, openraft::error::Unreachable>,
    > {
        log::debug!("Sending vote request to node {}", self.target);
        
        Ok(openraft::raft::VoteResponse {
            term: rpc.term,
            vote_granted: true,
            last_log_id: None,
        })
    }
}

/// Storage implementazione per OpenRaft
pub struct JsonVaultStorage {
    database: Arc<Database>,
    logs: Arc<RwLock<BTreeMap<LogIndex, openraft::Entry<JsonVaultRequest>>>>,
    state_machine: Arc<RwLock<JsonVaultStateMachine>>,
    hard_state: Arc<RwLock<Option<openraft::HardState<NodeId>>>>,
    snapshot: Arc<RwLock<Option<openraft::Snapshot<NodeId, BasicNode, Cursor<Vec<u8>>>>>>,
}

impl JsonVaultStorage {
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            database,
            logs: Arc::new(RwLock::new(BTreeMap::new())),
            state_machine: Arc::new(RwLock::new(JsonVaultStateMachine::default())),
            hard_state: Arc::new(RwLock::new(None)),
            snapshot: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait]
impl RaftStorage<JsonVaultRequest, JsonVaultResponse> for JsonVaultStorage {
    type SnapshotData = Cursor<Vec<u8>>;

    async fn save_hard_state(
        &mut self,
        hs: &openraft::HardState<NodeId>,
    ) -> Result<(), openraft::StorageError<NodeId>> {
        *self.hard_state.write().await = Some(hs.clone());
        Ok(())
    }

    async fn read_hard_state(
        &mut self,
    ) -> Result<Option<openraft::HardState<NodeId>>, openraft::StorageError<NodeId>> {
        Ok(self.hard_state.read().await.clone())
    }

    async fn save_vote(
        &mut self,
        vote: &openraft::Vote<NodeId>,
    ) -> Result<(), openraft::StorageError<NodeId>> {
        // Salva il voto - per ora in memoria
        log::debug!("Saving vote: {:?}", vote);
        Ok(())
    }

    async fn read_vote(
        &mut self,
    ) -> Result<Option<openraft::Vote<NodeId>>, openraft::StorageError<NodeId>> {
        // Leggi il voto salvato
        Ok(None)
    }

    async fn get_log_entries<RNG: openraft::RaftTypeConfig<D = JsonVaultRequest>>(
        &mut self,
        range: std::ops::Range<LogIndex>,
    ) -> Result<Vec<openraft::Entry<JsonVaultRequest>>, openraft::StorageError<NodeId>> {
        let logs = self.logs.read().await;
        let entries: Vec<_> = logs
            .range(range)
            .map(|(_, entry)| entry.clone())
            .collect();
        Ok(entries)
    }

    async fn delete_conflict_logs_since(
        &mut self,
        log_id: openraft::LogId<NodeId>,
    ) -> Result<(), openraft::StorageError<NodeId>> {
        let mut logs = self.logs.write().await;
        logs.retain(|&index, _| index < log_id.index);
        Ok(())
    }

    async fn purge_logs_upto(
        &mut self,
        log_id: openraft::LogId<NodeId>,
    ) -> Result<(), openraft::StorageError<NodeId>> {
        let mut logs = self.logs.write().await;
        logs.retain(|&index, _| index > log_id.index);
        Ok(())
    }

    async fn append_to_log(
        &mut self,
        entries: &[openraft::Entry<JsonVaultRequest>],
    ) -> Result<(), openraft::StorageError<NodeId>> {
        let mut logs = self.logs.write().await;
        for entry in entries {
            logs.insert(entry.log_id.index, entry.clone());
        }
        Ok(())
    }

    async fn apply_to_state_machine(
        &mut self,
        entries: &[openraft::Entry<JsonVaultRequest>],
    ) -> Result<Vec<JsonVaultResponse>, openraft::StorageError<NodeId>> {
        let mut responses = Vec::new();
        let mut sm = self.state_machine.write().await;

        for entry in entries {
            let response = match &entry.payload {
                openraft::EntryPayload::Blank => JsonVaultResponse {
                    id: Uuid::new_v4(),
                    response: Response::Ok(None),
                },
                openraft::EntryPayload::Normal(req) => {
                    // Applica il comando al database
                    let resp = self.database.execute_command(req.command.clone()).await;
                    
                    // Aggiorna anche lo state machine per snapshot
                    match &req.command {
                        Command::Set { key, value } => {
                            sm.data.insert(key.clone(), value.clone());
                        }
                        Command::Delete { key } => {
                            sm.data.remove(key);
                        }
                        _ => {}
                    }
                    
                    JsonVaultResponse {
                        id: req.id,
                        response: resp,
                    }
                }
                openraft::EntryPayload::Membership(membership) => {
                    log::info!("Applying membership change: {:?}", membership);
                    JsonVaultResponse {
                        id: Uuid::new_v4(),
                        response: Response::Ok(None),
                    }
                }
            };
            
            sm.last_applied_log = Some(entry.log_id.index);
            responses.push(response);
        }

        Ok(responses)
    }

    async fn build_snapshot(
        &mut self,
    ) -> Result<openraft::Snapshot<NodeId, BasicNode, Self::SnapshotData>, openraft::StorageError<NodeId>> {
        let sm = self.state_machine.read().await;
        let data = serde_json::to_vec(&*sm).map_err(|e| {
            openraft::StorageError::IO {
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            }
        })?;

        let snapshot_id = format!("{}-{}", sm.last_applied_log.unwrap_or(0), chrono::Utc::now().timestamp());
        
        Ok(openraft::Snapshot {
            meta: openraft::SnapshotMeta {
                last_log_id: Some(openraft::LogId::new(0, sm.last_applied_log.unwrap_or(0))),
                last_membership: openraft::Membership::new(vec![1], None),
                snapshot_id,
            },
            snapshot: Box::new(Cursor::new(data)),
        })
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<Self::SnapshotData>, openraft::StorageError<NodeId>> {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    async fn install_snapshot(
        &mut self,
        meta: &openraft::SnapshotMeta<NodeId, BasicNode>,
        snapshot: Box<Self::SnapshotData>,
    ) -> Result<(), openraft::StorageError<NodeId>> {
        let data = snapshot.into_inner();
        let sm: JsonVaultStateMachine = serde_json::from_slice(&data).map_err(|e| {
            openraft::StorageError::IO {
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            }
        })?;

        // Applica lo snapshot al database
        for (key, value) in &sm.data {
            self.database.execute_command(Command::Set {
                key: key.clone(),
                value: value.clone(),
            }).await;
        }

        *self.state_machine.write().await = sm;
        *self.snapshot.write().await = Some(openraft::Snapshot {
            meta: meta.clone(),
            snapshot: Box::new(Cursor::new(data)),
        });

        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<openraft::Snapshot<NodeId, BasicNode, Self::SnapshotData>>, openraft::StorageError<NodeId>> {
        Ok(self.snapshot.read().await.clone())
    }
}

/// Manager Raft aggiornato che usa OpenRaft
pub struct RaftManager {
    node_id: NodeId,
    raft: Option<Raft<JsonVaultRequest, JsonVaultResponse, JsonVaultNetwork, JsonVaultStorage>>,
    network: JsonVaultNetwork,
    storage: JsonVaultStorage,
}

impl RaftManager {
    pub async fn new(node_id: NodeId, database: Arc<Database>) -> Result<Self, String> {
        let network = JsonVaultNetwork::new();
        let storage = JsonVaultStorage::new(database);
        
        Ok(Self {
            node_id,
            raft: None,
            network,
            storage,
        })
    }

    /// Inizializza il cluster Raft
    pub async fn initialize_cluster(&mut self, members: Vec<(NodeId, String)>) -> Result<(), String> {
        // Aggiungi i nodi alla rete
        for (id, address) in &members {
            self.network.add_node(*id, address.clone()).await;
        }

        // Configura Raft
        let config = Config {
            heartbeat_interval: 250,
            election_timeout_min: 299,
            election_timeout_max: 499,
            ..Default::default()
        };

        // Crea il cluster con i membri iniziali
        let mut node_set = std::collections::BTreeSet::new();
        for (id, _) in members {
            node_set.insert(id);
        }

        // Inizializza Raft
        let raft = Raft::new(
            self.node_id,
            config,
            self.network.clone(),
            self.storage.clone(),
        ).await.map_err(|e| format!("Failed to create Raft instance: {}", e))?;

        // Se siamo il primo nodo, inizializza il cluster
        if node_set.contains(&self.node_id) && node_set.len() == 1 {
            raft.initialize(node_set).await
                .map_err(|e| format!("Failed to initialize Raft cluster: {}", e))?;
        }

        self.raft = Some(raft);
        log::info!("Raft cluster initialized with {} nodes", node_set.len());
        Ok(())
    }

    /// Sottometti un comando attraverso il consenso Raft
    pub async fn submit_command(&self, command: Command) -> Result<Response, String> {
        let raft = self.raft.as_ref().ok_or("Raft not initialized")?;
        
        let request = JsonVaultRequest {
            id: Uuid::new_v4(),
            command,
        };

        match raft.client_write(request).await {
            Ok(response) => {
                log::debug!("Command submitted successfully");
                Ok(response.data.response)
            }
            Err(e) => {
                log::error!("Failed to submit command: {}", e);
                Err(format!("Raft error: {}", e))
            }
        }
    }

    /// Verifica se questo nodo è il leader
    pub async fn is_leader(&self) -> bool {
        if let Some(raft) = &self.raft {
            let metrics = raft.metrics().borrow().clone();
            matches!(metrics.state, openraft::State::Leader)
        } else {
            false
        }
    }

    /// Ottieni le metriche del cluster
    pub async fn metrics(&self) -> ClusterMetrics {
        if let Some(raft) = &self.raft {
            let metrics = raft.metrics().borrow().clone();
            ClusterMetrics {
                node_id: self.node_id,
                current_term: metrics.current_term.unwrap_or(0),
                is_leader: matches!(metrics.state, openraft::State::Leader),
                cluster_size: metrics.membership_config.membership().unwrap_or(&BTreeMap::new()).len(),
                state: format!("{:?}", metrics.state),
                last_log_index: metrics.last_log_index.unwrap_or(0),
                last_applied: metrics.last_applied.unwrap_or(0),
            }
        } else {
            ClusterMetrics {
                node_id: self.node_id,
                current_term: 0,
                is_leader: false,
                cluster_size: 0,
                state: "Uninitialized".to_string(),
                last_log_index: 0,
                last_applied: 0,
            }
        }
    }

    /// Aggiungi un nuovo nodo al cluster
    pub async fn add_node(&mut self, new_node_id: NodeId, address: String) -> Result<(), String> {
        self.network.add_node(new_node_id, address).await;
        
        if let Some(raft) = &self.raft {
            let mut new_membership = BTreeMap::new();
            new_membership.insert(new_node_id, BasicNode::default());
            
            // In una implementazione completa, dovremmo gestire il cambio di membership
            log::info!("Node {} added to cluster", new_node_id);
        }
        
        Ok(())
    }

    /// Ottieni l'ID del leader corrente
    pub async fn leader_id(&self) -> Option<NodeId> {
        if let Some(raft) = &self.raft {
            let metrics = raft.metrics().borrow().clone();
            metrics.current_leader
        } else {
            None
        }
    }

    /// Shutdown del manager Raft
    pub async fn shutdown(self) -> Result<(), String> {
        if let Some(raft) = self.raft {
            raft.shutdown().await
                .map_err(|e| format!("Failed to shutdown Raft: {}", e))?;
        }
        log::info!("Raft manager shut down for node {}", self.node_id);
        Ok(())
    }
}
/// Metriche del cluster per il monitoraggio
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClusterMetrics {
    pub node_id: NodeId,
    pub current_term: u64,
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
    async fn test_cluster_initialization() {
        let database = Arc::new(Database::new());
        let mut manager = RaftManager::new(1, database).await.unwrap();

        let result = manager.initialize_cluster(vec![(1, "127.0.0.1:8080".to_string())]).await;
        assert!(result.is_ok());

        let metrics = manager.metrics().await;
        assert_eq!(metrics.node_id, 1);
    }
}
