#!/bin/bash

# Script per avviare un cluster JsonVault con Raft per testing

echo "=== JsonVault Raft Cluster Test ==="

# Pulisci processi precedenti
pkill -f "jsonvault-server" || true
sleep 1

# Compila il progetto
echo "Building JsonVault..."
cargo build --release
if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo "Starting JsonVault cluster with Raft consensus..."

# Avvia il primo nodo (leader)
echo "Starting Node 1 (port 8080)..."
./target/release/server \
    --enable-raft \
    --address "127.0.0.1:8080" \
    --node-id "1" &
NODE1_PID=$!

# Aspetta che il primo nodo si avvii
sleep 2

# Test del primo nodo
echo -e "\n=== Testing single node cluster ==="
echo "Setting test data..."
echo '{"command": "set", "key": "test1", "value": {"message": "Hello from Raft!"}}' | \
    nc 127.0.0.1 8080

sleep 1

echo "Getting test data..."
echo '{"command": "get", "key": "test1"}' | \
    nc 127.0.0.1 8080

echo -e "\n=== Testing JSON Path queries ==="
echo '{"command": "set", "key": "user1", "value": {"name": "Alice", "age": 30, "city": "Rome"}}' | \
    nc 127.0.0.1 8080

sleep 1

echo '{"command": "qget", "key": "user1", "query": "$.name"}' | \
    nc 127.0.0.1 8080

echo -e "\n=== Cluster Status ==="
echo "Node 1 is running with PID: $NODE1_PID"

echo -e "\nPress Enter to stop the cluster..."
read

# Cleanup
echo "Stopping cluster..."
kill $NODE1_PID 2>/dev/null || true
wait

echo "Cluster stopped."
