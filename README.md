# JsonVault 🔐

A high-performance, in-memory JSON database with JSONPath support, Raft consensus, and horizontal replication capabilities.

## Features

- **In-memory Key-Value Database**: Fast storage using `DashMap` for optimal concurrency
- **Native JSON Support**: All values are in interoperable JSON format with validation
- **JSONPath Queries**: Support for JSON queries and manipulations using JSONPath syntax
- **JSON Merge**: Capability to perform intelligent merges of JSON values
- **Lightweight TCP Protocol**: High-performance communication via TCP with JSON serialization
- **Raft Consensus**: Distributed consensus algorithm for automatic failover and data consistency
- **Horizontal Clustering**: Support for multi-node clusters with automatic failover
- **High-Performance APIs**: Operations optimized for minimal latency
- **Multi-language Clients**: Native support for Rust and Go clients

## Installation

### Prerequisites

- Rust 1.70 or higher
- Cargo
- Go 1.21+ (for Go client)

### Building

```bash
# Clone the repository
git clone <repository-url>
cd rust-json-db

# Build the project
cargo build --release
```

## Usage

### Starting the Server

#### With Raft Consensus (Recommended)

```bash
# Start a single-node cluster with Raft
cargo run --bin server -- --enable-raft --address 127.0.0.1:8080 --node-id "node1"

# Start a multi-node cluster (future support)
cargo run --bin server -- --enable-raft --address 127.0.0.1:8080 --node-id "1" --cluster-nodes "2,3"
cargo run --bin server -- --enable-raft --address 127.0.0.1:8081 --node-id "2" --cluster-nodes "1,3"
cargo run --bin server -- --enable-raft --address 127.0.0.1:8082 --node-id "3" --cluster-nodes "1,2"
```

#### Basic Server

```bash
# Start a single-node server (simple mode)
cargo run --bin server -- --address 127.0.0.1:8080
```

### Using the Client

#### Interactive Mode

```bash
# Connect to the server in interactive mode
cargo run --bin client -- --server 127.0.0.1:8080 interactive
```

#### Single Commands

```bash
# Set a value
cargo run --bin client -- --server 127.0.0.1:8080 set user '{"name": "Mario", "age": 30}'

# Get a value
cargo run --bin client -- --server 127.0.0.1:8080 get user

# Delete a value
cargo run --bin client -- --server 127.0.0.1:8080 delete user

# JSONPath Query (qget)
cargo run --bin client -- --server 127.0.0.1:8080 qget user '$.name'

# Set sub-property with JSONPath (qset)
cargo run --bin client -- --server 127.0.0.1:8080 qset user 'profession' '"Developer"'

# JSON Merge
cargo run --bin client -- --server 127.0.0.1:8080 merge user '{"city": "Roma", "age": 31}'

# Ping
cargo run --bin client -- --server 127.0.0.1:8080 ping
```

### Go Client

The project includes a complete Go client library:

```bash
cd go-client
go mod tidy

# Run the example
go run example/main.go
```

See `go-client/README.md` for detailed Go client documentation.

## API and Protocol

### Supported Commands

1. **SET** - Set a value for a key

   ```
   SET key json_value
   ```

2. **GET** - Read a value for a key

   ```
   GET key
   ```

3. **DELETE** - Delete a value for a key

   ```
   DELETE key
   ```

4. **QGET** - Execute a JSONPath query on a value

   ```
   QGET key jsonpath_query
   ```

5. **QSET** - Set a sub-property using JSONPath

   ```
   QSET key jsonpath_path json_value
   ```

6. **MERGE** - Merge a JSON value with existing data

   ```
   MERGE key json_value
   ```

7. **PING** - Server health check

   ```
   PING
   ```

### Communication Protocol

The protocol uses TCP with a lightweight format:

- Header: 4 bytes (payload length in big-endian)
- Payload: JSON-serialized data

### Usage Examples

#### Interactive Mode

```text
json-db> set config {"database": {"host": "localhost", "port": 5432}}
OK

json-db> get config
{
  "database": {
    "host": "localhost",
    "port": 5432
  }
}

json-db> qget config '$.database.host'
"localhost"

json-db> qset config 'database.timeout' '30'
OK

json-db> merge config {"database": {"ssl": true}, "cache": {"enabled": true}}
OK

json-db> get config
{
  "database": {
    "host": "localhost",
    "port": 5432,
    "ssl": true,
    "timeout": 30
  },
  "cache": {
    "enabled": true
  }
}
```

## Raft Clustering

### Architecture

The system uses Raft consensus algorithm for distributed clustering:

- **Leader Node**: Handles all write operations and replicates them to followers
- **Follower Nodes**: Receive replicated log entries from the leader and participate in leader election
- **Automatic Failover**: When a leader fails, followers automatically elect a new leader
- **Consensus**: All operations require majority consensus for commitment

### Cluster Configuration

```bash
# 1. Start the first node (will become leader in single-node cluster)
cargo run --bin server -- --enable-raft --address 127.0.0.1:8080 --node-id 1

# 2. Start additional nodes (multi-node support planned)
# cargo run --bin server -- --enable-raft --address 127.0.0.1:8081 --node-id 2 --cluster-nodes "1,3"
# cargo run --bin server -- --enable-raft --address 127.0.0.1:8082 --node-id 3 --cluster-nodes "1,2"
```

### Automatic Failover

Raft provides automatic failover capabilities:

- **Leader Election**: Automatic election of new leaders when the current leader fails
- **Log Replication**: Consistent replication of operations across all nodes
- **Split-Brain Prevention**: Majority consensus prevents split-brain scenarios

## Performance

### Benchmarks

The project includes benchmarks to measure performance:

```bash
# Run benchmarks
cargo bench
```

### Optimizations

- Use of `DashMap` for lock-free concurrency
- JSON serialization for interoperability
- Reusable TCP connection pools
- Optimized JSON operations

## Testing

```bash
# Run all tests
cargo test

# Specific tests
cargo test database
cargo test network
cargo test raft

# Run the complete test script
./test.sh
```

## Monitoring and Logging

The server uses Rust's logging system. Configure log level:

```bash
# Complete debug
RUST_LOG=debug cargo run --bin server

# Errors only
RUST_LOG=error cargo run --bin server
```

## Current Limitations

1. **Persistence**: The database is completely in-memory (disk persistence planned)
2. **Multi-node clusters**: Currently supports single-node Raft clusters (multi-node implementation in progress)
3. **Authentication**: Not implemented (planned for future versions)
4. **Compression**: Not implemented for network protocol

## Roadmap

- [ ] Disk persistence with WAL (Write-Ahead Log)
- [x] Raft consensus algorithm for automatic failover (single-node complete)
- [ ] Multi-node Raft cluster support
- [ ] Authentication and authorization system
- [ ] Network protocol compression
- [ ] Web interface for monitoring
- [ ] Client libraries for different languages
- [ ] Geographically distributed clustering

## Contributing

Contributions are welcome! Please:

1. Fork the project
2. Create a feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

This project is under the MIT license - see the `LICENSE` file for details.

## Support

For support and questions:

- Open an issue on GitHub
- Check the documentation
- See examples in the `examples/` directory
