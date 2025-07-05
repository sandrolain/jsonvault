#!/bin/bash

# JsonVault Quick Start Script
# This script helps you get started with JsonVault quickly

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

print_banner() {
    echo -e "${BLUE}"
    echo "╔══════════════════════════════════════════╗"
    echo "║              JsonVault 🔐                ║"
    echo "║    High-Performance JSON Database        ║"
    echo "║         Quick Start Guide                ║"
    echo "╚══════════════════════════════════════════╝"
    echo -e "${NC}"
}

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${MAGENTA}[STEP]${NC} $1"
}

show_help() {
    echo "Usage: $0 [OPTION]"
    echo
    echo "Options:"
    echo "  build         Build JsonVault from source"
    echo "  docker        Start JsonVault with Docker"
    echo "  k8s           Deploy to Kubernetes"
    echo "  demo          Run a quick demo"
    echo "  dev           Set up development environment"
    echo "  help          Show this help"
    echo
    echo "Examples:"
    echo "  $0 build      # Build and run locally"
    echo "  $0 docker     # Start with Docker Compose"
    echo "  $0 k8s        # Deploy to Kubernetes"
    echo "  $0 demo       # Interactive demo"
}

check_prerequisites() {
    log_step "Checking prerequisites..."
    
    local missing=0
    
    if ! command -v cargo &> /dev/null; then
        log_error "Rust/Cargo not found. Please install from https://rustup.rs/"
        missing=1
    fi
    
    if ! command -v task &> /dev/null; then
        log_warn "Task not found. Install with: brew install go-task/tap/go-task"
        log_warn "Falling back to cargo commands..."
    fi
    
    case "${1:-build}" in
        docker)
            if ! command -v docker &> /dev/null; then
                log_error "Docker not found. Please install Docker"
                missing=1
            fi
            if ! command -v docker-compose &> /dev/null; then
                log_error "Docker Compose not found"
                missing=1
            fi
            ;;
        k8s)
            if ! command -v kubectl &> /dev/null; then
                log_error "kubectl not found. Please install kubectl"
                missing=1
            fi
            ;;
    esac
    
    if [ $missing -eq 1 ]; then
        exit 1
    fi
    
    log_info "Prerequisites check passed ✓"
}

build_and_run() {
    log_step "Building JsonVault from source..."
    
    if command -v task &> /dev/null; then
        task ci
    else
        cargo build --release
        cargo test
    fi
    
    log_info "Build completed successfully ✓"
    
    log_step "Starting JsonVault server..."
    echo "Starting server on http://localhost:8080"
    echo "Press Ctrl+C to stop"
    echo
    
    cargo run --bin server -- --port 8080 &
    SERVER_PID=$!
    
    sleep 2
    
    log_info "Testing server connectivity..."
    if curl -s -f http://localhost:8080/health > /dev/null; then
        log_info "Server is running and healthy ✓"
        echo
        echo "🎉 JsonVault is ready!"
        echo "   API: http://localhost:8080"
        echo "   Try: curl http://localhost:8080/health"
    else
        log_error "Server health check failed"
        kill $SERVER_PID 2>/dev/null || true
        exit 1
    fi
    
    # Wait for interrupt
    trap "kill $SERVER_PID; exit 0" INT
    wait $SERVER_PID
}

start_docker() {
    log_step "Starting JsonVault cluster with Docker..."
    
    check_prerequisites docker
    
    if command -v task &> /dev/null; then
        task docker-start
    else
        ./docker-cluster.sh start
    fi
    
    log_info "Docker cluster started ✓"
    echo
    echo "🎉 JsonVault cluster is ready!"
    echo "   Load Balancer: http://localhost:8000"
    echo "   Node 1: http://localhost:8080"
    echo "   Node 2: http://localhost:8081" 
    echo "   Node 3: http://localhost:8082"
    echo "   Grafana: http://localhost:3000 (admin/admin)"
    echo "   Prometheus: http://localhost:9090"
    echo
    echo "Try: curl http://localhost:8000/health"
}

deploy_k8s() {
    log_step "Deploying JsonVault to Kubernetes..."
    
    check_prerequisites k8s
    
    if command -v task &> /dev/null; then
        task k8s-deploy
    else
        cd kubernetes
        ./setup.sh deploy
        cd ..
    fi
    
    log_info "Kubernetes deployment completed ✓"
    echo
    echo "🎉 JsonVault cluster deployed to Kubernetes!"
    echo "   Check status: kubectl get pods -n jsonvault"
    echo "   Port forward: kubectl port-forward -n jsonvault svc/jsonvault-service 8080:8080"
}

run_demo() {
    log_step "Running JsonVault interactive demo..."
    
    # Check if server is running
    if ! curl -s -f http://localhost:8080/health > /dev/null; then
        log_info "Starting demo server..."
        cargo run --bin server -- --port 8080 &
        SERVER_PID=$!
        sleep 3
    fi
    
    echo
    echo "🚀 JsonVault Demo"
    echo "=================="
    echo
    
    # Demo commands
    echo "1. Setting a user record..."
    curl -s -X POST http://localhost:8080/api/set \
         -H "Content-Type: application/json" \
         -d '{"key": "user:1", "value": {"name": "Alice", "age": 30, "city": "New York"}}' \
         | python3 -m json.tool 2>/dev/null || echo "Set OK"
    
    echo
    echo "2. Getting the user..."
    curl -s http://localhost:8080/api/get/user:1 | python3 -m json.tool 2>/dev/null || echo "Retrieved user"
    
    echo
    echo "3. JSONPath query for name..."
    curl -s "http://localhost:8080/api/qget/user:1?query=\$.name" | python3 -m json.tool 2>/dev/null || echo "Got name"
    
    echo
    echo "4. Setting nested property..."
    curl -s -X POST http://localhost:8080/api/qset \
         -H "Content-Type: application/json" \
         -d '{"key": "user:1", "path": "profile.status", "value": "active"}' \
         | python3 -m json.tool 2>/dev/null || echo "Set nested OK"
    
    echo
    echo "5. Final result..."
    curl -s http://localhost:8080/api/get/user:1 | python3 -m json.tool 2>/dev/null || echo "Final result"
    
    echo
    echo "✨ Demo completed!"
    echo "   Explore more at: http://localhost:8080"
    
    if [ -n "${SERVER_PID:-}" ]; then
        echo "   Stopping demo server..."
        kill $SERVER_PID 2>/dev/null || true
    fi
}

setup_dev() {
    log_step "Setting up development environment..."
    
    check_prerequisites
    
    log_info "Installing development tools..."
    if command -v task &> /dev/null; then
        task install-tools
    else
        cargo install cargo-audit cargo-outdated
    fi
    
    log_info "Running development cycle..."
    if command -v task &> /dev/null; then
        task dev
    else
        cargo fmt
        cargo clippy --all-targets --all-features
        cargo test
    fi
    
    log_info "Development environment ready ✓"
    echo
    echo "🛠️  Development commands:"
    echo "   task dev      # Quick development cycle"
    echo "   task build    # Build project"
    echo "   task test     # Run tests"
    echo "   task --list   # Show all tasks"
}

main() {
    local command=${1:-build}
    
    print_banner
    
    case $command in
        build)
            build_and_run
            ;;
        docker)
            start_docker
            ;;
        k8s)
            deploy_k8s
            ;;
        demo)
            run_demo
            ;;
        dev)
            setup_dev
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            log_error "Unknown command: $command"
            show_help
            exit 1
            ;;
    esac
}

# Check if script is being sourced or executed
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
