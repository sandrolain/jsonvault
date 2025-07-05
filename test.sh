#!/bin/bash

# Script to test the complete functionality of the JSON database

set -e

echo "🚀 Starting complete JSON database test..."

# Compile the project
echo "📦 Compiling project..."
cargo build --release

# Start the server in background
echo "🔧 Starting primary server..."
cargo run --bin server --release -- --address 127.0.0.1:8090 --primary --node-id master &
SERVER_PID=$!

# Wait for server to start
sleep 2

echo "✅ Server started (PID: $SERVER_PID)"

# Test basic operations
echo "🧪 Testing basic operations..."

# SET
echo "📝 Testing SET..."
cargo run --bin client --release -- --server 127.0.0.1:8090 set user '{"name": "Mario Rossi", "age": 30, "city": "Roma"}'

# GET
echo "📖 Testing GET..."
cargo run --bin client --release -- --server 127.0.0.1:8090 get user

# JSONPath Query (qget)
echo "🔍 Testing JSONPath query (qget)..."
cargo run --bin client --release -- --server 127.0.0.1:8090 qget user '$.name'

# QSET - Set sub-property
echo "🎯 Testing QSET..."
cargo run --bin client --release -- --server 127.0.0.1:8090 qset user 'profession' '"Developer"'

# GET after qset
echo "📖 Testing GET after qset..."
cargo run --bin client --release -- --server 127.0.0.1:8090 get user

# MERGE
echo "🔀 Testing MERGE..."
cargo run --bin client --release -- --server 127.0.0.1:8090 merge user '{"age": 31, "country": "Italy"}'

# GET after merge
echo "📖 Testing GET after merge..."
cargo run --bin client --release -- --server 127.0.0.1:8090 get user

# MERGE on non-existent key
echo "🔀 Testing MERGE on non-existent key..."
cargo run --bin client --release -- --server 127.0.0.1:8090 merge device '{"type": "sensor", "status": "active"}'

# GET after merge on non-existent key
echo "📖 Testing GET after merge on non-existent key..."
cargo run --bin client --release -- --server 127.0.0.1:8090 get device

# Test with complex data
echo "🏗️ Testing with complex data..."
cargo run --bin client --release -- --server 127.0.0.1:8090 set config '{
  "database": {
    "host": "localhost",
    "port": 5432,
    "ssl": true
  },
  "cache": {
    "enabled": true,
    "ttl": 3600
  },
  "features": ["analytics", "monitoring"]
}'

# Complex JSONPath queries
echo "🔬 Testing complex JSONPath queries..."
cargo run --bin client --release -- --server 127.0.0.1:8090 qget config '$.database.*'
cargo run --bin client --release -- --server 127.0.0.1:8090 qget config '$.features[*]'

# Test QSET with nested paths
echo "🎯 Testing QSET with nested paths..."
cargo run --bin client --release -- --server 127.0.0.1:8090 qset config 'database.timeout' '30'
cargo run --bin client --release -- --server 127.0.0.1:8090 qset config 'new_section.enabled' 'true'

# GET after nested qset
echo "📖 Testing GET after nested qset..."
cargo run --bin client --release -- --server 127.0.0.1:8090 get config

# PING
echo "🏓 Testing PING..."
cargo run --bin client --release -- --server 127.0.0.1:8090 ping

# Start a replica
echo "🔄 Starting replica..."
cargo run --bin server --release -- --address 127.0.0.1:8091 --replica-of 127.0.0.1:8090 --node-id replica-1 &
REPLICA_PID=$!

sleep 2

echo "✅ Replica started (PID: $REPLICA_PID)"

# Test connection to replica
echo "📡 Testing connection to replica..."
cargo run --bin client --release -- --server 127.0.0.1:8091 ping
cargo run --bin client --release -- --server 127.0.0.1:8091 get user

# DELETE
echo "🗑️ Testing DELETE..."
cargo run --bin client --release -- --server 127.0.0.1:8090 delete config
cargo run --bin client --release -- --server 127.0.0.1:8090 get config

echo "✅ All tests completed successfully!"

# Cleanup
echo "🧹 Cleaning up processes..."
kill $SERVER_PID $REPLICA_PID
wait $SERVER_PID $REPLICA_PID 2>/dev/null || true

echo "🎉 Test completed!"
