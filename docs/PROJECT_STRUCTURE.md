# JsonVault - Project Structure

JsonVault is a high-performance, in-memory JSON database with Raft consensus for automatic failover and distributed consistency.

## 📁 Project Structure

```
jsonvault/
├── 📄 README.md                    # Main project documentation
├── 📄 Cargo.toml                   # Rust project configuration
├── 📄 Dockerfile                   # Docker image definition
├── 📄 Taskfile.yml                 # Task automation (recommended)
├── 📄 LICENSE                      # Project license
│
├── 📂 src/                         # Rust source code
│   ├── 📄 lib.rs                   # Library entry point
│   ├── 📄 server.rs                # TCP server implementation
│   ├── 📄 client.rs                # Client binary
│   ├── 📄 database.rs              # Core database logic
│   ├── 📄 raft.rs                  # Raft consensus implementation
│   ├── 📄 network.rs               # Network protocol
│   └── 📄 protocol.rs              # Command/response definitions
│
├── 📂 examples/                    # Usage examples
│   ├── 📄 basic_usage.rs           # Basic operations example
│   └── 📄 raft_demo.rs             # Raft consensus demonstration
│
├── 📂 benches/                     # Performance benchmarks
│   └── 📄 benchmarks.rs            # Comprehensive benchmarks
│
├── 📂 go-client/                   # Go client library
│   ├── 📄 client.go                # Go client implementation
│   ├── 📄 go.mod                   # Go module definition
│   ├── 📄 README.md                # Go client documentation
│   └── 📂 example/                 # Go usage examples
│       ├── 📄 main.go              # Complete example
│       └── 📄 simple.go            # Simple operations
│
├── 📂 docs/                        # Documentation
│   ├── 📄 RAFT.md                  # Raft consensus documentation
│   ├── 📄 RAFT_SUMMARY.md          # Raft implementation summary
│   └── 📄 INTEGRATION_COMPLETE.md  # Integration completion report
│
├── 📂 deployment/                  # Deployment configurations
│   ├── 📄 docker-compose-raft.yml  # Docker Compose for Raft cluster
│   ├── 📂 docker-cluster/          # Advanced Docker setup
│   │   ├── 📄 docker-compose.yml   # Multi-node cluster configuration
│   │   └── 📄 haproxy.cfg          # Load balancer configuration
│   └── 📂 kubernetes/              # Kubernetes manifests
│       ├── 📄 statefulset.yaml     # StatefulSet for Raft cluster
│       ├── 📄 hpa.yaml             # Horizontal Pod Autoscaler
│       ├── 📄 rbac.yaml            # RBAC configuration
│       └── 📄 setup.sh             # Kubernetes setup script
│
└── 📂 scripts/                     # Utility scripts
    ├── 📄 test.sh                  # Integration test script
    ├── 📄 test-raft.sh             # Raft-specific tests
    ├── 📄 benchmark-raft.sh        # Raft benchmarking
    ├── 📄 docker-cluster.sh        # Docker cluster management
    └── 📄 quickstart.sh            # Quick start script
```

## 🚀 Quick Start

### Using Task (Recommended)

```bash
# Install Task: https://taskfile.dev/installation/
# Then run:

# Show all available tasks
task

# Start a Raft server
task run-server-raft

# Run interactive client
task run-client

# Run Raft demonstration
task run-raft-demo

# Run all tests
task ci
```

### Manual Commands

```bash
# Build the project
cargo build --release

# Start Raft server
cargo run --bin server -- --enable-raft --address 127.0.0.1:8090 --node-id 1

# Run client
cargo run --bin client -- --server 127.0.0.1:8090 interactive

# Run tests
cargo test
```

## 🏗️ Architecture Overview

JsonVault uses **Raft consensus algorithm** for distributed coordination:

- **Database**: Core in-memory JSON storage with DashMap for concurrency
- **Raft Manager**: Handles leader election, log replication, and consensus
- **Network Layer**: TCP protocol with JSON serialization
- **Client Libraries**: Native Rust and Go clients

## 📊 Performance

- **35,000+ operations/second** in single-node configuration
- **Automatic failover** with sub-second leader election
- **Consistent replication** across cluster nodes
- **JSONPath query support** with consensus guarantees

## 🔧 Available Commands

### Task Commands

| Command | Description |
|---------|-------------|
| `task run-server-raft` | Start Raft server (recommended) |
| `task run-client` | Interactive client |
| `task run-raft-demo` | Raft demonstration |
| `task test` | Run unit tests |
| `task test-raft` | Raft-specific tests |
| `task bench-raft` | Raft benchmarks |
| `task docker-raft` | Docker Raft cluster |
| `task ci` | Full CI pipeline |

### Database Operations

| Operation | Description | Example |
|-----------|-------------|---------|
| `SET` | Store JSON value | `set user {"name": "Alice"}` |
| `GET` | Retrieve value | `get user` |
| `DELETE` | Remove key | `delete user` |
| `QGET` | JSONPath query | `qget user $.name` |
| `QSET` | Set nested property | `qset user $.age 30` |
| `MERGE` | Merge JSON objects | `merge user {"city": "NYC"}` |
| `PING` | Health check | `ping` |

## 🐳 Deployment

### Docker

```bash
# Single-node Raft cluster
task docker-raft

# Advanced multi-node cluster
docker-compose -f deployment/docker-cluster/docker-compose.yml up
```

### Kubernetes

```bash
# Deploy to Kubernetes
task k8s-deploy

# Check status
task k8s-status
```

## 📚 Further Reading

- **[RAFT.md](docs/RAFT.md)** - Detailed Raft implementation
- **[Go Client README](go-client/README.md)** - Go client usage
- **[Integration Report](docs/INTEGRATION_COMPLETE.md)** - Project completion status

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes following the coding standards
4. Run `task ci` to verify everything works
5. Submit a pull request

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
