#!/bin/bash
echo "=== Service Status ==="
sudo systemctl status bitquan-testnet.service --no-pager
echo ""
echo "=== Last 20 Log Lines ==="
tail -n 20 /home/ubuntu/bitquan-audit/logs/testnet.log
echo ""
echo "=== RPC Health Check ==="
curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"net_version","params":[],"id":1}' http://127.0.0.1:19443 || echo "RPC offline"
echo ""
echo "=== Peer Count ==="
curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"net_peerCount","params":[],"id":1}' http://127.0.0.1:19443 || echo "RPC offline"
echo ""
