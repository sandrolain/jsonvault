use clap::{Arg, Command as ClapCommand};
use log::{error, info};
use jsonvault::{Database, RaftManager, TcpServer};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let matches = ClapCommand::new("jsonvault-server")
        .version("0.1.0")
        .about("JsonVault - High-performance JSON database with Raft consensus")
        .arg(
            Arg::new("address")
                .short('a')
                .long("address")
                .value_name("ADDRESS")
                .help("Server bind address")
                .default_value("127.0.0.1:8080"),
        )
        .arg(
            Arg::new("cluster-nodes")
                .short('c')
                .long("cluster-nodes")
                .value_name("NODE_LIST")
                .help("Cluster node IDs (comma-separated: 1,2,3)")
                .value_delimiter(','),
        )
        .arg(
            Arg::new("node-id")
                .short('n')
                .long("node-id")
                .value_name("NODE_ID")
                .help("Unique node identifier")
                .default_value("auto-generated"),
        )
        .get_matches();

    let address = matches.get_one::<String>("address").unwrap().clone();
    let node_id_arg = matches.get_one::<String>("node-id").unwrap();
    let cluster_nodes: Option<Vec<String>> = matches.get_many::<String>("cluster-nodes")
        .map(|values| values.cloned().collect());
    
    let node_id_str = if node_id_arg == "auto-generated" {
        Uuid::new_v4().to_string()
    } else {
        node_id_arg.clone()
    };
    
    // Convert node_id to u64 for Raft
    let node_id_numeric: u64 = node_id_str.chars()
        .take(8)
        .enumerate()
        .map(|(i, c)| (c as u64) << (i * 8))
        .sum::<u64>()
        .wrapping_add(8080); // Add offset to avoid 0

    info!("Starting JsonVault server with Raft consensus");
    info!("Node ID: {} (numeric: {})", node_id_str, node_id_numeric);
    info!("Address: {}", address);

    // Create database
    let database = Arc::new(Database::new());

    // Initialize Raft manager
    let mut raft_manager = RaftManager::new(node_id_numeric, Arc::clone(&database))
        .await
        .map_err(|e| {
            error!("Failed to create RaftManager: {}", e);
            std::process::exit(1);
        })
        .unwrap();

    // Parse cluster members
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

    // Initialize cluster with automatic failover
    if let Err(e) = raft_manager.initialize_cluster(cluster_members.clone()).await {
        error!("Failed to initialize Raft cluster: {}", e);
        std::process::exit(1);
    }

    // Display Raft metrics
    let metrics = raft_manager.metrics().await;
    info!("Raft metrics: {:?}", metrics);
    info!("Cluster size: {} nodes", cluster_members.len());
    
    if metrics.is_leader {
        info!("This node is the leader - ready to accept writes");
    } else {
        info!("This node is a follower - will redirect writes to leader");
    }

    // Create TCP server
    let server = TcpServer::new(Arc::clone(&database), address.clone());

    info!("Server ready for connections with automatic failover");

    // Start server (this will block the main thread)
    if let Err(e) = server.start().await {
        error!("Server error: {}", e);
        
        // Cleanup Raft
        let _ = raft_manager.shutdown().await;
        
        std::process::exit(1);
    }

    Ok(())
}
