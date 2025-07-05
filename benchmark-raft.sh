#!/bin/bash

# Benchmark script per JsonVault con Raft abilitato

echo "=== JsonVault Raft Performance Benchmark ==="

# Pulisci processi precedenti
pkill -f "jsonvault-server\|server" || true
sleep 1

# Compila il progetto
echo "Building JsonVault..."
cargo build --release
if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo "Starting JsonVault with Raft..."

# Avvia il server con Raft
RUST_LOG=warn ./target/release/server \
    --enable-raft \
    --address "127.0.0.1:8080" \
    --node-id "benchmark" &
SERVER_PID=$!

# Aspetta che il server si avvii
sleep 2

echo "Running benchmarks..."

# Benchmark 1: SET operations
echo -e "\n=== Benchmark 1: SET Operations ==="
START_TIME=$(date +%s%N)
for i in {1..1000}; do
    echo "{\"command\": \"set\", \"key\": \"bench_$i\", \"value\": {\"id\": $i, \"data\": \"test_data_$i\", \"timestamp\": \"$(date -Iseconds)\"}}" | nc -w 1 127.0.0.1 8080 > /dev/null
done
END_TIME=$(date +%s%N)
SET_DURATION=$((($END_TIME - $START_TIME) / 1000000))
SET_OPS_PER_SEC=$((1000 * 1000 / $SET_DURATION))
echo "1000 SET operations completed in ${SET_DURATION}ms"
echo "Performance: ${SET_OPS_PER_SEC} ops/sec"

# Benchmark 2: GET operations
echo -e "\n=== Benchmark 2: GET Operations ==="
START_TIME=$(date +%s%N)
for i in {1..1000}; do
    echo "{\"command\": \"get\", \"key\": \"bench_$i\"}" | nc -w 1 127.0.0.1 8080 > /dev/null
done
END_TIME=$(date +%s%N)
GET_DURATION=$((($END_TIME - $START_TIME) / 1000000))
GET_OPS_PER_SEC=$((1000 * 1000 / $GET_DURATION))
echo "1000 GET operations completed in ${GET_DURATION}ms"
echo "Performance: ${GET_OPS_PER_SEC} ops/sec"

# Benchmark 3: JSONPath queries
echo -e "\n=== Benchmark 3: JSONPath Query Operations ==="
START_TIME=$(date +%s%N)
for i in {1..500}; do
    echo "{\"command\": \"qget\", \"key\": \"bench_$i\", \"query\": \"$.id\"}" | nc -w 1 127.0.0.1 8080 > /dev/null
done
END_TIME=$(date +%s%N)
QGET_DURATION=$((($END_TIME - $START_TIME) / 1000000))
QGET_OPS_PER_SEC=$((500 * 1000 / $QGET_DURATION))
echo "500 JSONPath query operations completed in ${QGET_DURATION}ms"
echo "Performance: ${QGET_OPS_PER_SEC} ops/sec"

# Benchmark 4: Mixed operations
echo -e "\n=== Benchmark 4: Mixed Operations ==="
START_TIME=$(date +%s%N)
for i in {1..200}; do
    # SET
    echo "{\"command\": \"set\", \"key\": \"mixed_$i\", \"value\": {\"counter\": $i}}" | nc -w 1 127.0.0.1 8080 > /dev/null
    # GET
    echo "{\"command\": \"get\", \"key\": \"mixed_$i\"}" | nc -w 1 127.0.0.1 8080 > /dev/null
    # QGET
    echo "{\"command\": \"qget\", \"key\": \"mixed_$i\", \"query\": \"$.counter\"}" | nc -w 1 127.0.0.1 8080 > /dev/null
    # DELETE
    echo "{\"command\": \"delete\", \"key\": \"mixed_$i\"}" | nc -w 1 127.0.0.1 8080 > /dev/null
done
END_TIME=$(date +%s%N)
MIXED_DURATION=$((($END_TIME - $START_TIME) / 1000000))
MIXED_OPS_PER_SEC=$((800 * 1000 / $MIXED_DURATION))  # 4 ops per iteration * 200 iterations
echo "800 mixed operations completed in ${MIXED_DURATION}ms"
echo "Performance: ${MIXED_OPS_PER_SEC} ops/sec"

echo -e "\n=== Summary ==="
echo "SET Operations:     ${SET_OPS_PER_SEC} ops/sec"
echo "GET Operations:     ${GET_OPS_PER_SEC} ops/sec"
echo "JSONPath Queries:   ${QGET_OPS_PER_SEC} ops/sec"
echo "Mixed Operations:   ${MIXED_OPS_PER_SEC} ops/sec"

# Cleanup
echo -e "\nStopping server..."
kill $SERVER_PID 2>/dev/null || true
wait

echo "Benchmark completed."
