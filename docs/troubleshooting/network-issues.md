# Network Issues Troubleshooting

Can't connect to peers? Connection timeouts? Handshake failures? This guide helps diagnose and fix P2P networking problems.

## Symptoms

- Zero peer connections
- "Connection refused" errors
- "Connection timeout" errors
- "Handshake failed" messages
- "Peer disconnected" repeatedly
- High latency in block propagation

## Diagnostic Steps

### 1. Check Peer Count

```bash
# Check logs for peer connections
grep -i "peer\|connection" bitquan.log | tail -20

# Should see messages like:
# "Connected to peer: X.X.X.X:18444"
# "Peer count: N"
```

### 2. Verify Port Accessibility

```bash
# Check if P2P port is listening
netstat -tuln | grep 18444

# Or with lsof
lsof -i :18444

# Should show bitquan-node listening on 0.0.0.0:18444
```

### 3. Test Firewall

```bash
# Test from local machine
telnet 127.0.0.1 18444

# Test from external (ask friend or use online port checker)
# Port should be accessible from internet
```

### 4. Check Network Configuration

```bash
# Verify config file
cat config/devnet.toml | grep -A5 "\[p2p\]"

# Should show:
# listen_address = "0.0.0.0:18444"
```

## Common Issues and Solutions

### Issue: "Connection Refused"

**Symptoms:**
- `telnet <peer-ip> 18444` fails immediately
- "Connection refused" in logs

**Possible Causes:**

#### A. Port Not Open on Remote Peer

**What's happening:** Peer's P2P port is closed or firewalled.

**Solution:**
- Try different peer addresses
- Use seed nodes from documentation
- Check if peer is actually online

#### B. Your Firewall Blocking Outbound

**What's happening:** Your firewall prevents BitQuan from connecting.

**Solution:**
```bash
# Allow outbound P2P connections (Linux/ufw)
sudo ufw allow out 18444/tcp

# Or allow the specific binary
sudo ufw allow out to any app bitquan-node

# Allow inbound (if you want to accept connections)
sudo ufw allow 18444/tcp
```

#### C. Wrong Network/Port

**What's happening:** Connecting to wrong port or network.

**Solution:**
- Verify `--network` flag matches peer (devnet/testnet/mainnet)
- Check port is correct (default: 18444)
- Use `--peers` flag to specify correct addresses

### Issue: "Connection Timeout"

**Symptoms:**
- Connection attempt hangs then times out
- "Timeout connecting to peer" in logs

**Possible Causes:**

#### A. Remote Firewall Blocking You

**What's happening:** Peer's firewall is silently dropping packets.

**Solution:**
- Try different peers
- If you control the peer, open their firewall
- Use port forwarding if behind NAT

#### B. NAT/Router Issues

**What's happening:** Your router isn't forwarding P2P port correctly.

**Solution:**
```bash
# Port forwarding setup (router-specific)
# Forward external port 18444 to internal port 18444
# Forward to your machine's local IP (192.168.x.x)

# Alternatively, use UPnP (if router supports)
# BitQuan will attempt UPnP automatically
```

#### C. ISP Blocking P2P Traffic

**What's happening:** ISP is throttling or blocking P2P protocols.

**Solution:**
- Contact ISP to verify
- Use VPN as workaround
- Try different port (configure in config file)

### Issue: "Handshake Failed"

**Symptoms:**
- Connection establishes but immediately drops
- "Handshake failed" or "Invalid version" in logs

**Possible Causes:**

#### A. Protocol Version Mismatch

**What's happening:** Peer has incompatible BitQuan version.

**Solution:**
```bash
# Update to latest version
cd BitQuan
git pull origin main
cargo build --release

# Verify version
./target/release/bitquan-node --version
```

#### B. Wrong Network Magic

**What's happening:** You're on devnet trying to connect to testnet (or vice versa).

**Solution:**
- Verify `--network` flag
- Ensure config matches network
- Check peer is on same network

#### C. Noise Protocol Encryption Failure

**What's happening:** Encrypted handshake negotiation failed.

**Solution:**
- Verify both peers support Noise Protocol
- Check logs for specific crypto errors
- Report as bug if persists

### Issue: Zero Peers Connected

**Symptoms:**
- Node starts but never connects to anyone
- "Peer count: 0" in logs

**Solution:**
```bash
# Add seed peers manually
./target/release/bitquan-node \
  --network devnet \
  --datadir ./data/chainstate \
  --peers 149.56.132.54:18444 \
  --peers <another-peer>:18444

# Or specify multiple peers
# (Check documentation/community for active peer list)
```

### Issue: Peers Disconnect Repeatedly

**Symptoms:**
- Connections establish but drop after short time
- "Peer disconnected" messages repeatedly

**Possible Causes:**

#### A. Slowloris Protection Triggering

**What's happening:** 30-second message timeout is kicking in.

**Solution:**
- Verify network is stable
- Check for high latency
- Ensure peer is sending regular messages

#### B. Bandwidth Saturation

**What's happening:** Can't handle incoming data fast enough.

**Solution:**
```bash
# Limit max connections (if applicable)
# Check config for max_peers setting

# Monitor bandwidth
iftop -i eth0  # or use netstat/nethogs
```

#### C. Protocol Violation

**What's happening:** Peer sending invalid messages.

**Solution:**
- Ban peer (if client supports it)
- Report peer misbehavior
- Connect to different peers

## Configuration Examples

### Basic P2P Config

```toml
# config/devnet.toml
[p2p]
listen_address = "0.0.0.0:18444"
max_peers = 50
peer_discovery = true
enable_upnp = true

[network]
magic = "devnet"
```

### Static Peer Configuration

```bash
# Start with specific peers
./target/release/bitquan-node \
  --network devnet \
  --peers 1.2.3.4:18444 \
  --peers 5.6.7.8:18444 \
  --peers 9.10.11.12:18444
```

### Test Network Connectivity

```bash
# Test connection to peer
nc -zv <peer-ip> 18444

# Test with timeout
timeout 5 nc -zv <peer-ip> 18444

# Trace route
traceroute <peer-ip>
```

## Firewall Configuration by Platform

### Linux (ufw)

```bash
# Allow inbound P2P
sudo ufw allow 18444/tcp

# Allow outbound
sudo ufw allow out 18444/tcp

# Check status
sudo ufw status
```

### Linux (iptables)

```bash
# Allow inbound
sudo iptables -A INPUT -p tcp --dport 18444 -j ACCEPT

# Allow outbound
sudo iptables -A OUTPUT -p tcp --dport 18444 -j ACCEPT

# Save rules
sudo iptables-save | sudo tee /etc/iptables/rules.v4
```

### macOS

```bash
# Allow BitQuan through firewall
# System Preferences > Security & Privacy > Firewall
# Click "Options" and add bitquan-node

# Or use pf (packet filter)
sudo echo "pass in proto tcp to any port 18444" | sudo pfctl -ef -
```

### Windows (Firewall)

```
1. Open Windows Defender Firewall
2. Advanced Settings > Inbound Rules
3. New Rule > Port > TCP > 18444
4. Allow the connection
5. Apply to all profiles
6. Name: "BitQuan P2P"

Repeat for Outbound Rules.
```

## Testing Your Connection

### Local Test

```bash
# Start a test node (terminal 1)
./target/release/bitquan-node --network devnet --datadir ./data/node1

# Connect to it from another node (terminal 2)
./target/release/bitquan-node \
  --network devnet \
  --datadir ./data/node2 \
  --peers 127.0.0.1:18444
```

### External Test

```bash
# Check if port is open from internet
# Use online port checker:
# - https://www.yougetsignal.com/tools/open-ports/
# - https://canyouseeme.org/

# Enter: your-ip:18444
# Should show "Open"
```

### Latency Test

```bash
# Measure latency to peer
ping <peer-ip>

# Test TCP latency
tcptraceroute <peer-ip> 18444
```

## Advanced: Manually Add Peers

### From Peers File

```bash
# Create peers list
cat > peers.txt << EOF
149.56.132.54:18444
<another-peer>:18444
<yet-another>:18444
EOF

# Use with node (if supported)
./target/release/bitquan-node --peers-file peers.txt
```

### DNS Seed Fallback

If DNS seeds fail, use hardcoded IPs:

```bash
# Common seed nodes (check docs for current list)
./target/release/bitquan-node \
  --peers seed1.bitquan.org:18444 \
  --peers seed2.bitquan.org:18444
```

## Prevention Tips

1. **Static IP:** Use static IP if hosting a seed node
2. **Port Forwarding:** Configure router for inbound connections
3. **Firewall Rules:** Persist rules across reboots
4. **Monitoring:** Log peer connections for diagnostic
5. **Backup Peers:** Maintain list of known good peers

## Still Having Issues?

1. **Check Firewall Logs:**
   ```bash
   sudo journalctl -u firewalld | grep 18444
   # Or
   sudo grep 18444 /var/log/syslog
   ```

2. **Network Dump:**
   ```bash
   # Capture traffic for analysis
   sudo tcpdump -i any -w capture.pcap port 18444
   # Open capture.pcap in Wireshark
   ```

3. **Gather Info:**
   ```bash
   # System info
   uname -a > net-diag.txt
   netstat -tuln >> net-diag.txt
   iptables -L -n >> net-diag.txt
   ```

## Related Guides

- [Sync Issues](sync-issues.md) - Sync problems due to no peers
- [FAQ](faq.md) - "How do I connect to testnet?"
