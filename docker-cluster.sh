#!/bin/bash

# JsonVault Docker Cluster Management Script

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
COMPOSE_FILE="docker/docker-compose.yml"
PROJECT_NAME="jsonvault"

print_banner() {
    echo -e "${BLUE}"
    echo "╔══════════════════════════════════════╗"
    echo "║           JsonVault Docker           ║"
    echo "║         Cluster Manager              ║"
    echo "╚══════════════════════════════════════╝"
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

check_prerequisites() {
    log_info "Checking prerequisites..."
    
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed or not in PATH"
        exit 1
    fi
    
    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose is not installed or not in PATH"
        exit 1
    fi
    
    if ! docker info &> /dev/null; then
        log_error "Docker daemon is not running"
        exit 1
    fi
    
    log_info "Prerequisites check passed"
}

build_image() {
    log_info "Building JsonVault Docker image..."
    docker build -t jsonvault:latest .
}

start_cluster() {
    log_info "Starting JsonVault cluster..."
    docker-compose -f $COMPOSE_FILE -p $PROJECT_NAME up -d
    
    log_info "Waiting for services to be healthy..."
    sleep 10
    
    show_status
}

stop_cluster() {
    log_info "Stopping JsonVault cluster..."
    docker-compose -f $COMPOSE_FILE -p $PROJECT_NAME down
}

restart_cluster() {
    log_info "Restarting JsonVault cluster..."
    stop_cluster
    start_cluster
}

show_status() {
    log_info "Cluster status:"
    docker-compose -f $COMPOSE_FILE -p $PROJECT_NAME ps
    
    echo
    log_info "Service endpoints:"
    echo "  JsonVault Load Balancer: http://localhost:8000"
    echo "  JsonVault Node 1:        http://localhost:8080"
    echo "  JsonVault Node 2:        http://localhost:8081"
    echo "  JsonVault Node 3:        http://localhost:8082"
    echo "  HAProxy Stats:           http://localhost:8404/stats"
    echo "  Prometheus:              http://localhost:9090"
    echo "  Grafana:                 http://localhost:3000 (admin/admin)"
}

show_logs() {
    local service=${1:-}
    if [ -n "$service" ]; then
        log_info "Showing logs for $service..."
        docker-compose -f $COMPOSE_FILE -p $PROJECT_NAME logs -f $service
    else
        log_info "Showing logs for all services..."
        docker-compose -f $COMPOSE_FILE -p $PROJECT_NAME logs -f
    fi
}

cleanup() {
    log_warn "Cleaning up JsonVault cluster..."
    docker-compose -f $COMPOSE_FILE -p $PROJECT_NAME down -v --remove-orphans
    docker system prune -f
    log_info "Cleanup completed"
}

scale_service() {
    local service=$1
    local replicas=$2
    
    log_info "Scaling $service to $replicas replicas..."
    docker-compose -f $COMPOSE_FILE -p $PROJECT_NAME up -d --scale $service=$replicas
}

health_check() {
    log_info "Performing health checks..."
    
    # Check each JsonVault node
    for port in 8080 8081 8082; do
        if curl -s -f http://localhost:$port/health > /dev/null; then
            echo -e "${GREEN}✓${NC} JsonVault node on port $port is healthy"
        else
            echo -e "${RED}✗${NC} JsonVault node on port $port is unhealthy"
        fi
    done
    
    # Check load balancer
    if curl -s -f http://localhost:8000/health > /dev/null; then
        echo -e "${GREEN}✓${NC} Load balancer is healthy"
    else
        echo -e "${RED}✗${NC} Load balancer is unhealthy"
    fi
    
    # Check Prometheus
    if curl -s -f http://localhost:9090/-/healthy > /dev/null; then
        echo -e "${GREEN}✓${NC} Prometheus is healthy"
    else
        echo -e "${RED}✗${NC} Prometheus is unhealthy"
    fi
    
    # Check Grafana
    if curl -s -f http://localhost:3000/api/health > /dev/null; then
        echo -e "${GREEN}✓${NC} Grafana is healthy"
    else
        echo -e "${RED}✗${NC} Grafana is unhealthy"
    fi
}

show_help() {
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo
    echo "Commands:"
    echo "  build         Build JsonVault Docker image"
    echo "  start         Start the cluster"
    echo "  stop          Stop the cluster"
    echo "  restart       Restart the cluster"
    echo "  status        Show cluster status"
    echo "  logs [SVC]    Show logs (optionally for specific service)"
    echo "  health        Perform health checks"
    echo "  cleanup       Remove cluster and cleanup"
    echo "  scale SVC N   Scale service to N replicas"
    echo "  help          Show this help"
    echo
    echo "Services:"
    echo "  jsonvault-node1, jsonvault-node2, jsonvault-node3"
    echo "  loadbalancer, prometheus, grafana"
    echo
    echo "Examples:"
    echo "  $0 build && $0 start     # Build and start cluster"
    echo "  $0 logs jsonvault-node1  # Show logs for node 1"
    echo "  $0 scale jsonvault-node1 2  # Scale node 1 to 2 replicas"
}

main() {
    local command=${1:-help}
    
    case $command in
        build)
            check_prerequisites
            build_image
            ;;
        start)
            print_banner
            check_prerequisites
            build_image
            start_cluster
            log_info "JsonVault cluster started successfully!"
            ;;
        stop)
            stop_cluster
            ;;
        restart)
            restart_cluster
            ;;
        status)
            show_status
            ;;
        logs)
            show_logs ${2:-}
            ;;
        health)
            health_check
            ;;
        cleanup)
            cleanup
            ;;
        scale)
            if [ $# -lt 3 ]; then
                log_error "Usage: $0 scale <service> <replicas>"
                exit 1
            fi
            scale_service $2 $3
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
