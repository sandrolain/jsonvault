use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jsonvault::{Command, Database};
use serde_json::json;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn benchmark_set_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let database = Arc::new(Database::new());

    c.bench_function("set_simple_json", |b| {
        let db = Arc::clone(&database);
        b.iter(|| {
            rt.block_on(async {
                let value = json!({"name": "test", "value": 42});
                let command = Command::Set {
                    key: format!("key_{}", fastrand::u32(..)),
                    value,
                };
                black_box(db.execute_command(command).await);
            });
        });
    });

    c.bench_function("set_complex_json", |b| {
        let db = Arc::clone(&database);
        b.iter(|| {
            rt.block_on(async {
                let value = json!({
                    "user": {
                        "id": 123,
                        "profile": {
                            "name": "Test User",
                            "email": "test@example.com",
                            "preferences": {
                                "theme": "dark",
                                "notifications": true,
                                "language": "en"
                            }
                        },
                        "history": [
                            {"action": "login", "timestamp": "2023-01-01T00:00:00Z"},
                            {"action": "update_profile", "timestamp": "2023-01-02T00:00:00Z"}
                        ]
                    }
                });
                let command = Command::Set {
                    key: format!("complex_key_{}", fastrand::u32(..)),
                    value,
                };
                black_box(db.execute_command(command).await);
            });
        });
    });
}

fn benchmark_get_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let database = Arc::new(Database::new());

    // Pre-populate the database
    rt.block_on(async {
        for i in 0..1000 {
            let value = json!({"id": i, "data": format!("test_data_{}", i)});
            let command = Command::Set {
                key: format!("bench_key_{}", i),
                value,
            };
            database.execute_command(command).await;
        }
    });

    c.bench_function("get_existing_key", |b| {
        let db = Arc::clone(&database);
        b.iter(|| {
            rt.block_on(async {
                let key = format!("bench_key_{}", fastrand::u32(..1000));
                let command = Command::Get { key };
                black_box(db.execute_command(command).await);
            });
        });
    });

    c.bench_function("get_nonexistent_key", |b| {
        let db = Arc::clone(&database);
        b.iter(|| {
            rt.block_on(async {
                let key = format!("nonexistent_key_{}", fastrand::u32(..));
                let command = Command::Get { key };
                black_box(db.execute_command(command).await);
            });
        });
    });
}

fn benchmark_jq_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let database = Arc::new(Database::new());

    // Pre-populate with complex JSON
    rt.block_on(async {
        let complex_value = json!({
            "users": [
                {"name": "Alice", "age": 30, "city": "New York"},
                {"name": "Bob", "age": 25, "city": "London"},
                {"name": "Charlie", "age": 35, "city": "Paris"}
            ],
            "metadata": {
                "total": 3,
                "last_updated": "2023-01-01T00:00:00Z"
            }
        });
        let command = Command::Set {
            key: "jq_test_data".to_string(),
            value: complex_value,
        };
        database.execute_command(command).await;
    });

    c.bench_function("jsonpath_simple_query", |b| {
        let db = Arc::clone(&database);
        b.iter(|| {
            rt.block_on(async {
                let command = Command::QGet {
                    key: "jq_test_data".to_string(),
                    query: "$.metadata.total".to_string(),
                };
                black_box(db.execute_command(command).await);
            });
        });
    });

    c.bench_function("jsonpath_complex_query", |b| {
        let db = Arc::clone(&database);
        b.iter(|| {
            rt.block_on(async {
                let command = Command::QGet {
                    key: "jq_test_data".to_string(),
                    query: "$.users[?(@.age > 25)].name".to_string(),
                };
                black_box(db.execute_command(command).await);
            });
        });
    });
}

fn benchmark_merge_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let database = Arc::new(Database::new());

    // Pre-populate with base data
    rt.block_on(async {
        let base_value = json!({
            "config": {
                "database": {"host": "localhost", "port": 5432},
                "cache": {"enabled": false}
            }
        });
        let command = Command::Set {
            key: "merge_test".to_string(),
            value: base_value,
        };
        database.execute_command(command).await;
    });

    c.bench_function("merge_simple", |b| {
        let db = Arc::clone(&database);
        b.iter(|| {
            rt.block_on(async {
                let merge_value = json!({"config": {"cache": {"enabled": true, "ttl": 3600}}});
                let command = Command::Merge {
                    key: "merge_test".to_string(),
                    value: merge_value,
                };
                black_box(db.execute_command(command).await);
            });
        });
    });

    c.bench_function("merge_complex", |b| {
        let db = Arc::clone(&database);
        b.iter(|| {
            rt.block_on(async {
                let merge_value = json!({
                    "config": {
                        "database": {"ssl": true, "pool_size": 10},
                        "logging": {"level": "info", "file": "/var/log/app.log"},
                        "features": ["analytics", "monitoring"]
                    },
                    "version": "1.0.0"
                });
                let command = Command::Merge {
                    key: "merge_test".to_string(),
                    value: merge_value,
                };
                black_box(db.execute_command(command).await);
            });
        });
    });
}

fn benchmark_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let database = Arc::new(Database::new());

    c.bench_function("concurrent_set_get", |b| {
        let db = Arc::clone(&database);
        b.iter(|| {
            rt.block_on(async {
                let mut tasks = Vec::new();

                // Spawn multiple concurrent operations
                for i in 0..10 {
                    let db_clone = Arc::clone(&db);
                    let task = tokio::spawn(async move {
                        // Set operation
                        let set_cmd = Command::Set {
                            key: format!("concurrent_key_{}", i),
                            value: json!({"id": i, "concurrent": true}),
                        };
                        db_clone.execute_command(set_cmd).await;

                        // Get operation
                        let get_cmd = Command::Get {
                            key: format!("concurrent_key_{}", i),
                        };
                        db_clone.execute_command(get_cmd).await
                    });
                    tasks.push(task);
                }

                // Wait for all tasks
                for task in tasks {
                    black_box(task.await.unwrap());
                }
            });
        });
    });
}

criterion_group!(
    benches,
    benchmark_set_operations,
    benchmark_get_operations,
    benchmark_jq_operations,
    benchmark_merge_operations,
    benchmark_concurrent_operations
);
criterion_main!(benches);
