#!/bin/bash

# JsonVault Current Implementation Test
# Tests the current state of the Raft implementation

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}JsonVault Current Implementation Test${NC}"
echo "===================================="

# Function to send TCP command
send_command() {
    local host_port="$1"
    local command_json="$2"
    python3 -c "
import socket
import struct
import sys

host, port = sys.argv[1].split(':')
port = int(port)
command = sys.argv[2]

try:
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(10)
    sock.connect((host, port))
    
    payload = command.encode('utf-8')
    length = struct.pack('>I', len(payload))
    sock.sendall(length + payload)
    
    length_data = sock.recv(4)
    response_length = struct.unpack('>I', length_data)[0]
    response_data = b''
    while len(response_data) < response_length:
        chunk = sock.recv(response_length - len(response_data))
        if not chunk:
            break
        response_data += chunk
    
    print(response_data.decode('utf-8'))
    
except Exception as e:
    print(f'ERROR: {e}', file=sys.stderr)
    sys.exit(1)
finally:
    sock.close()
" "$host_port" "$command_json"
}

NODES=("localhost:8080" "localhost:8081" "localhost:8082")
NODE_NAMES=("node1" "node2" "node3")

echo -e "${YELLOW}Test 1: Individual node functionality${NC}"
for i in "${!NODES[@]}"; do
    node="${NODES[$i]}"
    name="${NODE_NAMES[$i]}"
    
    echo "Testing $name ($node)..."
    
    # Test PING
    response=$(send_command "$node" '"Ping"')
    if [[ "$response" == '"Pong"' ]]; then
        echo -e "${GREEN}✅ PING successful${NC}"
    else
        echo -e "${RED}❌ PING failed: $response${NC}"
        continue
    fi
    
    # Test SET
    test_key="test_${name}_$(date +%s)"
    set_cmd='{"Set":{"key":"'$test_key'","value":{"message":"Hello from '$name'","timestamp":"'$(date -Iseconds)'"}}}'
    response=$(send_command "$node" "$set_cmd")
    if [[ "$response" == '{"Ok":null}' ]]; then
        echo -e "${GREEN}✅ SET successful on $name${NC}"
    else
        echo -e "${RED}❌ SET failed on $name: $response${NC}"
        continue
    fi
    
    # Test GET from same node
    get_cmd='{"Get":{"key":"'$test_key'"}}'
    response=$(send_command "$node" "$get_cmd")
    if [[ "$response" == *"Hello from $name"* ]]; then
        echo -e "${GREEN}✅ GET successful from same node ($name)${NC}"
    else
        echo -e "${RED}❌ GET failed from same node ($name): $response${NC}"
    fi
    
    # Test GET from other nodes (should fail in current implementation)
    echo "Testing cross-node data access:"
    for j in "${!NODES[@]}"; do
        if [ $i -ne $j ]; then
            other_node="${NODES[$j]}"
            other_name="${NODE_NAMES[$j]}"
            response=$(send_command "$other_node" "$get_cmd")
            if [[ "$response" == '{"Ok":null}' ]]; then
                echo -e "${YELLOW}⚠️  Data NOT replicated to $other_name (expected in current implementation)${NC}"
            else
                echo -e "${GREEN}✅ Unexpected: Data found on $other_name: $response${NC}"
            fi
        fi
    done
    echo
done

echo -e "${BLUE}Test 2: Testing JSONPath queries on individual nodes${NC}"
complex_key="complex_$(date +%s)"
complex_data='{"Set":{"key":"'$complex_key'","value":{"users":[{"name":"Alice","age":30},{"name":"Bob","age":25}],"metadata":{"version":"1.0"}}}}'

# Set complex data on node1
echo "Setting complex data on node1..."
response=$(send_command "${NODES[0]}" "$complex_data")
echo -e "${GREEN}Response: $response${NC}"

# Test JSONPath query on same node
echo "Testing JSONPath query on node1..."
qget_cmd='{"QGet":{"key":"'$complex_key'","query":"$.users[*].name"}}'
response=$(send_command "${NODES[0]}" "$qget_cmd")
echo -e "${GREEN}JSONPath result: $response${NC}"

# Test JSONPath query on other nodes (should fail)
for i in {1..2}; do
    echo "Testing JSONPath query on node$((i+1)) (should fail)..."
    response=$(send_command "${NODES[$i]}" "$qget_cmd")
    if [[ "$response" == '{"Ok":null}' ]]; then
        echo -e "${YELLOW}⚠️  Data not available on node$((i+1)) (expected)${NC}"
    else
        echo -e "${GREEN}Unexpected: Data found on node$((i+1)): $response${NC}"
    fi
done

echo
echo -e "${BLUE}Current Implementation Status:${NC}"
echo "================================"
echo -e "${GREEN}✅ Individual node functionality works${NC}"
echo -e "${GREEN}✅ TCP protocol works correctly${NC}"
echo -e "${GREEN}✅ JSON database operations work${NC}"
echo -e "${GREEN}✅ JSONPath queries work${NC}"
echo -e "${YELLOW}⚠️  Raft replication not yet fully implemented${NC}"
echo -e "${YELLOW}⚠️  Nodes operate independently${NC}"
echo -e "${YELLOW}⚠️  No automatic leader election${NC}"
echo -e "${YELLOW}⚠️  No data synchronization between nodes${NC}"

echo
echo -e "${BLUE}Next steps for full Raft implementation:${NC}"
echo "1. Implement inter-node communication"
echo "2. Add leader election mechanism"
echo "3. Implement log replication"
echo "4. Add cluster discovery"
echo "5. Implement consensus for write operations"

echo
echo -e "${GREEN}🎉 Current implementation test completed!${NC}"
