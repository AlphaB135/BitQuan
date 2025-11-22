#!/usr/bin/env bash
# Deploy BitQuan node to cluster nodes
# Usage: ./deploy-cluster.sh <nodes-file>

set -euo pipefail

NODES_FILE="${1:-}"
BINARY="dist/bitquan-node"
REMOTE_DIR="${REMOTE_DIR:-/opt/bitquan}"
DEPLOY_USER="${DEPLOY_USER:-bitquan}"
SERVICE_NAME="bitquan-node"

if [[ -z "$NODES_FILE" ]]; then
    echo "Usage: $0 <nodes-file>"
    echo "Example: $0 deploy/configs/cluster-nodes-testnet.txt"
    exit 1
fi

if [[ ! -f "$NODES_FILE" ]]; then
    echo "Error: Nodes file not found: $NODES_FILE"
    exit 1
fi

if [[ ! -f "$BINARY" ]]; then
    echo "Error: Binary not found: $BINARY"
    exit 1
fi

echo "📦 Deploying BitQuan node to cluster..."
echo "Nodes file: $NODES_FILE"
echo "Binary: $BINARY"
echo "Remote dir: $REMOTE_DIR"
echo ""

# Read nodes from file (format: user@host or host)
NODES=$(grep -v '^#' "$NODES_FILE" | grep -v '^$' || true)

if [[ -z "$NODES" ]]; then
    echo "Error: No nodes found in $NODES_FILE"
    exit 1
fi

TOTAL=$(echo "$NODES" | wc -l)
echo "Deploying to $TOTAL nodes..."
echo ""

SUCCESS=0
FAILED=0

while IFS= read -r node; do
    [[ -z "$node" ]] && continue
    [[ "$node" =~ ^# ]] && continue

    echo "→ Deploying to $node..."

    # Add user if not specified
    if [[ ! "$node" =~ @ ]]; then
        node="${DEPLOY_USER}@${node}"
    fi

    # Deploy steps
    if ssh -o StrictHostKeyChecking=no "$node" "mkdir -p $REMOTE_DIR/bin $REMOTE_DIR/backups" && \
       ssh "$node" "test -f $REMOTE_DIR/bin/bitquan-node && cp $REMOTE_DIR/bin/bitquan-node $REMOTE_DIR/backups/bitquan-node.backup.$(date +%s) || true" && \
       scp "$BINARY" "$node:$REMOTE_DIR/bin/bitquan-node" && \
       ssh "$node" "chmod +x $REMOTE_DIR/bin/bitquan-node" && \
       ssh "$node" "sudo systemctl restart $SERVICE_NAME || true"; then
        echo "  ✅ Deployed successfully to $node"
        ((SUCCESS++))
    else
        echo "  ❌ Failed to deploy to $node"
        ((FAILED++))
    fi
    echo ""
done <<< "$NODES"

echo ""
echo "📊 Deployment Summary:"
echo "  Success: $SUCCESS/$TOTAL"
echo "  Failed:  $FAILED/$TOTAL"
echo ""

if [[ $FAILED -gt 0 ]]; then
    echo "⚠️  Some deployments failed. Check logs above."
    exit 1
fi

echo "✅ All deployments completed successfully!"
