#!/usr/bin/env bash
set -euo pipefail

# BitQuan Production Deployment Script
# This script handles the complete deployment pipeline for BitQuan nodes

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Configuration
ENVIRONMENT="${1:-staging}"
REGION="${2:-us-west-2}"
VERSION="${3:-latest}"
FORCE_DEPLOY="${4:-false}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Pre-deployment checks
pre_deployment_checks() {
    log_info "Running pre-deployment checks..."

    # Check if required tools are installed
    local required_tools=("kubectl" "helm" "terraform" "docker")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "$tool is not installed"
            exit 1
        fi
    done

    # Check if we're connected to the right cluster
    local current_context=$(kubectl config current-context)
    log_info "Current Kubernetes context: $current_context"

    # Validate environment
    if [[ ! "$ENVIRONMENT" =~ ^(staging|testnet|mainnet)$ ]]; then
        log_error "Invalid environment: $ENVIRONMENT"
        log_info "Valid environments: staging, testnet, mainnet"
        exit 1
    fi

    # Check if namespace exists
    if ! kubectl get namespace "bitquan-$ENVIRONMENT" &> /dev/null; then
        log_warning "Namespace bitquan-$ENVIRONMENT does not exist"
        log_info "Creating namespace..."
        kubectl create namespace "bitquan-$ENVIRONMENT"
    fi

    log_success "Pre-deployment checks passed"
}

# Build and push Docker image
build_and_push() {
    log_info "Building Docker image..."

    cd "$PROJECT_ROOT"

    # Build image
    docker build -t "ghcr.io/alphab/bitquan:$VERSION" .

    # Push image
    log_info "Pushing Docker image..."
    docker push "ghcr.io/alphab/bitquan:$VERSION"

    # Also tag as latest if this is the main branch
    if [[ "$VERSION" != "latest" ]]; then
        docker tag "ghcr.io/alphab/bitquan:$VERSION" "ghcr.io/alphab/bitquan:latest"
        docker push "ghcr.io/alphab/bitquan:latest"
    fi

    log_success "Docker image built and pushed"
}

# Deploy infrastructure with Terraform
deploy_infrastructure() {
    log_info "Deploying infrastructure with Terraform..."

    cd "$PROJECT_ROOT/deploy/terraform"

    # Initialize Terraform
    terraform init -input=false

    # Plan deployment
    terraform plan -out=tfplan -input=false \
        -var="environment=$ENVIRONMENT" \
        -var="aws_region=$REGION"

    # Apply changes
    if [[ "$FORCE_DEPLOY" == "true" ]]; then
        terraform apply -auto-approve -input=false tfplan
    else
        terraform apply -input=false tfplan
    fi

    log_success "Infrastructure deployed"
}

# Deploy Kubernetes manifests
deploy_kubernetes() {
    log_info "Deploying Kubernetes manifests..."

    cd "$PROJECT_ROOT/deploy/kubernetes"

    # Apply namespaces and RBAC
    kubectl apply -f namespaces.yaml

    # Apply ConfigMaps
    envsubst < configmaps.yaml | kubectl apply -f -

    # Update deployment with new image version
    sed "s|ghcr.io/alphab/bitquan:latest|ghcr.io/alphab/bitquan:$VERSION|g" deployments.yaml | \
    kubectl apply -f -

    # Apply services and PVCs
    kubectl apply -f services.yaml

    log_success "Kubernetes manifests deployed"
}

# Wait for deployment to be ready
wait_for_deployment() {
    log_info "Waiting for deployment to be ready..."

    local namespace="bitquan-$ENVIRONMENT"
    local timeout=600
    local interval=10

    local elapsed=0
    while [ $elapsed -lt $timeout ]; do
        local ready=$(kubectl get deployment bitquan-node -n "$namespace" -o jsonpath='{.status.readyReplicas}' 2>/dev/null || echo "0")
        local desired=$(kubectl get deployment bitquan-node -n "$namespace" -o jsonpath='{.spec.replicas}' 2>/dev/null || echo "0")

        if [[ "$ready" == "$desired" && "$ready" != "0" ]]; then
            log_success "Deployment is ready ($ready/$desired replicas)"
            return 0
        fi

        log_info "Waiting for deployment... ($ready/$desired replicas ready)"
        sleep $interval
        elapsed=$((elapsed + interval))
    done

    log_error "Deployment timed out after ${timeout}s"
    return 1
}

# Health checks
health_checks() {
    log_info "Running health checks..."

    local namespace="bitquan-$ENVIRONMENT"
    local service_name="bitquan-node"

    # Get service URL
    local service_url
    if [[ "$ENVIRONMENT" == "staging" ]]; then
        service_url="http://localhost:8080"
    else
        # For LoadBalancer services, get the external IP
        local external_ip=$(kubectl get service "$service_name" -n "$namespace" -o jsonpath='{.status.loadBalancer.ingress[0].hostname}' 2>/dev/null || echo "")
        if [[ -z "$external_ip" ]]; then
            external_ip=$(kubectl get service "$service_name" -n "$namespace" -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || echo "")
        fi
        service_url="http://$external_ip"
    fi

    # Wait a bit for the service to be fully ready
    sleep 30

    # Test health endpoint
    local max_attempts=10
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if curl -f "$service_url/health" &> /dev/null; then
            log_success "Health check passed"
            break
        fi

        log_warning "Health check attempt $attempt/$max_attempts failed"
        sleep 10
        attempt=$((attempt + 1))
    done

    if [ $attempt -gt $max_attempts ]; then
        log_error "Health checks failed"
        return 1
    fi

    # Test metrics endpoint
    if curl -f "$service_url/metrics" &> /dev/null; then
        log_success "Metrics endpoint is accessible"
    else
        log_warning "Metrics endpoint is not accessible"
    fi
}

# Post-deployment verification
post_deployment_verification() {
    log_info "Running post-deployment verification..."

    local namespace="bitquan-$ENVIRONMENT"

    # Check pod status
    log_info "Checking pod status..."
    kubectl get pods -n "$namespace" -l app=bitquan-node

    # Check service status
    log_info "Checking service status..."
    kubectl get services -n "$namespace"

    # Check HPA status
    log_info "Checking HPA status..."
    kubectl get hpa -n "$namespace" || log_warning "HPA not found"

    # Show recent logs
    log_info "Recent logs from pods..."
    kubectl logs -n "$namespace" -l app=bitquan-node --tail=20 || true

    log_success "Post-deployment verification completed"
}

# Rollback function
rollback() {
    log_warning "Initiating rollback..."

    local namespace="bitquan-$ENVIRONMENT"

    # Rollback deployment
    kubectl rollout undo deployment/bitquan-node -n "$namespace"

    # Wait for rollback to complete
    kubectl rollout status deployment/bitquan-node -n "$namespace" --timeout=300s

    log_success "Rollback completed"
}

# Main deployment function
main() {
    log_info "Starting BitQuan deployment to $ENVIRONMENT"
    log_info "Region: $REGION"
    log_info "Version: $VERSION"
    log_info "Force deploy: $FORCE_DEPLOY"

    # Trap to handle errors and cleanup
    trap 'log_error "Deployment failed. Check logs for details."' ERR

    # Run deployment pipeline
    pre_deployment_checks
    build_and_push
    deploy_infrastructure
    deploy_kubernetes
    wait_for_deployment
    health_checks
    post_deployment_verification

    log_success "🎉 BitQuan deployment to $ENVIRONMENT completed successfully! 🎉"

    # Show access information
    if [[ "$ENVIRONMENT" != "staging" ]]; then
        local namespace="bitquan-$ENVIRONMENT"
        local external_ip=$(kubectl get service bitquan-node -n "$namespace" -o jsonpath='{.status.loadBalancer.ingress[0].hostname}' 2>/dev/null || echo "")
        if [[ -z "$external_ip" ]]; then
            external_ip=$(kubectl get service bitquan-node -n "$namespace" -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || echo "")
        fi

        echo ""
        log_info "Access Information:"
        echo "  HTTP API: http://$external_ip"
        echo "  Metrics: http://$external_ip/metrics"
        echo "  Health: http://$external_ip/health"
        echo "  Stratum: $external_ip:3333"
    fi
}

# Handle script arguments
case "${1:-}" in
    "rollback")
        rollback
        ;;
    "health")
        health_checks
        ;;
    *)
        main
        ;;
esac
