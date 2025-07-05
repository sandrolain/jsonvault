use jsonvault::{RaftManager, Database, Command, ClusterMetrics};
use std::sync::Arc;
use tokio;
use serde_json::json;

/// Esempio che dimostra l'utilizzo di JsonVault con Raft consensus
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== JsonVault Raft Example ===");

    // Crea il database
    let database = Arc::new(Database::new());
    println!("✓ Database created");

    // Crea il manager Raft
    let mut raft_manager = RaftManager::new(1, Arc::clone(&database)).await?;
    println!("✓ Raft manager created");

    // Inizializza un cluster single-node
    raft_manager.initialize_cluster(vec![1]).await?;
    println!("✓ Single-node cluster initialized");

    // Aspetta un momento per l'inizializzazione completa
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verifica che siamo leader
    if raft_manager.is_leader().await {
        println!("✓ Node is leader");
    } else {
        println!("✗ Node is not leader");
        return Ok(());
    }

    // Mostra le metriche iniziali
    let metrics = raft_manager.metrics().await;
    println!("📊 Initial metrics: {:?}", metrics);

    println!("\n=== Testing Raft Consensus Operations ===");

    // Test 1: SET operation
    println!("\n1. Testing SET operation...");
    let set_command = Command::Set {
        key: "user:1".to_string(),
        value: json!({
            "id": 1,
            "name": "Alice",
            "email": "alice@example.com",
            "role": "admin",
            "created_at": "2025-07-05T12:00:00Z"
        }),
    };

    match raft_manager.submit_command(set_command).await {
        Ok(response) => println!("   ✓ SET successful: {:?}", response),
        Err(e) => println!("   ✗ SET failed: {}", e),
    }

    // Test 2: GET operation
    println!("\n2. Testing GET operation...");
    let get_command = Command::Get {
        key: "user:1".to_string(),
    };

    match raft_manager.submit_command(get_command).await {
        Ok(response) => println!("   ✓ GET successful: {:?}", response),
        Err(e) => println!("   ✗ GET failed: {}", e),
    }

    // Test 3: JSONPath query
    println!("\n3. Testing JSONPath query...");
    let qget_command = Command::QGet {
        key: "user:1".to_string(),
        query: "$.name".to_string(),
    };

    match raft_manager.submit_command(qget_command).await {
        Ok(response) => println!("   ✓ JSONPath query successful: {:?}", response),
        Err(e) => println!("   ✗ JSONPath query failed: {}", e),
    }

    // Test 4: Merge operation
    println!("\n4. Testing MERGE operation...");
    let merge_command = Command::Merge {
        key: "user:1".to_string(),
        value: json!({
            "last_login": "2025-07-05T12:30:00Z",
            "login_count": 1
        }),
    };

    match raft_manager.submit_command(merge_command).await {
        Ok(response) => println!("   ✓ MERGE successful: {:?}", response),
        Err(e) => println!("   ✗ MERGE failed: {}", e),
    }

    // Verifica il risultato del merge
    println!("\n5. Verifying MERGE result...");
    let verify_command = Command::Get {
        key: "user:1".to_string(),
    };

    match raft_manager.submit_command(verify_command).await {
        Ok(response) => println!("   ✓ Merged data: {:?}", response),
        Err(e) => println!("   ✗ Verification failed: {}", e),
    }

    // Test 6: Batch operations
    println!("\n6. Testing batch operations...");
    for i in 2..=5 {
        let command = Command::Set {
            key: format!("user:{}", i),
            value: json!({
                "id": i,
                "name": format!("User{}", i),
                "email": format!("user{}@example.com", i),
                "role": "user"
            }),
        };

        match raft_manager.submit_command(command).await {
            Ok(_) => println!("   ✓ User {} created", i),
            Err(e) => println!("   ✗ Failed to create user {}: {}", i, e),
        }
    }

    // Mostra le metriche finali
    println!("\n=== Final Metrics ===");
    let final_metrics = raft_manager.metrics().await;
    println!("📊 Final metrics: {:?}", final_metrics);

    // Simulazione di uno scenario di failover
    println!("\n=== Simulating Leadership Scenarios ===");
    println!("Leader ID: {:?}", raft_manager.leader_id().await);
    println!("Is Leader: {}", raft_manager.is_leader().await);

    // Test delle performance
    println!("\n=== Performance Test ===");
    let start = std::time::Instant::now();
    let num_ops = 100;

    for i in 0..num_ops {
        let command = Command::Set {
            key: format!("perf_test:{}", i),
            value: json!({
                "index": i,
                "data": format!("test_data_{}", i),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        };

        raft_manager.submit_command(command).await?;
    }

    let duration = start.elapsed();
    let ops_per_sec = num_ops as f64 / duration.as_secs_f64();
    println!("✓ {} operations completed in {:?}", num_ops, duration);
    println!("✓ Performance: {:.2} ops/sec", ops_per_sec);

    // Cleanup
    println!("\n=== Cleanup ===");
    raft_manager.shutdown().await?;
    println!("✓ Raft manager shut down successfully");

    println!("\n🎉 JsonVault Raft example completed successfully!");

    Ok(())
}
