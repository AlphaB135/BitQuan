#!/usr/bin/env bash
set -euo pipefail

# BitQuan Cluster Deployment Script
# Deploys node binaries to cluster nodes via SSH

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DIST_DIR="$PROJECT_ROOT/dist"
NODES_FILE="${1:-$SCRIPT_DIR/../configs/cluster-nodes.txt}"
DEPLOY_USER="${DEPLOY_USER:-bitquan}"
REMOTE_DIR="${REMOTE_DIR:-/opt/bitquan}"

if [ ! -f "$NODES_FILE" ]; then
    echo "❌ Cluster nodes file not found: $NODES_FILE"
    echo "Usage: $0 [nodes-file]"
    exit 1
fi

if [ ! -d "$DIST_DIR" ]; then
    echo "❌ Distribution directory not found. Run build-release.sh first."
    exit 1
fi

echo "🚀 Deploying BitQuan to cluster nodes..."
echo "Nodes file: $NODES_FILE"
echo "Deploy user: $DEPLOY_USER"

# Read nodes from file
mapfile -t NODES < "$NODES_FILE"

for NODE in "${NODES[@]}"; do
    # Skip comments and empty lines
    [[ "$NODE" =~ ^#.*$ || -z "$NODE" ]] && continue

    echo ""
    echo "Deploying to: $NODE"

    # Stop existing node
    ssh "$DEPLOY_USER@$NODE" "systemctl --user stop bitquan-node || true"

    # Backup existing binary
    ssh "$DEPLOY_USER@$NODE" "[ -f $REMOTE_DIR/bitquan-node ] && mv $REMOTE_DIR/bitquan-node $REMOTE_DIR/bitquan-node.bak || true"

    # Upload new binary
    scp "$DIST_DIR/bitquan-node" "$DEPLOY_USER@$NODE:$REMOTE_DIR/"

    # Set permissions
    ssh "$DEPLOY_USER@$NODE" "chmod +x $REMOTE_DIR/bitquan-node"

    # Restart node
    ssh "$DEPLOY_USER@$NODE" "systemctl --user start bitquan-node"

    # Check status
    sleep 2
    ssh "$DEPLOY_USER@$NODE" "systemctl --user status bitquan-node --no-pager" || true

    echo "✓ Deployed to $NODE"
done

echo ""
echo "✅ Cluster deployment complete!"
