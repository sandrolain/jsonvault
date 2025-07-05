#!/bin/bash

# JsonVault Kubernetes Setup Script
# This script deploys JsonVault cluster on Kubernetes

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
NAMESPACE=${JSONVAULT_NAMESPACE:-jsonvault}
REPLICAS=${JSONVAULT_REPLICAS:-3}
IMAGE_TAG=${JSONVAULT_IMAGE_TAG:-latest}
STORAGE_CLASS=${JSONVAULT_STORAGE_CLASS:-standard}
STORAGE_SIZE=${JSONVAULT_STORAGE_SIZE:-10Gi}

print_banner() {
    echo -e "${BLUE}"
    echo "╔══════════════════════════════════════╗"
    echo "║           JsonVault Setup            ║"
    echo "║      Kubernetes Deployment           ║"
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
    
    # Check if kubectl is installed
    if ! command -v kubectl &> /dev/null; then
        log_error "kubectl is not installed or not in PATH"
        exit 1
    fi
    
    # Check if we can connect to Kubernetes cluster
    if ! kubectl cluster-info &> /dev/null; then
        log_error "Cannot connect to Kubernetes cluster"
        exit 1
    fi
    
    # Check if we have permission to create namespaces
    if ! kubectl auth can-i create namespaces &> /dev/null; then
        log_warn "May not have permission to create namespaces"
    fi
    
    log_info "Prerequisites check passed"
}

create_namespace() {
    log_info "Creating namespace: $NAMESPACE"
    kubectl apply -f - <<EOF
apiVersion: v1
kind: Namespace
metadata:
  name: $NAMESPACE
  labels:
    name: $NAMESPACE
    app: jsonvault
EOF
}

deploy_rbac() {
    log_info "Deploying RBAC configuration..."
    kubectl apply -f rbac.yaml
}

deploy_configmap() {
    log_info "Creating ConfigMap..."
    kubectl apply -f - <<EOF
apiVersion: v1
kind: ConfigMap
metadata:
  name: jsonvault-config
  namespace: $NAMESPACE
data:
  RUST_LOG: "info"
  JSONVAULT_PORT: "8080"
  JSONVAULT_RAFT_PORT: "8090"
  JSONVAULT_DATA_DIR: "/data"
  JSONVAULT_STORAGE_CLASS: "$STORAGE_CLASS"
EOF
}

deploy_services() {
    log_info "Deploying services..."
    
    # Headless service for StatefulSet
    kubectl apply -f - <<EOF
apiVersion: v1
kind: Service
metadata:
  name: jsonvault-headless
  namespace: $NAMESPACE
  labels:
    app: jsonvault
spec:
  clusterIP: None
  selector:
    app: jsonvault
  ports:
  - name: api
    port: 8080
    targetPort: 8080
  - name: raft
    port: 8090
    targetPort: 8090
EOF

    # LoadBalancer service
    kubectl apply -f - <<EOF
apiVersion: v1
kind: Service
metadata:
  name: jsonvault-service
  namespace: $NAMESPACE
  labels:
    app: jsonvault
spec:
  type: LoadBalancer
  selector:
    app: jsonvault
  ports:
  - name: api
    port: 8080
    targetPort: 8080
    protocol: TCP
  - name: raft
    port: 8090
    targetPort: 8090
    protocol: TCP
EOF
}

deploy_statefulset() {
    log_info "Deploying StatefulSet with $REPLICAS replicas..."
    
    # Generate cluster nodes list
    CLUSTER_NODES=""
    for ((i=0; i<REPLICAS; i++)); do
        if [ $i -gt 0 ]; then
            CLUSTER_NODES+=","
        fi
        CLUSTER_NODES+="jsonvault-$i.jsonvault-headless.$NAMESPACE.svc.cluster.local:8090"
    done
    
    kubectl apply -f - <<EOF
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: jsonvault
  namespace: $NAMESPACE
  labels:
    app: jsonvault
spec:
  serviceName: jsonvault-headless
  replicas: $REPLICAS
  selector:
    matchLabels:
      app: jsonvault
  template:
    metadata:
      labels:
        app: jsonvault
    spec:
      serviceAccountName: jsonvault-sa
      automountServiceAccountToken: false
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        runAsGroup: 1000
        fsGroup: 1000
      containers:
      - name: jsonvault
        image: jsonvault:$IMAGE_TAG
        imagePullPolicy: Always
        ports:
        - containerPort: 8080
          name: api
        - containerPort: 8090
          name: raft
        env:
        - name: JSONVAULT_NODE_ID
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: JSONVAULT_CLUSTER_NODES
          value: "$CLUSTER_NODES"
        envFrom:
        - configMapRef:
            name: jsonvault-config
        volumeMounts:
        - name: data
          mountPath: /data
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
            ephemeral-storage: 1Gi
          limits:
            cpu: 500m
            memory: 512Mi
            ephemeral-storage: 2Gi
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 30
          timeoutSeconds: 10
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        startupProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 30
  volumeClaimTemplates:
  - metadata:
      name: data
      labels:
        app: jsonvault
    spec:
      accessModes: [ "ReadWriteOnce" ]
      storageClassName: "$STORAGE_CLASS"
      resources:
        requests:
          storage: $STORAGE_SIZE
EOF
}

deploy_hpa() {
    log_info "Deploying Horizontal Pod Autoscaler..."
    kubectl apply -f hpa.yaml
}

wait_for_pods() {
    log_info "Waiting for pods to be ready..."
    kubectl wait --for=condition=ready pod -l app=jsonvault -n $NAMESPACE --timeout=300s
}

show_status() {
    log_info "Deployment status:"
    echo
    kubectl get all -n $NAMESPACE
    echo
    
    log_info "Service endpoints:"
    kubectl get svc -n $NAMESPACE
    echo
    
    # Try to get LoadBalancer IP
    EXTERNAL_IP=$(kubectl get svc jsonvault-service -n $NAMESPACE -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || echo "pending")
    if [ "$EXTERNAL_IP" != "pending" ] && [ -n "$EXTERNAL_IP" ]; then
        log_info "JsonVault is available at: http://$EXTERNAL_IP:8080"
    else
        log_warn "LoadBalancer IP is pending. Use port-forward to access:"
        echo "kubectl port-forward -n $NAMESPACE svc/jsonvault-service 8080:8080"
    fi
}

cleanup() {
    log_warn "Cleaning up JsonVault deployment..."
    kubectl delete namespace $NAMESPACE --ignore-not-found=true
    log_info "Cleanup completed"
}

show_help() {
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo
    echo "Commands:"
    echo "  deploy    Deploy JsonVault cluster (default)"
    echo "  cleanup   Remove JsonVault cluster"
    echo "  status    Show cluster status"
    echo "  help      Show this help"
    echo
    echo "Environment Variables:"
    echo "  JSONVAULT_NAMESPACE      Kubernetes namespace (default: jsonvault)"
    echo "  JSONVAULT_REPLICAS       Number of replicas (default: 3)"
    echo "  JSONVAULT_IMAGE_TAG      Docker image tag (default: latest)"
    echo "  JSONVAULT_STORAGE_CLASS  Storage class (default: standard)"
    echo "  JSONVAULT_STORAGE_SIZE   Storage size per node (default: 10Gi)"
    echo
    echo "Examples:"
    echo "  $0 deploy                    # Deploy with defaults"
    echo "  JSONVAULT_REPLICAS=5 $0      # Deploy with 5 replicas"
    echo "  $0 cleanup                   # Remove deployment"
}

main() {
    local command=${1:-deploy}
    
    case $command in
        deploy)
            print_banner
            check_prerequisites
            create_namespace
            deploy_rbac
            deploy_configmap
            deploy_services
            deploy_statefulset
            deploy_hpa
            wait_for_pods
            show_status
            log_info "JsonVault deployment completed successfully!"
            ;;
        cleanup)
            cleanup
            ;;
        status)
            show_status
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
