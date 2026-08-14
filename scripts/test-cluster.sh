#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$PROJECT_ROOT/docker-compose.cluster.yml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Command: Start cluster
cmd_start() {
    local nodes="${1:-3}"
    log_info "Starting $nodes-node BitQuan testnet cluster via Docker Compose..."
    docker compose -f "$COMPOSE_FILE" up -d
    
    log_info "Waiting for nodes to initialize..."
    sleep 5
    
    log_info "Cluster started. Endpoints:"
    log_info "  - node-seed:    http://localhost:19443 (P2P: 19444)"
    log_info "  - node-miner-1: http://localhost:19445 (P2P: 19446)"
    log_info "  - node-relay-2: http://localhost:19447 (P2P: 19448)"
    log_info "  - faucet:       http://localhost:5000"
}

# Command: Stop cluster
cmd_stop() {
    log_info "Stopping BitQuan testnet cluster..."
    docker compose -f "$COMPOSE_FILE" down
    log_info "Cluster stopped"
}

# Command: Status check
cmd_status() {
    log_info "Cluster status:"
    docker compose -f "$COMPOSE_FILE" ps
}

# Command: Logs
cmd_logs() {
    local node="${1:-all}"
    if [ "$node" = "all" ]; then
        docker compose -f "$COMPOSE_FILE" logs -f
    else
        docker compose -f "$COMPOSE_FILE" logs -f "$node"
    fi
}

# Main
case "${1:-help}" in
    start)
        cmd_start "${2:-3}"
        ;;
    stop)
        cmd_stop
        ;;
    status)
        cmd_status
        ;;
    logs)
        cmd_logs "${2:-all}"
        ;;
    help|*)
        echo "Usage: $0 <command> [options]"
        echo "Commands: start, stop, status, logs"
        ;;
esac
