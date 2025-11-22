#!/bin/bash
# BitQuan Testnet Faucet Deployment

set -e

echo "🚰 Deploying BitQuan Testnet Faucet..."

# Configuration
FAUCET_DIR="/opt/bitquan-faucet"
SERVICE_FILE="/etc/systemd/system/bitquan-faucet.service"
VENV_DIR="$FAUCET_DIR/venv"

# Create directories
echo "📁 Creating directories..."
sudo mkdir -p $FAUCET_DIR
sudo mkdir -p /var/log/bitquan

# Install Python dependencies
echo "🐍 Installing Python dependencies..."
sudo apt-get update
sudo apt-get install -y python3 python3-pip python3-venv

# Create virtual environment
echo "🐍 Creating virtual environment..."
sudo python3 -m venv $VENV_DIR
sudo $VENV_DIR/bin/pip install -r tools/requirements-faucet.txt

# Copy faucet files
echo "📁 Installing faucet files..."
sudo cp tools/testnet_faucet.py $FAUCET_DIR/
sudo chown -R bitquan:bitquan $FAUCET_DIR 2>/dev/null || sudo chown -R $USER:$USER $FAUCET_DIR

# Create systemd service
echo "🔧 Creating systemd service..."
sudo tee $SERVICE_FILE > /dev/null <<EOF
[Unit]
Description=BitQuan Testnet Faucet
After=network.target

[Service]
Type=simple
User=bitquan
WorkingDirectory=$FAUCET_DIR
Environment="PATH=$VENV_DIR/bin"
ExecStart=$VENV_DIR/bin/python testnet_faucet.py
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$FAUCET_DIR /var/log/bitquan

[Install]
WantedBy=multi-user.target
EOF

# Create bitquan user if not exists
echo "👤 Creating bitquan user..."
if ! id "bitquan" &>/dev/null; then
    sudo useradd -r -s /bin/false -d $FAUCET_DIR bitquan
fi
sudo chown -R bitquan:bitquan $FAUCET_DIR
sudo chown -R bitquan:bitquan /var/log/bitquan

# Configure firewall
echo "🔥 Configuring firewall..."
if command -v ufw >/dev/null 2>&1; then
    sudo ufw allow 8080/tcp comment "BitQuan Testnet Faucet"
fi

# Enable and start service
echo "🚀 Starting faucet service..."
sudo systemctl daemon-reload
sudo systemctl enable bitquan-faucet
sudo systemctl start bitquan-faucet

# Check status
echo "📊 Checking service status..."
sleep 3
sudo systemctl status bitquan-faucet --no-pager

echo ""
echo "✅ Faucet deployment complete!"
echo ""
echo "🌐 Faucet URL:"
echo "   http://$(hostname -I | awk '{print $1}'):8080"
echo ""
echo "🔍 To monitor logs:"
echo "   sudo journalctl -u bitquan-faucet -f"
echo ""
echo "📊 Faucet stats:"
echo "   curl http://$(hostname -I | awk '{print $1}'):8080/stats"
echo ""
echo "🔗 Health check:"
echo "   curl http://$(hostname -I | awk '{print $1}'):8080/health"
