#!/bin/bash
# BitQuan Testnet Bootstrap Node Setup

set -e

echo "🚀 Setting up BitQuan Testnet Bootstrap Node..."

# Configuration
NODE_NAME="bootstrap-node"
DATA_DIR="/opt/bitquan-testnet"
CONFIG_FILE="/etc/bitquan/testnet.toml"
SERVICE_FILE="/etc/systemd/system/bitquan-testnet.service"

# Create directories
echo "📁 Creating directories..."
sudo mkdir -p $DATA_DIR
sudo mkdir -p $(dirname $CONFIG_FILE)
sudo mkdir -p /var/log/bitquan

# Copy configuration
echo "⚙️ Installing configuration..."
sudo cp config/testnet.toml $CONFIG_FILE

# Create systemd service
echo "🔧 Creating systemd service..."
sudo tee $SERVICE_FILE > /dev/null <<EOF
[Unit]
Description=BitQuan Testnet Bootstrap Node
After=network.target

[Service]
Type=simple
User=bitquan
WorkingDirectory=$DATA_DIR
ExecStart=/usr/local/bin/bitquan-node run --config $CONFIG_FILE
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$DATA_DIR /var/log/bitquan

[Install]
WantedBy=multi-user.target
EOF

# Create bitquan user
echo "👤 Creating bitquan user..."
if ! id "bitquan" &>/dev/null; then
    sudo useradd -r -s /bin/false -d $DATA_DIR bitquan
fi
sudo chown -R bitquan:bitquan $DATA_DIR
sudo chown -R bitquan:bitquan /var/log/bitquan

# Install binary
echo "📦 Installing bitquan-node..."
sudo cp target/release/bitquan-node /usr/local/bin/
sudo chmod +x /usr/local/bin/bitquan-node

# Configure firewall
echo "🔥 Configuring firewall..."
if command -v ufw >/dev/null 2>&1; then
    sudo ufw allow 19444/tcp comment "BitQuan Testnet P2P"
    sudo ufw allow 19443/tcp comment "BitQuan Testnet RPC"
fi

# Enable and start service
echo "🚀 Starting bootstrap node..."
sudo systemctl daemon-reload
sudo systemctl enable bitquan-testnet
sudo systemctl start bitquan-testnet

# Check status
echo "📊 Checking service status..."
sleep 5
sudo systemctl status bitquan-testnet --no-pager

echo ""
echo "✅ Bootstrap node setup complete!"
echo ""
echo "📍 Node Information:"
echo "   P2P Port: 19444"
echo "   RPC Port: 19443"
echo "   Data Dir: $DATA_DIR"
echo "   Config: $CONFIG_FILE"
echo ""
echo "🔍 To monitor logs:"
echo "   sudo journalctl -u bitquan-testnet -f"
echo ""
echo "🌐 Add this node to bootstrap_nodes in testnet.toml:"
echo "   $(hostname -I | awk '{print $1}'):19444"
echo ""
echo "🔗 Test RPC:"
echo "   curl http://$(hostname -I | awk '{print $1}'):19443 -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"method\":\"getblockcount\",\"params\":[],\"id\":1}'"
