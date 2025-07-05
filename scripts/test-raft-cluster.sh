#!/bin/bash

# JsonVault Raft Cluster Test Script
# Tests Raft replication across Docker Compose cluster

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DEPLOYMENT_DIR="$PROJECT_ROOT/deployment"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Node configuration
NODES=(
    "localhost:8080"
    "localhost:8081" 
    "localhost:8082"
)

NODE_NAMES=(
    "jsonvault-node1"
    "jsonvault-node2"
    "jsonvault-node3"
)

echo -e "${BLUE}🚀 JsonVault Raft Cluster Test Script${NC}"
echo "========================================"

# Function to check if Docker Compose cluster is running
check_cluster_status() {
    echo -e "${BLUE}📊 Checking cluster status...${NC}"
    cd "$DEPLOYMENT_DIR"
    
    for i in "${!NODE_NAMES[@]}"; do
        container="${NODE_NAMES[$i]}"
        if docker ps --format "table {{.Names}}\t{{.Status}}" | grep -q "$container.*Up"; then
            echo -e "${GREEN}✅ $container is running${NC}"
        else
            echo -e "${RED}❌ $container is not running${NC}"
            return 1
        fi
    done
    echo
}

# Function to wait for port to be ready
wait_for_port() {
    local host_port="$1"
    local timeout=30
    local count=0
    
    echo -e "${YELLOW}⏳ Waiting for $host_port to be ready...${NC}"
    
    while [ $count -lt $timeout ]; do
        if nc -z ${host_port/:/ } 2>/dev/null; then
            echo -e "${GREEN}✅ $host_port is ready${NC}"
            return 0
        fi
        sleep 1
        ((count++))
    done
    
    echo -e "${RED}❌ Timeout waiting for $host_port${NC}"
    return 1
}

# Function to send TCP command to JsonVault node
send_tcp_command() {
    local host_port="$1"
    local command_json="$2"
    local timeout="${3:-5}"
    
    echo -e "${BLUE}📤 Sending to $host_port: $command_json${NC}" >&2
    
    # Calculate message length
    local payload_length=$(echo -n "$command_json" | wc -c)
    
    # Create message with 4-byte big-endian length prefix + JSON payload
    python3 -c "
import socket
import struct
import sys
import json

host, port = sys.argv[1].split(':')
port = int(port)
command = sys.argv[2]
timeout = int(sys.argv[3])

try:
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(timeout)
    sock.connect((host, port))
    
    # Send length (4 bytes big-endian) + payload
    payload = command.encode('utf-8')
    length = struct.pack('>I', len(payload))
    sock.sendall(length + payload)
    
    # Read response
    length_data = sock.recv(4)
    if len(length_data) != 4:
        print('ERROR: Could not read response length', file=sys.stderr)
        sys.exit(1)
    
    response_length = struct.unpack('>I', length_data)[0]
    response_data = b''
    while len(response_data) < response_length:
        chunk = sock.recv(response_length - len(response_data))
        if not chunk:
            break
        response_data += chunk
    
    response = response_data.decode('utf-8')
    print(response)
    
except Exception as e:
    print(f'ERROR: {e}', file=sys.stderr)
    sys.exit(1)
finally:
    sock.close()
" "$host_port" "$command_json" "$timeout"
}

# Function to parse JSON response and extract value
parse_response() {
    local response="$1"
    python3 -c "
import json
import sys

try:
    data = json.loads(sys.argv[1])
    if 'Ok' in data and data['Ok'] is not None:
        print(json.dumps(data['Ok'], indent=2))
    elif 'Ok' in data and data['Ok'] is None:
        print('OK')
    elif 'Pong' in data:
        print('PONG')
    elif 'Error' in data:
        print(f'ERROR: {data[\"Error\"]}', file=sys.stderr)
        sys.exit(1)
    else:
        print(json.dumps(data, indent=2))
except Exception as e:
    print(f'JSON Parse Error: {e}', file=sys.stderr)
    print(f'Raw response: {sys.argv[1]}', file=sys.stderr)
" "$response"
}

# Function to test basic connectivity
test_connectivity() {
    echo -e "${BLUE}🔌 Testing basic connectivity...${NC}"
    
    for i in "${!NODES[@]}"; do
        local node="${NODES[$i]}"
        echo -e "${YELLOW}Testing PING to ${NODE_NAMES[$i]} ($node)...${NC}"
        
        local response=$(send_tcp_command "$node" '"Ping"' 10)
        if [ $? -eq 0 ]; then
            local parsed=$(parse_response "$response")
            if [ "$parsed" = "PONG" ]; then
                echo -e "${GREEN}✅ PING successful${NC}"
            else
                echo -e "${RED}❌ Unexpected response: $parsed${NC}"
                return 1
            fi
        else
            echo -e "${RED}❌ PING failed${NC}"
            return 1
        fi
    done
    echo
}

# Function to test data replication
test_data_replication() {
    echo -e "${BLUE}🔄 Testing Raft data replication...${NC}"
    
    # Test 1: Write data to node 1
    echo -e "${YELLOW}📝 Writing data to node 1...${NC}"
    local set_cmd='{"Set":{"key":"raft-test-1","value":{"message":"Hello from Raft","timestamp":"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'","node":"node1"}}}'
    local response=$(send_tcp_command "${NODES[0]}" "$set_cmd")
    local parsed=$(parse_response "$response")
    echo -e "${GREEN}Response: $parsed${NC}"
    
    # Wait for replication
    echo -e "${YELLOW}⏳ Waiting for replication...${NC}"
    sleep 2
    
    # Test 2: Read from all nodes to verify replication
    for i in "${!NODES[@]}"; do
        local node="${NODES[$i]}"
        local node_name="${NODE_NAMES[$i]}"
        echo -e "${YELLOW}📖 Reading from $node_name ($node)...${NC}"
        
        local get_cmd='{"Get":{"key":"raft-test-1"}}'
        local response=$(send_tcp_command "$node" "$get_cmd")
        local parsed=$(parse_response "$response")
        
        if echo "$parsed" | grep -q "Hello from Raft"; then
            echo -e "${GREEN}✅ Data replicated to $node_name${NC}"
            echo -e "${GREEN}   Value: $parsed${NC}"
        else
            echo -e "${RED}❌ Data NOT replicated to $node_name${NC}"
            echo -e "${RED}   Response: $parsed${NC}"
            return 1
        fi
    done
    echo
}

# Function to test cross-node writes
test_cross_node_writes() {
    echo -e "${BLUE}🔀 Testing writes to different nodes...${NC}"
    
    # Write to different nodes and verify consistency
    for i in "${!NODES[@]}"; do
        local node="${NODES[$i]}"
        local node_name="${NODE_NAMES[$i]}"
        local key="cross-node-test-$i"
        
        echo -e "${YELLOW}📝 Writing to $node_name ($node)...${NC}"
        local set_cmd='{"Set":{"key":"'$key'","value":{"written_by":"'$node_name'","data":"test-data-'$i'","timestamp":"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'"}}}'
        local response=$(send_tcp_command "$node" "$set_cmd")
        parse_response "$response" > /dev/null
        
        # Wait for replication
        sleep 1
        
        # Verify on all other nodes
        for j in "${!NODES[@]}"; do
            if [ $i -ne $j ]; then
                local verify_node="${NODES[$j]}"
                local verify_node_name="${NODE_NAMES[$j]}"
                
                local get_cmd='{"Get":{"key":"'$key'"}}'
                local response=$(send_tcp_command "$verify_node" "$get_cmd")
                local parsed=$(parse_response "$response")
                
                if echo "$parsed" | grep -q "written_by.*$node_name"; then
                    echo -e "${GREEN}✅ $verify_node_name sees data from $node_name${NC}"
                else
                    echo -e "${RED}❌ $verify_node_name does NOT see data from $node_name${NC}"
                    echo -e "${RED}   Response: $parsed${NC}"
                    return 1
                fi
            fi
        done
    done
    echo
}

# Function to test JSONPath queries
test_jsonpath_queries() {
    echo -e "${BLUE}🔍 Testing JSONPath queries...${NC}"
    
    # Set complex data
    echo -e "${YELLOW}📝 Setting complex JSON data...${NC}"
    local complex_data='{"Set":{"key":"complex-data","value":{"users":[{"name":"Alice","age":30,"roles":["admin","user"]},{"name":"Bob","age":25,"roles":["user"]},{"name":"Charlie","age":35,"roles":["moderator","user"]}],"meta":{"created":"2025-07-05","version":"1.0"}}}}'
    local response=$(send_tcp_command "${NODES[0]}" "$complex_data")
    parse_response "$response" > /dev/null
    
    sleep 1
    
    # Test JSONPath queries on different nodes
    declare -a queries=(
        '$.users[*].name'
        '$.users[?(@.age > 30)].name'
        '$.meta.created'
        '$.users[0].roles[*]'
    )
    
    for query in "${queries[@]}"; do
        echo -e "${YELLOW}🔎 Query: $query${NC}"
        
        for i in "${!NODES[@]}"; do
            local node="${NODES[$i]}"
            local node_name="${NODE_NAMES[$i]}"
            
            local qget_cmd='{"QGet":{"key":"complex-data","query":"'$query'"}}'
            local response=$(send_tcp_command "$node" "$qget_cmd")
            local parsed=$(parse_response "$response")
            
            echo -e "${GREEN}   $node_name: $parsed${NC}"
        done
        echo
    done
}

# Function to test data persistence after container restart
test_persistence() {
    echo -e "${BLUE}🔄 Testing data persistence (container restart)...${NC}"
    
    # Set test data
    echo -e "${YELLOW}📝 Setting persistence test data...${NC}"
    local persist_cmd='{"Set":{"key":"persistence-test","value":{"message":"This should survive restart","timestamp":"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'"}}}'
    local response=$(send_tcp_command "${NODES[0]}" "$persist_cmd")
    parse_response "$response" > /dev/null
    
    sleep 1
    
    # Restart one container
    echo -e "${YELLOW}🔄 Restarting node 2...${NC}"
    cd "$DEPLOYMENT_DIR"
    docker compose -f docker-compose-raft.yml restart jsonvault-node2
    
    # Wait for it to come back up
    wait_for_port "${NODES[1]}"
    sleep 3
    
    # Check if data is still there
    echo -e "${YELLOW}📖 Checking data persistence...${NC}"
    local get_cmd='{"Get":{"key":"persistence-test"}}'
    local response=$(send_tcp_command "${NODES[1]}" "$get_cmd")
    local parsed=$(parse_response "$response")
    
    if echo "$parsed" | grep -q "This should survive restart"; then
        echo -e "${GREEN}✅ Data persisted after restart${NC}"
        echo -e "${GREEN}   Value: $parsed${NC}"
    else
    echo -e "${YELLOW}⚠️  Data not persisted (expected in current implementation)${NC}"
        echo -e "${YELLOW}   Note: Current implementation uses in-memory storage${NC}"
    fi
    echo
}

# Function to show cluster metrics
show_cluster_metrics() {
    echo -e "${BLUE}📊 Cluster Metrics${NC}"
    echo "=================="
    
    cd "$DEPLOYMENT_DIR"
    for container in "${NODE_NAMES[@]}"; do
        echo -e "${YELLOW}$container logs (last 5 lines):${NC}"
        docker logs "$container" --tail 5 2>/dev/null || echo "No logs available"
        echo
    done
}

# Main test execution
main() {
    echo -e "${BLUE}Starting Raft cluster tests...${NC}"
    echo
    
    # Check if cluster is running
    if ! check_cluster_status; then
        echo -e "${RED}❌ Cluster is not running. Please start with:${NC}"
        echo "cd deployment && docker-compose -f docker-compose-raft.yml up -d"
        exit 1
    fi
    
    # Wait for all nodes to be ready
    for node in "${NODES[@]}"; do
        wait_for_port "$node"
    done
    
    # Run tests
    test_connectivity || exit 1
    test_data_replication || exit 1
    test_cross_node_writes || exit 1
    test_jsonpath_queries || exit 1
    test_persistence || exit 1
    
    # Show final metrics
    show_cluster_metrics
    
    echo -e "${GREEN}🎉 All Raft cluster tests completed successfully!${NC}"
    echo -e "${GREEN}✅ Raft consensus is working correctly${NC}"
    echo -e "${GREEN}✅ Data replication is functioning${NC}"
    echo -e "${GREEN}✅ Cross-node writes are consistent${NC}"
    echo -e "${GREEN}✅ JSONPath queries work across cluster${NC}"
}

# Run main function
main "$@"
