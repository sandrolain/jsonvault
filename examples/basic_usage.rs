use jsonvault::{Command, Database};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Esempio di utilizzo del database JSON");

    // Crea una nuova istanza del database
    let database = Arc::new(Database::new());

    println!("\n📝 Impostazione valori...");

    // Imposta alcuni valori di esempio
    let user_data = json!({
        "id": 1,
        "name": "Alice",
        "email": "alice@example.com",
        "profile": {
            "age": 28,
            "city": "Milano",
            "preferences": {
                "theme": "dark",
                "language": "it"
            }
        }
    });

    let set_cmd = Command::Set {
        key: "user:1".to_string(),
        value: user_data,
    };

    let response = database.execute_command(set_cmd).await;
    println!("SET user:1 -> {:?}", response);

    // Imposta dati di configurazione
    let config_data = json!({
        "app": {
            "name": "Rust JSON DB",
            "version": "0.1.0"
        },
        "database": {
            "max_connections": 100,
            "timeout": 30
        }
    });

    let set_cmd = Command::Set {
        key: "config".to_string(),
        value: config_data,
    };

    let response = database.execute_command(set_cmd).await;
    println!("SET config -> {:?}", response);

    println!("\n📖 Lettura valori...");

    // Leggi i valori
    let get_cmd = Command::Get {
        key: "user:1".to_string(),
    };
    let response = database.execute_command(get_cmd).await;
    println!("GET user:1 -> {:?}", response);

    println!("\n🔍 Query JSONPath...");

    // Query JSONPath per estrarre il nome dell'utente
    let jq_cmd = Command::QGet {
        key: "user:1".to_string(),
        query: "$.name".to_string(),
    };
    let response = database.execute_command(jq_cmd).await;
    println!("JSONPath user:1 $.name -> {:?}", response);

    // Query JSONPath per estrarre l'età
    let jq_cmd = Command::QGet {
        key: "user:1".to_string(),
        query: "$.profile.age".to_string(),
    };
    let response = database.execute_command(jq_cmd).await;
    println!("JSONPath user:1 $.profile.age -> {:?}", response);

    // Query JSONPath per estrarre la città
    let jq_cmd = Command::QGet {
        key: "user:1".to_string(),
        query: "$.profile.city".to_string(),
    };
    let response = database.execute_command(jq_cmd).await;
    println!("JSONPath user:1 $.profile.city -> {:?}", response);

    println!("\n🔀 Merge di valori...");

    // Merge per aggiornare il profilo utente
    let merge_data = json!({
        "profile": {
            "age": 29,
            "phone": "+39 123 456 789",
            "preferences": {
                "notifications": true
            }
        },
        "last_login": "2024-01-15T10:30:00Z"
    });

    let merge_cmd = Command::Merge {
        key: "user:1".to_string(),
        value: merge_data,
    };
    let response = database.execute_command(merge_cmd).await;
    println!("MERGE user:1 -> {:?}", response);

    // Verifica il risultato del merge
    let get_cmd = Command::Get {
        key: "user:1".to_string(),
    };
    let response = database.execute_command(get_cmd).await;
    println!("GET user:1 (dopo merge) -> {:?}", response);

    println!("\n📊 Statistiche database...");
    println!("Numero di chiavi: {}", database.len());
    println!("Database vuoto: {}", database.is_empty());

    println!("\n🗑️ Cancellazione...");

    // Cancella una chiave
    let delete_cmd = Command::Delete {
        key: "config".to_string(),
    };
    let response = database.execute_command(delete_cmd).await;
    println!("DELETE config -> {:?}", response);

    // Verifica che sia stata cancellata
    let get_cmd = Command::Get {
        key: "config".to_string(),
    };
    let response = database.execute_command(get_cmd).await;
    println!("GET config (dopo delete) -> {:?}", response);

    println!("\n📊 Statistiche finali...");
    println!("Numero di chiavi: {}", database.len());

    println!("\n✅ Esempio completato!");

    Ok(())
}
