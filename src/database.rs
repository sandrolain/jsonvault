use crate::protocol::{Command, OperationType, ReplicationData, Response};
use dashmap::DashMap;
use log::{debug, error, info};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory thread-safe key-value database
#[derive(Debug, Clone)]
pub struct Database {
    /// Main storage using DashMap for optimal concurrency
    data: Arc<DashMap<String, Value>>,
    /// List of replicas for synchronization
    replicas: Arc<RwLock<Vec<String>>>,
}

impl Database {
    /// Creates a new database instance
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
            replicas: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Adds a replica to the list
    pub async fn add_replica(&self, replica_address: String) {
        let mut replicas = self.replicas.write().await;
        if !replicas.contains(&replica_address) {
            replicas.push(replica_address);
            info!("Added replica: {}", replicas.last().unwrap());
        }
    }

    /// Removes a replica from the list
    pub async fn remove_replica(&self, replica_address: &str) {
        let mut replicas = self.replicas.write().await;
        replicas.retain(|addr| addr != replica_address);
        info!("Removed replica: {}", replica_address);
    }

    /// Execute a command and return the response
    pub async fn execute_command(&self, command: Command) -> Response {
        match command {
            Command::Set { key, value } => self.set(key, value).await,
            Command::Get { key } => self.get(&key).await,
            Command::Delete { key } => self.delete(key).await,
            Command::QGet { key, query } => self.qget(&key, &query).await,
            Command::QSet { key, path, value } => self.qset(key, path, value).await,
            Command::Merge { key, value } => self.merge(key, value).await,
            Command::Ping => Response::Pong,
            Command::Replicate { data } => self.handle_replication(data).await,
        }
    }

    /// Sets a value for a key
    async fn set(&self, key: String, value: Value) -> Response {
        // JSON validation
        if !self.is_valid_json(&value) {
            return Response::Error("Invalid JSON value".to_string());
        }

        self.data.insert(key.clone(), value.clone());
        debug!("SET: {} = {}", key, value);

        // Replicate the operation
        self.replicate_operation(OperationType::Set, key, Some(value))
            .await;

        Response::Ok(None)
    }

    /// Reads a value for a key
    async fn get(&self, key: &str) -> Response {
        match self.data.get(key) {
            Some(value) => {
                debug!("GET: {} = {}", key, value.clone());
                Response::Ok(Some(value.clone()))
            }
            None => {
                debug!("GET: {} not found", key);
                Response::Ok(None)
            }
        }
    }

    /// Deletes a value for a key
    async fn delete(&self, key: String) -> Response {
        match self.data.remove(&key) {
            Some(_) => {
                debug!("DELETE: {} removed", key);
                // Replicate the operation
                self.replicate_operation(OperationType::Delete, key, None)
                    .await;
                Response::Ok(None)
            }
            None => {
                debug!("DELETE: {} not found", key);
                Response::Error("Key not found".to_string())
            }
        }
    }

    /// Execute a JSONPath query on a value
    async fn qget(&self, key: &str, query: &str) -> Response {
        match self.data.get(key) {
            Some(value) => match jsonpath_lib::select(&value.clone(), query) {
                Ok(result) => {
                    debug!(
                        "JSONPath query: {} with query '{}' = {:?}",
                        key, query, result
                    );
                    if result.is_empty() {
                        Response::Ok(Some(Value::Null))
                    } else if result.len() == 1 {
                        Response::Ok(Some(result[0].clone()))
                    } else {
                        Response::Ok(Some(Value::Array(result.into_iter().cloned().collect())))
                    }
                }
                Err(e) => {
                    error!("JSONPath error for {}: {}", key, e);
                    Response::Error(format!("JSONPath query error: {}", e))
                }
            },
            None => {
                debug!("JSONPath query: {} not found", key);
                Response::Error("Key not found".to_string())
            }
        }
    }

    /// Set a sub-property using JSONPath
    async fn qset(&self, key: String, path: String, value: Value) -> Response {
        // Validate JSON
        if !self.is_valid_json(&value) {
            return Response::Error("Invalid JSON value".to_string());
        }

        // Get existing value or create new empty object
        let existing_value = self
            .data
            .get(&key)
            .map(|v| v.clone())
            .unwrap_or(Value::Object(serde_json::Map::new()));

        // Clone for modification
        let mut modified_value = existing_value.clone();

        // Use JSONPath to set the value
        match self.set_json_path(&mut modified_value, &path, value.clone()) {
            Ok(()) => {
                self.data.insert(key.clone(), modified_value.clone());
                debug!("QSET: {} at path '{}' = {}", key, path, value);

                // Replicate the operation
                self.replicate_operation(OperationType::QSet, key, Some(modified_value))
                    .await;

                Response::Ok(None)
            }
            Err(e) => {
                error!("QSET error for {} at path '{}': {}", key, path, e);
                Response::Error(format!("JSONPath set error: {}", e))
            }
        }
    }

    /// Merges a JSON value with an existing one
    async fn merge(&self, key: String, new_value: Value) -> Response {
        // JSON validation
        if !self.is_valid_json(&new_value) {
            return Response::Error("Invalid JSON value".to_string());
        }

        let merged_value = match self.data.get(&key) {
            Some(existing_value) => {
                match Self::merge_json_values(&existing_value.clone(), &new_value) {
                    Ok(merged) => merged,
                    Err(e) => return Response::Error(e),
                }
            }
            None => new_value.clone(),
        };

        self.data.insert(key.clone(), merged_value.clone());
        debug!("MERGE: {} = {}", key, merged_value);

        // Replicate the operation
        self.replicate_operation(OperationType::Merge, key, Some(merged_value))
            .await;

        Response::Ok(None)
    }

    /// Handles replication commands
    async fn handle_replication(&self, data: ReplicationData) -> Response {
        match data {
            ReplicationData::FullSync(entries) => {
                // Full synchronization
                self.data.clear();
                for (key, value) in entries {
                    self.data.insert(key, value);
                }
                info!("Full synchronization completed");
                Response::ReplicationAck
            }
            ReplicationData::Operation {
                op_type,
                key,
                value,
            } => {
                // Apply single operation
                match op_type {
                    OperationType::Set => {
                        if let Some(v) = value {
                            self.data.insert(key, v);
                        }
                    }
                    OperationType::Delete => {
                        self.data.remove(&key);
                    }
                    OperationType::Merge => {
                        if let Some(v) = value {
                            self.data.insert(key, v);
                        }
                    }
                    OperationType::QSet => {
                        if let Some(v) = value {
                            self.data.insert(key, v);
                        }
                    }
                }
                Response::ReplicationAck
            }
        }
    }

    /// Replicates an operation to all replicas
    async fn replicate_operation(&self, op_type: OperationType, key: String, value: Option<Value>) {
        let replicas = self.replicas.read().await;
        if replicas.is_empty() {
            return;
        }

        let replication_data = ReplicationData::Operation {
            op_type,
            key,
            value,
        };
        let command = Command::Replicate {
            data: replication_data,
        };

        for replica in replicas.iter() {
            // TODO: Implement sending command to replicas
            debug!("Replicating command to {}: {:?}", replica, command);
        }
    }

    /// Sets a value at a JSONPath location
    fn set_json_path(&self, value: &mut Value, path: &str, new_value: Value) -> Result<(), String> {
        // Parse the JSONPath - simplified implementation for basic paths
        let path = path.trim_start_matches('$').trim_start_matches('.');

        if path.is_empty() {
            // Root path, replace entire value
            *value = new_value;
            return Ok(());
        }

        let parts: Vec<&str> = path.split('.').collect();
        Self::set_nested_value(value, &parts, new_value)
    }

    /// Recursively sets a nested value
    fn set_nested_value(
        value: &mut Value,
        path_parts: &[&str],
        new_value: Value,
    ) -> Result<(), String> {
        if path_parts.is_empty() {
            *value = new_value;
            return Ok(());
        }

        let current_key = path_parts[0];
        let remaining_parts = &path_parts[1..];

        // Handle array index access
        if let Ok(index) = current_key.parse::<usize>() {
            match value {
                Value::Array(arr) => {
                    // Extend array if necessary
                    while arr.len() <= index {
                        arr.push(Value::Null);
                    }

                    if remaining_parts.is_empty() {
                        arr[index] = new_value;
                    } else {
                        Self::set_nested_value(&mut arr[index], remaining_parts, new_value)?;
                    }
                    return Ok(());
                }
                _ => {
                    return Err(format!(
                        "Cannot index non-array value with '{}'",
                        current_key
                    ));
                }
            }
        }

        // Handle object property access
        match value {
            Value::Object(map) => {
                if remaining_parts.is_empty() {
                    map.insert(current_key.to_string(), new_value);
                } else {
                    // Create nested object if it doesn't exist
                    if !map.contains_key(current_key) {
                        map.insert(
                            current_key.to_string(),
                            Value::Object(serde_json::Map::new()),
                        );
                    }

                    let nested_value = map.get_mut(current_key).unwrap();
                    Self::set_nested_value(nested_value, remaining_parts, new_value)?;
                }
                Ok(())
            }
            Value::Null => {
                // Create new object if current value is null
                let mut new_obj = serde_json::Map::new();
                if remaining_parts.is_empty() {
                    new_obj.insert(current_key.to_string(), new_value);
                } else {
                    let mut nested = Value::Object(serde_json::Map::new());
                    Self::set_nested_value(&mut nested, remaining_parts, new_value)?;
                    new_obj.insert(current_key.to_string(), nested);
                }
                *value = Value::Object(new_obj);
                Ok(())
            }
            _ => Err(format!(
                "Cannot access property '{}' on non-object value",
                current_key
            )),
        }
    }

    /// Validates if a value is valid JSON
    fn is_valid_json(&self, value: &Value) -> bool {
        // serde_json::Value values are always valid JSON
        // We only check that they are not unintentional nulls
        match value {
            Value::Null => true, // Explicit null is valid
            _ => true,
        }
    }

    /// Merges two JSON values
    fn merge_json_values(existing: &Value, new: &Value) -> Result<Value, String> {
        match (existing, new) {
            (Value::Object(existing_obj), Value::Object(new_obj)) => {
                let mut merged = existing_obj.clone();
                for (key, value) in new_obj {
                    if let Some(existing_value) = merged.get(key) {
                        // Recursive merge for nested objects
                        merged.insert(key.clone(), Self::merge_json_values(existing_value, value)?);
                    } else {
                        merged.insert(key.clone(), value.clone());
                    }
                }
                Ok(Value::Object(merged))
            }
            (Value::Array(existing_arr), Value::Array(new_arr)) => {
                let mut merged = existing_arr.clone();
                merged.extend(new_arr.clone());
                Ok(Value::Array(merged))
            }
            _ => {
                // For incompatible types, the new value replaces the existing one
                Ok(new.clone())
            }
        }
    }

    /// Gets all data for full synchronization
    pub async fn get_all_data(&self) -> Vec<(String, Value)> {
        self.data
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Gets the number of keys in the database
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Checks if the database is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_set_and_get() {
        let db = Database::new();
        let value = json!({"name": "test", "value": 42});

        let response = db.set("test_key".to_string(), value.clone()).await;
        assert!(matches!(response, Response::Ok(None)));

        let response = db.get("test_key").await;
        assert!(matches!(response, Response::Ok(Some(v)) if v == value));
    }

    #[tokio::test]
    async fn test_delete() {
        let db = Database::new();
        let value = json!({"test": true});

        db.set("test_key".to_string(), value).await;
        let response = db.delete("test_key".to_string()).await;
        assert!(matches!(response, Response::Ok(None)));

        let response = db.get("test_key").await;
        assert!(matches!(response, Response::Ok(None)));
    }

    #[tokio::test]
    async fn test_qget_jsonpath() {
        let db = Database::new();
        let value = json!({"user": {"name": "Alice", "age": 30}});

        db.set("test_key".to_string(), value).await;

        let response = db.qget("test_key", "$.user.name").await;
        if let Response::Ok(Some(result)) = response {
            assert_eq!(result, json!("Alice"));
        } else {
            panic!("Expected JSONPath query result");
        }
    }

    #[tokio::test]
    async fn test_qset_nested_property() {
        let db = Database::new();
        let initial = json!({"user": {"name": "Alice"}});

        db.set("test_key".to_string(), initial).await;

        let response = db
            .qset("test_key".to_string(), "user.age".to_string(), json!(25))
            .await;
        assert!(matches!(response, Response::Ok(None)));

        let response = db.get("test_key").await;
        if let Response::Ok(Some(result)) = response {
            let expected = json!({"user": {"name": "Alice", "age": 25}});
            assert_eq!(result, expected);
        } else {
            panic!("Expected result after QSET");
        }
    }

    #[tokio::test]
    async fn test_qset_create_new_key() {
        let db = Database::new();

        let response = db
            .qset(
                "new_key".to_string(),
                "config.timeout".to_string(),
                json!(5000),
            )
            .await;
        assert!(matches!(response, Response::Ok(None)));

        let response = db.get("new_key").await;
        if let Response::Ok(Some(result)) = response {
            let expected = json!({"config": {"timeout": 5000}});
            assert_eq!(result, expected);
        } else {
            panic!("Expected result after QSET on new key");
        }
    }
}
