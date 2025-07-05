use clap::{Arg, Command as ClapCommand};
use log::{error, info};
use jsonvault::{Database, RaftManager, ReplicationManager, TcpServer};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let matches = ClapCommand::new("rust-json-db-server")
        .version("0.1.0")
        .about("Server del database JSON key-value in-memory")
        .arg(
            Arg::new("address")
                .short('a')
                .long("address")
                .value_name("ADDRESS")
                .help("Indirizzo di bind del server")
                .default_value("127.0.0.1:8080"),
        )
        .arg(
            Arg::new("primary")
                .short('p')
                .long("primary")
                .help("Avvia come nodo primario")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("replica-of")
                .short('r')
                .long("replica-of")
                .value_name("PRIMARY_ADDRESS")
                .help("Indirizzo del nodo primario (per le repliche)"),
        )
        .arg(
            Arg::new("cluster-nodes")
                .short('c')
                .long("cluster-nodes")
                .value_name("NODE_LIST")
                .help("Lista dei nodi del cluster (format: id1:address1,id2:address2)")
                .value_delimiter(','),
        )
        .arg(
            Arg::new("enable-raft")
                .long("enable-raft")
                .help("Abilita il consenso Raft")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("node-id")
                .short('n')
                .long("node-id")
                .value_name("NODE_ID")
                .help("ID univoco del nodo")
                .default_value("auto-generated"),
        )
        .get_matches();

    let address = matches.get_one::<String>("address").unwrap().clone();
    let is_primary = matches.get_flag("primary");
    let replica_of = matches.get_one::<String>("replica-of");
    let node_id_arg = matches.get_one::<String>("node-id").unwrap();
    let enable_raft = matches.get_flag("enable-raft");
    let cluster_nodes: Option<Vec<String>> = matches.get_many::<String>("cluster-nodes")
        .map(|values| values.cloned().collect());
    
    let node_id_str = if node_id_arg == "auto-generated" {
        Uuid::new_v4().to_string()
    } else {
        node_id_arg.clone()
    };
    
    // Converte node_id in u64 per Raft
    let node_id_numeric: u64 = node_id_str.chars()
        .take(8)
        .enumerate()
        .map(|(i, c)| (c as u64) << (i * 8))
        .sum::<u64>()
        .wrapping_add(8080); // Aggiungi offset per evitare 0

    info!("Avvio del server JSON DB");
    info!("Node ID: {} (numeric: {})", node_id_str, node_id_numeric);
    info!("Indirizzo: {}", address);
    info!(
        "Modalità: {}",
        if enable_raft {
            "Raft Consensus"
        } else if is_primary {
            "Primario (Legacy)"
        } else {
            "Replica (Legacy)"
        }
    );

    // Crea il database
    let database = Arc::new(Database::new());

    // Inizializza Raft se abilitato
    let mut raft_manager = if enable_raft {
        let mut manager = RaftManager::new(node_id_numeric, Arc::clone(&database))
            .await
            .map_err(|e| {
                error!("Errore nella creazione del RaftManager: {}", e);
                std::process::exit(1);
            })
            .unwrap();

        // Parsifica i nodi del cluster se forniti
        let cluster_members = if let Some(nodes) = cluster_nodes {
            let mut members = vec![node_id_numeric];
            
            for node_spec in nodes {
                if let Ok(parsed_id) = node_spec.parse::<u64>() {
                    members.push(parsed_id);
                }
            }
            members
        } else {
            vec![node_id_numeric]
        };

        if let Err(e) = manager.initialize_cluster(cluster_members).await {
            error!("Errore nell'inizializzazione del cluster Raft: {}", e);
            std::process::exit(1);
        }

        Some(manager)
    } else {
        None
    };

    // Crea il manager di replicazione legacy se Raft non è abilitato
    let replication_manager = if !enable_raft {
        Some(ReplicationManager::new(Arc::clone(&database), node_id_str.clone(), is_primary))
    } else {
        None
    };

    // Legacy replication logic (solo se Raft non è abilitato)
    if let Some(ref repl_manager) = replication_manager {
        // Se è una replica, connettiti al primario
        if let Some(primary_addr) = replica_of {
            info!("Connessione al nodo primario: {}", primary_addr);
            if let Err(e) = repl_manager.sync_with_primary(primary_addr).await {
                error!("Errore nella sincronizzazione con il primario: {}", e);
                std::process::exit(1);
            }
        }

        // Avvia il processo di replicazione in background se è un primario
        if is_primary {
            let repl_manager_clone = repl_manager.clone();
            tokio::spawn(async move {
                repl_manager_clone.start_replication_process().await;
            });
        }

        // Stampa lo stato della replicazione
        let status = repl_manager.get_replication_status();
        info!("Stato replicazione: {}", status);
    }

    // Stampa metriche Raft se abilitato
    if let Some(ref manager) = raft_manager {
        let metrics = manager.metrics().await;
        info!("Metriche Raft: {:?}", metrics);
    }

    // Crea il server TCP
    let server = TcpServer::new(Arc::clone(&database), address.clone());

    info!("Server pronto per le connessioni");

    // Avvia il server (questo bloccherà il thread principale)
    if let Err(e) = server.start().await {
        error!("Errore del server: {}", e);
        
        // Cleanup Raft se necessario
        if let Some(manager) = raft_manager {
            let _ = manager.shutdown().await;
        }
        
        std::process::exit(1);
    }

    Ok(())
}
