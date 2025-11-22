#!/usr/bin/env bash
# BitQuan Testnet Quick Setup Script
# Usage: sudo bash setup-testnet.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}"
echo "╔═══════════════════════════════════════╗"
echo "║   BitQuan Testnet Setup v1.0.0       ║"
echo "╚═══════════════════════════════════════╝"
echo -e "${NC}"

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo -e "${RED}❌ This script must be run as root (use sudo)${NC}"
   exit 1
fi

# Variables
BITQUAN_USER="bitquan"
INSTALL_DIR="/opt/bitquan"
DATA_DIR="/opt/bitquan/data/testnet"
BIN_URL="https://github.com/AlphaB135/BitQuan/releases/download/v1.0.0/bitquan-linux-x86_64"
REPO_URL="https://github.com/AlphaB135/BitQuan.git"

# Detect OS
if [[ -f /etc/os-release ]]; then
    . /etc/os-release
    OS=$ID
else
    echo -e "${RED}❌ Cannot detect OS${NC}"
    exit 1
fi

echo -e "${YELLOW}📋 System Information:${NC}"
echo "  OS: $PRETTY_NAME"
echo "  User: $BITQUAN_USER"
echo "  Install Dir: $INSTALL_DIR"
echo ""

# Ask for confirmation
read -p "Continue with installation? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Installation cancelled."
    exit 0
fi

echo ""
echo -e "${GREEN}🚀 Starting installation...${NC}"
echo ""

# Update system
echo -e "${YELLOW}📦 Updating system...${NC}"
if [[ "$OS" == "ubuntu" ]] || [[ "$OS" == "debian" ]]; then
    apt update -qq
    apt install -y curl wget git build-essential pkg-config libssl-dev python3 python3-pip jq
elif [[ "$OS" == "centos" ]] || [[ "$OS" == "rhel" ]]; then
    yum install -y curl wget git gcc gcc-c++ make openssl-devel python3 python3-pip jq
fi

# Create user
echo -e "${YELLOW}👤 Creating user: $BITQUAN_USER...${NC}"
if ! id "$BITQUAN_USER" &>/dev/null; then
    useradd -m -s /bin/bash "$BITQUAN_USER"
    echo "✅ User created"
else
    echo "✅ User already exists"
fi

# Create directories
echo -e "${YELLOW}📁 Creating directories...${NC}"
mkdir -p "$INSTALL_DIR"/{bin,data,logs,backups,config}
mkdir -p "$DATA_DIR"
chown -R "$BITQUAN_USER":"$BITQUAN_USER" "$INSTALL_DIR"
echo "✅ Directories created"

# Download or build binary
echo -e "${YELLOW}⬇️  Getting BitQuan binary...${NC}"
if curl -fsSL "$BIN_URL" -o "$INSTALL_DIR/bin/bitquan-node" 2>/dev/null; then
    echo "✅ Downloaded from GitHub releases"
else
    echo "⚠️  Release not found, building from source..."

    # Install Rust
    if ! command -v cargo &> /dev/null; then
        echo "📦 Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi

    # Clone and build
    cd /tmp
    if [[ -d BitQuan ]]; then
        rm -rf BitQuan
    fi
    git clone "$REPO_URL" BitQuan
    cd BitQuan
    git checkout v1.0.0
    cargo build --release --bin bitquan-node
    cp target/release/bitquan-node "$INSTALL_DIR/bin/"
    cd /tmp
    rm -rf BitQuan
    echo "✅ Built from source"
fi

chmod +x "$INSTALL_DIR/bin/bitquan-node"
chown "$BITQUAN_USER":"$BITQUAN_USER" "$INSTALL_DIR/bin/bitquan-node"

# Generate JWT secret
echo -e "${YELLOW}🔐 Generating JWT secret...${NC}"
JWT_SECRET=$(openssl rand -hex 32)
echo "$JWT_SECRET" > "$INSTALL_DIR/config/jwt.secret"
chmod 600 "$INSTALL_DIR/config/jwt.secret"
chown "$BITQUAN_USER":"$BITQUAN_USER" "$INSTALL_DIR/config/jwt.secret"
echo "✅ JWT secret generated"

# Create wallet for mining
echo -e "${YELLOW}💰 Creating testnet wallet...${NC}"
sudo -u "$BITQUAN_USER" "$INSTALL_DIR/bin/bitquan-node" wallet create \
  --network testnet \
  --output "$INSTALL_DIR/config/pool-wallet.keystore" \
  --password "changeme-$(openssl rand -hex 16)" || true

# Get mining address
MINING_ADDRESS=$(sudo -u "$BITQUAN_USER" "$INSTALL_DIR/bin/bitquan-node" wallet address \
  --keystore "$INSTALL_DIR/config/pool-wallet.keystore" 2>/dev/null | tail -1 || echo "tBQ1_CHANGE_ME")
echo "✅ Mining address: $MINING_ADDRESS"

# Create config file
echo -e "${YELLOW}⚙️  Creating configuration...${NC}"
cat > "$INSTALL_DIR/config/testnet.toml" << EOF
# BitQuan Testnet Configuration

[network]
network_id = "testnet"
p2p_port = 8333
max_peers = 50

# Add bootstrap nodes here
bootstrap_nodes = []

[rpc]
enabled = true
bind = "0.0.0.0"
port = 8334
jwt_secret_file = "$INSTALL_DIR/config/jwt.secret"
max_connections = 100
require_auth = true

[mining]
enabled = false
address = "$MINING_ADDRESS"

[pool]
enabled = true
bind = "0.0.0.0"
stratum_port = 3333
difficulty = 1000
vardiff_min = 100
vardiff_max = 10000
dashboard_enabled = true
dashboard_port = 8080

[consensus]
min_difficulty_bits = 0x1d00ffff

[storage]
db_path = "$DATA_DIR/chaindata"
cache_size_mb = 512

[metrics]
enabled = true
bind = "127.0.0.1"
port = 9090
EOF

chown "$BITQUAN_USER":"$BITQUAN_USER" "$INSTALL_DIR/config/testnet.toml"
echo "✅ Configuration created"

# Create systemd service
echo -e "${YELLOW}🔧 Creating systemd service...${NC}"
cat > /etc/systemd/system/bitquan-testnet.service << EOF
[Unit]
Description=BitQuan Testnet Node
After=network.target

[Service]
Type=simple
User=$BITQUAN_USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/bin/bitquan-node \\
  --network testnet \\
  --data-dir $DATA_DIR \\
  --config $INSTALL_DIR/config/testnet.toml \\
  --rpc-port 8334 \\
  --p2p-port 8333 \\
  --enable-stratum \\
  --stratum-port 3333

Restart=always
RestartSec=10
StandardOutput=append:$INSTALL_DIR/logs/testnet.log
StandardError=append:$INSTALL_DIR/logs/testnet-error.log

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$INSTALL_DIR/data
ReadWritePaths=$INSTALL_DIR/logs

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
echo "✅ Systemd service created"

# Configure firewall
echo -e "${YELLOW}🔥 Configuring firewall...${NC}"
if command -v ufw &> /dev/null; then
    ufw allow 8333/tcp comment 'BitQuan P2P'
    ufw allow 8334/tcp comment 'BitQuan RPC'
    ufw allow 3333/tcp comment 'BitQuan Stratum'
    ufw allow 8080/tcp comment 'BitQuan Dashboard'
    echo "✅ UFW rules added"
elif command -v firewall-cmd &> /dev/null; then
    firewall-cmd --permanent --add-port=8333/tcp
    firewall-cmd --permanent --add-port=8334/tcp
    firewall-cmd --permanent --add-port=3333/tcp
    firewall-cmd --permanent --add-port=8080/tcp
    firewall-cmd --reload
    echo "✅ Firewalld rules added"
else
    echo "⚠️  No firewall detected, please configure manually"
fi

# Create helpful scripts
echo -e "${YELLOW}📝 Creating helper scripts...${NC}"

# Start script
cat > "$INSTALL_DIR/start.sh" << 'EOF'
#!/bin/bash
sudo systemctl start bitquan-testnet
sudo systemctl status bitquan-testnet
EOF
chmod +x "$INSTALL_DIR/start.sh"

# Stop script
cat > "$INSTALL_DIR/stop.sh" << 'EOF'
#!/bin/bash
sudo systemctl stop bitquan-testnet
EOF
chmod +x "$INSTALL_DIR/stop.sh"

# Status script
cat > "$INSTALL_DIR/status.sh" << 'EOF'
#!/bin/bash
echo "=== Service Status ==="
sudo systemctl status bitquan-testnet --no-pager

echo ""
echo "=== Latest Logs ==="
sudo tail -20 /opt/bitquan/logs/testnet.log

echo ""
echo "=== Network Info ==="
curl -s http://localhost:8334/health || echo "RPC not responding"

echo ""
echo "=== Pool Stats ==="
curl -s http://localhost:8080/pool/stats | jq . || echo "Pool not responding"
EOF
chmod +x "$INSTALL_DIR/status.sh"

# Logs script
cat > "$INSTALL_DIR/logs.sh" << 'EOF'
#!/bin/bash
sudo journalctl -u bitquan-testnet -f
EOF
chmod +x "$INSTALL_DIR/logs.sh"

echo "✅ Helper scripts created"

# Enable and start service
echo -e "${YELLOW}🚀 Starting BitQuan testnet node...${NC}"
systemctl enable bitquan-testnet
systemctl start bitquan-testnet
sleep 3

# Check status
if systemctl is-active --quiet bitquan-testnet; then
    echo -e "${GREEN}✅ BitQuan testnet node is running!${NC}"
else
    echo -e "${RED}❌ Failed to start node${NC}"
    echo "Check logs: sudo journalctl -u bitquan-testnet -n 50"
    exit 1
fi

# Print summary
echo ""
echo -e "${GREEN}╔═══════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║          BitQuan Testnet Setup Complete! 🎉              ║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}📊 Node Information:${NC}"
echo "  Install Directory: $INSTALL_DIR"
echo "  Data Directory: $DATA_DIR"
echo "  Mining Address: $MINING_ADDRESS"
echo ""
echo -e "${YELLOW}🌐 Endpoints:${NC}"
echo "  P2P Port: 8333"
echo "  RPC Port: 8334"
echo "  Stratum Port: 3333"
echo "  Dashboard: http://$(hostname -I | awk '{print $1}'):8080"
echo "  Metrics: http://127.0.0.1:9090/metrics"
echo ""
echo -e "${YELLOW}🛠️  Useful Commands:${NC}"
echo "  Start node:   $INSTALL_DIR/start.sh"
echo "  Stop node:    $INSTALL_DIR/stop.sh"
echo "  View status:  $INSTALL_DIR/status.sh"
echo "  View logs:    $INSTALL_DIR/logs.sh"
echo ""
echo -e "${YELLOW}📖 Next Steps:${NC}"
echo "  1. Check status: $INSTALL_DIR/status.sh"
echo "  2. View dashboard: http://$(hostname -I | awk '{print $1}'):8080"
echo "  3. Read docs: cat $INSTALL_DIR/docs/TESTNET_SETUP.md"
echo "  4. Add to cluster: edit deploy/configs/cluster-nodes-testnet.txt"
echo ""
echo -e "${YELLOW}⚠️  Important:${NC}"
echo "  • Backup wallet: $INSTALL_DIR/config/pool-wallet.keystore"
echo "  • JWT secret: $INSTALL_DIR/config/jwt.secret"
echo "  • Monitor logs: sudo journalctl -u bitquan-testnet -f"
echo ""
echo -e "${GREEN}Happy testing! 🚀${NC}"
