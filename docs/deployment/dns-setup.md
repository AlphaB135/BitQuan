# BitQuan DNS Records Setup Guide

## Required DNS Records for Mainnet Launch

### Seed Nodes (A Records)
```
seed1.bitquan.network.    IN A    <SERVER_IP_1>
seed2.bitquan.network.    IN A    <SERVER_IP_2>
seed3.bitquan.network.    IN A    <SERVER_IP_3>
seed4.bitquan.network.    IN A    <SERVER_IP_4>
seed5.bitquan.network.    IN A    <SERVER_IP_5>
```

### Bootstrap Nodes (A Records)
```
node1.bitquan.network.    IN A    <SERVER_IP_1>
node2.bitquan.network.    IN A    <SERVER_IP_2>
node3.bitquan.network.    IN A    <SERVER_IP_3>
```

### Ports Configuration
- **P2P Port:** 8333 (TCP/UDP)
- **RPC Port:** 8332 (TCP)
- **Stratum Port:** 8334 (TCP)

### Example BIND Configuration
```bind
; BitQuan Mainnet Seeds
$TTL 300
bitquan.network.    IN SOA   ns1.bitquan.network. admin.bitquan.network. (
                     2025110901 ; Serial
                     3600       ; Refresh
                     1800       ; Retry
                     604800     ; Expire
                     300 )      ; Minimum TTL

; Seed Nodes
seed1    IN A    192.168.1.101
seed2    IN A    192.168.1.102
seed3    IN A    192.168.1.103
seed4    IN A    192.168.1.104
seed5    IN A    192.168.1.105

; Bootstrap Nodes
node1    IN A    192.168.1.101
node2    IN A    192.168.1.102
node3    IN A    192.168.1.103
```

### Cloudflare Setup (Alternative)
1. Add domain `bitquan.network` to Cloudflare
2. Create A records for each seed/node
3. Disable proxy (orange cloud) - use DNS only
4. Set TTL to 300 (5 minutes)

### Testing DNS Configuration
```bash
# Test DNS resolution
nslookup seed1.bitquan.network
dig seed1.bitquan.network A

# Test connectivity
telnet seed1.bitquan.network 8333
nc -zv seed1.bitquan.network 8333
```

### Firewall Requirements
Ensure these ports are open on seed servers:
- 8333/tcp (P2P)
- 8333/udp (P2P discovery)
- 8332/tcp (RPC - internal only)
- 8334/tcp (Stratum mining)

### Geographic Distribution
Recommended server locations:
- Asia: seed1.bitquan.network (Tokyo/Singapore)
- Europe: seed2.bitquan.network (Frankfurt/London)
- US East: seed3.bitquan.network (New York/Virginia)
- US West: seed4.bitquan.network (California/Oregon)
- Australia: seed5.bitquan.network (Sydney)

### Monitoring
Monitor seed nodes with:
```bash
# Check node status
./target/release/bitquan-node --config config/mainnet.toml --check-seeds

# Monitor connectivity
watch -n 30 'nslookup seed1.bitquan.network'
```

## Deployment Checklist
- [ ] Register domain `bitquan.network`
- [ ] Setup 5+ geographically distributed servers
- [ ] Configure firewall rules
- [ ] Create DNS A records
- [ ] Test DNS resolution
- [ ] Verify port connectivity
- [ ] Deploy bootstrap nodes
- [ ] Test peer discovery
- [ ] Monitor for 24 hours before launch
