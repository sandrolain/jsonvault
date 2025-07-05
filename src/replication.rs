use crate::database::Database;
use crate::network::TcpClient;
use crate::protocol::{Command, ReplicationData};
use log::{error, info, warn};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

/// Manager per la replicazione tra nodi
#[derive(Clone)]
pub struct ReplicationManager {
    database: Arc<Database>,
    node_id: String,
    is_primary: bool,
    replicas: Vec<String>,
}

impl ReplicationManager {
    /// Crea un nuovo manager di replicazione
    pub fn new(database: Arc<Database>, node_id: String, is_primary: bool) -> Self {
        Self {
            database,
            node_id,
            is_primary,
            replicas: Vec::new(),
        }
    }

    /// Aggiunge una replica
    pub async fn add_replica(&mut self, replica_address: String) {
        self.replicas.push(replica_address.clone());
        self.database.add_replica(replica_address.clone()).await;

        if self.is_primary {
            // Invia sincronizzazione completa alla nuova replica
            self.send_full_sync_to_replica(&replica_address).await;
        }
    }

    /// Rimuove una replica
    pub async fn remove_replica(&mut self, replica_address: &str) {
        self.replicas.retain(|addr| addr != replica_address);
        self.database.remove_replica(replica_address).await;
    }

    /// Invia una sincronizzazione completa a una replica specifica
    async fn send_full_sync_to_replica(&self, replica_address: &str) {
        let all_data = self.database.get_all_data().await;
        let sync_data = ReplicationData::FullSync(all_data);
        let command = Command::Replicate { data: sync_data };

        match TcpClient::connect(replica_address).await {
            Ok(mut client) => {
                match client.send_command(command).await {
                    Ok(_) => {
                        info!("Sincronizzazione completa inviata a {}", replica_address);
                    }
                    Err(e) => {
                        error!(
                            "Errore nell'invio della sincronizzazione a {}: {}",
                            replica_address, e
                        );
                    }
                }
                let _ = client.close().await;
            }
            Err(e) => {
                error!(
                    "Impossibile connettersi alla replica {}: {}",
                    replica_address, e
                );
            }
        }
    }

    /// Avvia il processo di replicazione (solo per il nodo primario)
    pub async fn start_replication_process(&self) {
        if !self.is_primary {
            warn!("Tentativo di avviare la replicazione su un nodo non primario");
            return;
        }

        info!(
            "Avviato processo di replicazione per il nodo primario {}",
            self.node_id
        );

        // Per ora, la replicazione è gestita direttamente dalle operazioni del database
        // In una implementazione più avanzata, qui potremmo implementare:
        // - Health check delle repliche
        // - Retry automatici
        // - Gestione della consistenza

        let mut health_check_interval = interval(Duration::from_secs(30));

        loop {
            health_check_interval.tick().await;
            self.check_replicas_health().await;
        }
    }

    /// Controlla lo stato di salute delle repliche
    async fn check_replicas_health(&self) {
        for replica in &self.replicas {
            match TcpClient::connect(replica).await {
                Ok(mut client) => {
                    match client.send_command(Command::Ping).await {
                        Ok(_) => {
                            info!("Replica {} è online", replica);
                        }
                        Err(e) => {
                            warn!("Replica {} non risponde al ping: {}", replica, e);
                        }
                    }
                    let _ = client.close().await;
                }
                Err(e) => {
                    warn!("Impossibile connettersi alla replica {}: {}", replica, e);
                }
            }
        }
    }

    /// Gestisce il failover (promozione di una replica a primario)
    pub async fn handle_failover(&mut self) -> Result<(), String> {
        if self.is_primary {
            warn!("Tentativo di failover su un nodo già primario");
            return Ok(());
        }

        info!("Promozione del nodo {} a primario", self.node_id);
        self.is_primary = true;

        // In una implementazione più completa, qui dovremmo:
        // 1. Notificare le altre repliche del cambio di leadership
        // 2. Implementare un algoritmo di consenso (come Raft)
        // 3. Gestire la sincronizzazione dei dati

        Ok(())
    }

    /// Sincronizza con il nodo primario (per le repliche)
    pub async fn sync_with_primary(&self, primary_address: &str) -> Result<(), String> {
        if self.is_primary {
            warn!("Tentativo di sincronizzazione su un nodo primario");
            return Ok(());
        }

        info!("Sincronizzazione con il primario {}", primary_address);

        match TcpClient::connect(primary_address).await {
            Ok(mut client) => {
                // Richiedi sincronizzazione completa
                // In una implementazione reale, implementeremmo un comando specifico per questo
                match client.send_command(Command::Ping).await {
                    Ok(_) => {
                        info!("Connessione al primario stabilita");
                    }
                    Err(e) => {
                        error!("Errore nella comunicazione con il primario: {}", e);
                    }
                }
                let _ = client.close().await;
            }
            Err(e) => {
                error!(
                    "Impossibile connettersi al primario {}: {}",
                    primary_address, e
                );
                return Err(format!("Connessione al primario fallita: {}", e));
            }
        }

        Ok(())
    }

    /// Ottiene lo stato della replicazione
    pub fn get_replication_status(&self) -> ReplicationStatus {
        ReplicationStatus {
            node_id: self.node_id.clone(),
            is_primary: self.is_primary,
            replica_count: self.replicas.len(),
            replicas: self.replicas.clone(),
        }
    }
}

/// Stato della replicazione
#[derive(Debug, Clone)]
pub struct ReplicationStatus {
    pub node_id: String,
    pub is_primary: bool,
    pub replica_count: usize,
    pub replicas: Vec<String>,
}

impl std::fmt::Display for ReplicationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Node: {} | Primary: {} | Replicas: {} | Addresses: {:?}",
            self.node_id, self.is_primary, self.replica_count, self.replicas
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_replication_manager_creation() {
        let database = Arc::new(Database::new());
        let manager = ReplicationManager::new(database, "node-1".to_string(), true);

        let status = manager.get_replication_status();
        assert_eq!(status.node_id, "node-1");
        assert!(status.is_primary);
        assert_eq!(status.replica_count, 0);
    }

    #[tokio::test]
    async fn test_add_replica() {
        let database = Arc::new(Database::new());
        let mut manager = ReplicationManager::new(database, "node-1".to_string(), true);

        manager.add_replica("127.0.0.1:8082".to_string()).await;

        let status = manager.get_replication_status();
        assert_eq!(status.replica_count, 1);
        assert!(status.replicas.contains(&"127.0.0.1:8082".to_string()));
    }
}
