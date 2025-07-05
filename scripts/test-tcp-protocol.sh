#!/bin/bash

# Simple JsonVault TCP Test Script
# Quick tests for TCP protocol

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

HOST="${1:-localhost}"
PORT="${2:-8080}"
NODE_ADDR="$HOST:$PORT"

echo -e "${BLUE}🔧 JsonVault TCP Protocol Test${NC}"
echo "==============================="
echo -e "${BLUE}Target: $NODE_ADDR${NC}"
echo

# Function to send TCP command
send_tcp_command() {
    local command_json="$1"
    local timeout="${2:-5}"
    
    echo -e "${BLUE}📤 Command: $command_json${NC}"
    
    python3 -c "
import socket
import struct
import sys
import json

host = '$HOST'
port = $PORT
command = '$command_json'
timeout = $timeout

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
"
}

echo -e "${YELLOW}Test 1: PING${NC}"
response=$(send_tcp_command '{"type":"Ping"}')
echo -e "${GREEN}📥 Response: $response${NC}"
echo

echo -e "${YELLOW}Test 2: SET key${NC}"
response=$(send_tcp_command '{"type":"Set","key":"test-key","value":{"message":"Hello World","timestamp":"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'"}}')
echo -e "${GREEN}📥 Response: $response${NC}"
echo

echo -e "${YELLOW}Test 3: GET key${NC}"
response=$(send_tcp_command '{"type":"Get","key":"test-key"}')
echo -e "${GREEN}📥 Response: $response${NC}"
echo

echo -e "${YELLOW}Test 4: JSONPath Query${NC}"
response=$(send_tcp_command '{"type":"QGet","key":"test-key","query":"$.message"}')
echo -e "${GREEN}📥 Response: $response${NC}"
echo

echo -e "${YELLOW}Test 5: MERGE operation${NC}"
response=$(send_tcp_command '{"type":"Merge","key":"test-key","value":{"status":"updated","counter":42}}')
echo -e "${GREEN}📥 Response: $response${NC}"
echo

echo -e "${YELLOW}Test 6: GET merged result${NC}"
response=$(send_tcp_command '{"type":"Get","key":"test-key"}')
echo -e "${GREEN}📥 Response: $response${NC}"
echo

echo -e "${YELLOW}Test 7: DELETE key${NC}"
response=$(send_tcp_command '{"type":"Delete","key":"test-key"}')
echo -e "${GREEN}📥 Response: $response${NC}"
echo

echo -e "${YELLOW}Test 8: GET deleted key${NC}"
response=$(send_tcp_command '{"type":"Get","key":"test-key"}')
echo -e "${GREEN}📥 Response: $response${NC}"
echo

echo -e "${GREEN}🎉 TCP Protocol tests completed!${NC}"
