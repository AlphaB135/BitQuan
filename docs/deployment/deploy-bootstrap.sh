#!/bin/bash

# BitQuan Bootstrap Node Deployment Script
# Usage: ./deploy-bootstrap.sh <NODE_ID> <SERVER_IP> <REGION>

set -e

NODE_ID=$1
SERVER_IP=$2
REGION=$3

if [ -z "$NODE_ID" ] || [ -z "$SERVER_IP" ] || [ -z "$REGION" ]; then
    echo "Usage: $0 <NODE_ID> <SERVER_IP> <REGION>"
    echo "Example: $0 1 192.168.1.101 asia"
    exit 1
fi

echo "🚀 Deploying BitQuan Bootstrap Node $NODE_ID in $REGION..."

# Update system
echo "📦 Updating system packages..."
sudo apt update && sudo apt upgrade -y

# Install dependencies
echo "🔧 Installing dependencies..."
sudo apt install -y curl wget git build-essential pkg-config libssl-dev

# Install Rust
echo "🦀 Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Create bitquan user
echo "👤 Creating bitquan user..."
sudo useradd -m -s /bin/bash bitquan
sudo -u bitquan mkdir -p /home/bitquan/.bitquan

# Clone and build BitQuan
echo "🏗️ Building BitQuan..."
cd /home/bitquan
sudo -u bitquan git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan
sudo -u bitquan ~/.cargo/bin/cargo build --release

# Create mainnet config
echo "⚙️ Creating mainnet configuration..."
sudo -u bitquan mkdir -p /home/bitquan/.bitquan/mainnet

# Update config with server IP
sed "s/127.0.0.1/$SERVER_IP/g" config/mainnet.toml > /home/bitquan/.bitquan/mainnet/bitquan.toml

# Create systemd service
echo "🔧 Creating systemd service..."
sudo tee /etc/systemd/system/bitquan-node.service > /dev/null <<EOF
[Unit]
Description=BitQuan Bootstrap Node $NODE_ID
After=network.target

[Service]
Type=simple
User=bitquan
WorkingDirectory=/home/bitquan/BitQuan
ExecStart=/home/bitquan/BitQuan/target/release/bitquan-node run --config /home/bitquan/.bitquan/mainnet/bitquan.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/home/bitquan/.bitquan

[Install]
WantedBy=multi-user.target
EOF

# Setup firewall
echo "🔥 Setting up firewall..."
sudo ufw allow 22/tcp
sudo ufw allow 8333/tcp
sudo ufw allow 8333/udp
sudo ufw allow 8332/tcp
sudo ufw allow 8334/tcp
sudo ufw --force enable

# Create monitoring script
echo "📊 Setting up monitoring..."
sudo tee /usr/local/bin/bitquan-monitor.sh > /dev/null <<'EOF'
#!/bin/bash
NODE_ID=$1
LOG_FILE="/var/log/bitquan-node-$NODE_ID.log"

# Check if node is running
if ! systemctl is-active --quiet bitquan-node; then
    echo "$(date): Node $NODE_ID is down, restarting..." >> $LOG_FILE
    sudo systemctl restart bitquan-node
fi

# Check port connectivity
if ! nc -z localhost 8333; then
    echo "$(date): Port 8333 not responding on node $NODE_ID" >> $LOG_FILE
fi

# Log system stats
echo "$(date): CPU: $(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | cut -d'%' -f1)%, Memory: $(free | grep Mem | awk '{printf("%.1f%%"), $3/$2 * 100.0}')" >> $LOG_FILE
EOF

sudo chmod +x /usr/local/bin/bitquan-monitor.sh

# Setup cron job for monitoring
echo "⏰ Setting up monitoring cron job..."
(crontab -l 2>/dev/null; echo "*/5 * * * * /usr/local/bin/bitquan-monitor.sh $NODE_ID") | crontab -

# Start the node
echo "🚀 Starting BitQuan node..."
sudo systemctl daemon-reload
sudo systemctl enable bitquan-node
sudo systemctl start bitquan-node

# Wait and check status
sleep 10
if systemctl is-active --quiet bitquan-node; then
    echo "✅ BitQuan Bootstrap Node $NODE_ID is running successfully!"
    echo "📍 Node: $SERVER_IP:8333"
    echo "📊 Status: systemctl status bitquan-node"
    echo "📝 Logs: journalctl -u bitquan-node -f"
else
    echo "❌ Failed to start BitQuan node"
    echo "📝 Check logs: journalctl -u bitquan-node"
    exit 1
fi

# Show node info
echo ""
echo "🎯 Bootstrap Node Information:"
echo "   Node ID: $NODE_ID"
echo "   Region: $REGION"
echo "   IP Address: $SERVER_IP"
echo "   P2P Port: 8333"
echo "   RPC Port: 8332"
echo "   Stratum Port: 8334"
echo ""
echo "🔗 Add to DNS: seed$NODE_ID.bitquan.network -> $SERVER_IP"
echo ""
echo "📊 Monitor with: watch -n 5 'systemctl status bitquan-node'"
echo "📝 View logs: journalctl -u bitquan-node -f"