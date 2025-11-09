# Bootstrap Node Deployment Checklist

## 🚀 Immediate Actions Required

### 1. DNS Configuration (URGENT)
Configure the following DNS records:

```
seed1.bitquan.network A <SERVER_IP_1>
seed2.bitquan.network A <SERVER_IP_2>
seed3.bitquan.network A <SERVER_IP_3>
seed4.bitquan.network A <SERVER_IP_4>
seed5.bitquan.network A <SERVER_IP_5>
```

### 2. Server Deployment
For each server (5 total):

```bash
# Deploy bootstrap node
./deploy/deploy-bootstrap.sh <NODE_ID> <SERVER_IP> <REGION>

# Examples:
./deploy/deploy-bootstrap.sh 1 192.168.1.101 usa-east
./deploy/deploy-bootstrap.sh 2 192.168.1.102 eu-west
./deploy/deploy-bootstrap.sh 3 192.168.1.103 asia-east
./deploy/deploy-bootstrap.sh 4 192.168.1.104 asia-west
./deploy/deploy-bootstrap.sh 5 192.168.1.105 south-america
```

### 3. Verification Commands
After deployment, verify each node:

```bash
# Check node status
systemctl status bitquan-node

# Check port connectivity
telnet seed1.bitquan.network 8333

# Check P2P connectivity
./target/release/bitquan-node p2p-connect --peer seed1.bitquan.network:8333

# Monitor logs
journalctl -u bitquan-node -f
```

---

## 📋 Deployment Regions Recommended

| Node | Region | Provider | IP Range |
|------|--------|----------|----------|
| seed1 | USA East | AWS/Vultr | 192.168.1.101 |
| seed2 | EU West | Hetzner/DigitalOcean | 192.168.1.102 |
| seed3 | Asia East | AWS/Linode | 192.168.1.103 |
| seed4 | Asia West | Vultra/Google Cloud | 192.168.1.104 |
| seed5 | South America | AWS/Azure | 192.168.1.105 |

---

## 🔧 Technical Requirements

### Server Specs (Minimum)
- **CPU**: 2+ cores
- **RAM**: 4GB+ 
- **Storage**: 50GB+ SSD
- **Network**: 100Mbps+ upload
- **OS**: Ubuntu 20.04+ LTS

### Firewall Rules
```bash
# Required ports
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 8333/tcp  # P2P
sudo ufw allow 8333/udp  # P2P
sudo ufw allow 8332/tcp  # RPC (optional)
sudo ufw allow 8334/tcp  # Stratum (optional)
```

### Monitoring Setup
```bash
# Check node status every 5 minutes
*/5 * * * * /usr/local/bin/bitquan-monitor.sh <NODE_ID>

# Monitor with:
watch -n 5 'systemctl status bitquan-node'
tail -f /var/log/bitquan-node-<NODE_ID>.log
```

---

## ✅ Verification Checklist

After deploying all 5 nodes:

### Network Tests
- [ ] All 5 DNS records resolve correctly
- [ ] All nodes accept P2P connections on port 8333
- [ ] Nodes can interconnect (test p2p-connect between each)
- [ ] Bootstrap nodes appear in peer lists
- [ ] No firewall blocks

### Performance Tests  
- [ ] Block propagation < 30 seconds between nodes
- [ ] Peer connections stable (24+ hours)
- [ ] Memory usage < 2GB per node
- [ ] CPU usage < 50% normally
- [ ] Disk I/O within limits

### Security Tests
- [ ] TLS certificates valid (if RPC enabled)
- [ ] JWT authentication working
- [ ] Rate limiting active
- [ ] No open ports except required ones
- [ ] System updates applied

---

## 🚨 Rollback Plan

If critical issues detected:

### Immediate Actions
1. **Stop all nodes**: `systemctl stop bitquan-node`
2. **Update DNS**: Point to backup nodes
3. **Announce pause**: Community notification
4. **Fix issues**: Debug and resolve
5. **Relaunch**: Gradual restart

### DNS Rollback
```bash
# Quick DNS switch to backup
seed1.bitquan.network A <BACKUP_IP_1>
seed2.bitquan.network A <BACKUP_IP_2>
```

---

## 📊 Monitoring Dashboard

Set up monitoring for:

### Key Metrics
- **Node uptime**: Target 99.9%+
- **Peer count**: 8+ outbound, 50+ inbound
- **Block sync**: Current with network
- **Memory usage**: < 2GB per node
- **Network I/O**: Within bandwidth limits

### Alert Thresholds
- **Node down**: Immediate alert
- **< 5 peers**: Warning after 5 minutes
- **High memory**: Warning at 80% usage
- **High CPU**: Warning at 90% usage
- **Disk full**: Critical at 90% usage

---

## 🎯 Success Criteria

Bootstrap network considered **LIVE** when:

- ✅ **5+ nodes** running globally
- ✅ **DNS propagation** complete (24-48 hours)
- ✅ **P2P connectivity** working between all nodes
- ✅ **24+ hours** stable operation
- ✅ **Monitoring** active and alerting
- ✅ **Documentation** published to community

---

## 📞 Emergency Contacts

### Technical Issues
- **Infrastructure**: infrastructure@bitquan.org
- **Security**: security@bitquan.org
- **DNS Issues**: dns@bitquan.org

### Community Updates
- **Twitter**: @BitQuanCrypto
- **Telegram**: t.me/bitquan
- **GitHub**: github.com/AlphaB135/BitQuan/issues

---

**Priority**: CRITICAL - Complete within 24 hours of mainnet launch  
**Impact**: High - Affects network bootstrapping and user onboarding  
**Owner**: Infrastructure Team / DevOps