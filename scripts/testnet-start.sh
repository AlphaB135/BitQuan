#!/bin/bash
# testnet-start.sh — One-command BitQuan testnet launcher
# Usage: ./scripts/testnet-start.sh [--stop] [--status]
#
# Builds from source (if needed), starts node on testnet, shows status.

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

DATA_DIR="./data/testnet"
BINARY="./target/release/bitquan-node"
PID_FILE="/tmp/bitquan-testnet.pid"

start_node() {
    echo -e "${GREEN}BitQuan Testnet Launcher${NC}"
    echo ""

    # Build if binary doesn't exist
    if [ ! -f "$BINARY" ]; then
        echo -e "${YELLOW}Building BitQuan...${NC}"
        cargo build --release
        echo ""
    fi

    # Create data directory
    mkdir -p "$DATA_DIR"

    # Check if already running
    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        echo -e "${YELLOW}BitQuan testnet is already running (PID: $(cat "$PID_FILE"))${NC}"
        return 0
    fi

    # Start node in background
    echo -e "${GREEN}Starting BitQuan testnet node...${NC}"
    $BINARY \
        --network testnet \
        --datadir "$DATA_DIR" \
        --rpc \
        --mine \
        --threads 2 \
        > "$DATA_DIR/node.log" 2>&1 &

    echo $! > "$PID_FILE"
    sleep 3

    # Check if still running
    if kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        echo -e "${GREEN}Node started (PID: $(cat "$PID_FILE"))${NC}"
        echo ""
        show_status
    else
        echo -e "${RED}Node failed to start. Check $DATA_DIR/node.log${NC}"
        rm -f "$PID_FILE"
        return 1
    fi
}

stop_node() {
    if [ -f "$PID_FILE" ]; then
        PID=$(cat "$PID_FILE")
        if kill -0 "$PID" 2>/dev/null; then
            echo -e "${YELLOW}Stopping BitQuan testnet (PID: $PID)...${NC}"
            kill "$PID"
            rm -f "$PID_FILE"
            echo -e "${GREEN}Stopped.${NC}"
        else
            echo "Node not running (stale PID file)"
            rm -f "$PID_FILE"
        fi
    else
        echo "No PID file found. Node may not be running."
    fi
}

show_status() {
    echo "P2P Port:  19444"
    echo "RPC Port:  19443"
    echo "Data Dir:  $DATA_DIR"
    echo "Log File:  $DATA_DIR/node.log"
    echo ""

    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        echo -e "${GREEN}Status: RUNNING (PID: $(cat "$PID_FILE"))${NC}"

        # Try to get block count via RPC
        if command -v curl >/dev/null 2>&1; then
            BLOCKS=$(curl -sf -X POST http://localhost:19443 \
                -H "Content-Type: application/json" \
                -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}' \
                2>/dev/null | grep -o '"result":[0-9]*' | grep -o '[0-9]*' || echo "?")
            echo "Blocks:    $BLOCKS"
        fi
    else
        echo -e "${RED}Status: NOT RUNNING${NC}"
    fi
}

case "${1:-start}" in
    start)   start_node ;;
    stop)    stop_node ;;
    status)  show_status ;;
    restart) stop_node; sleep 2; start_node ;;
    *)       echo "Usage: $0 [--start|--stop|--status|--restart]"; exit 1 ;;
esac
