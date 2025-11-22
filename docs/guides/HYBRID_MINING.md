# Hybrid Mining Guide

**Last Updated: 2025-11-09**

BitQuan's hybrid mining system supports multiple Proof-of-Work algorithms simultaneously, creating a more decentralized and resilient network. This guide covers everything you need to know about mining BitQuan.

## Quick Start

### 1. Choose Your Mining Hardware

| Algorithm | Hardware Type | Efficiency | Examples |
|-----------|---------------|------------|----------|
| **SHA-256d** | ASIC | Highest | Antminer S19, WhatsMiner M30S |
| **Ethash** | GPU | High | RTX 4090, RX 7900 XTX |
| **RandomX** | CPU | Medium | Ryzen 9, Xeon, Threadripper |

### 2. Select a Mining Pool

Find a pool that supports your preferred algorithm:

```bash
# SHA-256d pools
stratum+tcp://pool1.bitquan.org:3334  # ASIC-focused
stratum+tcp://pool2.bitquan.org:3334  # Backup pool

# Ethash pools
stratum+tcp://pool1.bitquan.org:3335  # GPU-focused
stratum+tcp://pool2.bitquan.org:3335  # Backup pool

# RandomX pools (mainnet enabled at block 10,000)
stratum+tcp://pool1.bitquan.org:3336  # CPU-focused
stratum+tcp://pool2.bitquan.org:3336  # Backup pool

# Hybrid pools (auto-detect)
stratum+tcp://pool1.bitquan.org:3333  # All algorithms
```

### 3. Configure Your Miner

#### ASIC Miners (SHA-256d)

```bash
# Antminer configuration
Pool URL: stratum+tcp://pool1.bitquan.org:3334
Worker: your_wallet_address.worker_name
Password: x
```

#### GPU Miners (Ethash)

**PhoenixMiner:**
```bash
PhoenixMiner.exe -pool pool1.bitquan.org:3335 -wal your_wallet_address.rig_name -proto stratum
```

**lolMiner:**
```bash
lolMiner --pool pool1.bitquan.org:3335 --user your_wallet_address.rig_name --algo ethash
```

#### CPU Miners (RandomX)

**XMRig:**
```bash
xmrig -o pool1.bitquan.org:3336 -u your_wallet_address.rig_name -a rx/0 --donate-level=1
```

## Algorithm Details

### SHA-256d (Bitcoin-style)

**Best for:** ASIC miners
**Hash function:** Double SHA-256
**Network share:** 33% (default weight: 1.0)

**Pros:**
- Highest efficiency with ASICs
- Mature ecosystem
- Stable difficulty

**Cons:**
- High hardware cost
- Centralization risk
- Not quantum-resistant

**Hardware recommendations:**
- Antminer S19 Pro (110 TH/s)
- WhatsMiner M30S++ (112 TH/s)
- MicroBT WhatsMiner M50 (126 TH/s)

### Ethash (Ethereum-style)

**Best for:** GPU miners
**Hash function:** Ethash (Dagger-Hashimoto)
**Network share:** 50% (default weight: 2.0)

**Pros:**
- GPU-friendly
- Good decentralization
- Widely available hardware

**Cons:**
- Higher power consumption than ASICs
- Memory intensive
- Lower efficiency than SHA-256d

**Hardware recommendations:**
- NVIDIA RTX 4090 (120 MH/s)
- AMD RX 7900 XTX (115 MH/s)
- NVIDIA RTX 3090 (120 MH/s)

### RandomX (Monero-style)

**Best for:** CPU miners
**Hash function:** RandomX
**Network share:** 17% (default weight: 1.0)

**Pros:**
- CPU-optimized
- Quantum-resistant
- Low barrier to entry
- Excellent decentralization

**Cons:**
- Testnet/devnet only
- Lower efficiency
- Not available on mainnet

**Hardware recommendations:**
- AMD Ryzen 9 7950X (15 KH/s)
- Intel Core i9-13900K (12 KH/s)
- AMD Threadripper 3990X (25 KH/s)

## Pool Configuration

### Finding Pools

Use the BitQuan pool explorer:
- Web: https://pools.bitquan.org
- API: https://api.bitquan.org/pools

### Pool Selection Criteria

1. **Algorithm Support**: Ensure pool supports your hardware
2. **Fee Structure**: Compare pool fees (typically 1-3%)
3. **Payout Scheme**: PPS, PPLNS, SOLO
4. **Geographic Location**: Lower latency = better performance
5. **Reputation**: Check community feedback

### Pool Configuration Examples

**Multi-pool setup for redundancy:**
```bash
# Primary pool
Pool 1: stratum+tcp://pool1.bitquan.org:3334
Worker: wallet.worker1
Password: x

# Backup pool
Pool 2: stratum+tcp://pool2.bitquan.org:3334
Worker: wallet.worker1
Password: x

# Tertiary pool
Pool 3: stratum+tcp://pool3.bitquan.org:3334
Worker: wallet.worker1
Password: x
```

## Mining Software Setup

### ASIC Configuration

**Antminer Web Interface:**
1. Navigate to Miner Configuration
2. Add pool URL: `stratum+tcp://pool.bitquan.org:3334`
3. Set Worker: `your_wallet_address.worker_name`
4. Save and restart

**Command line configuration:**
```bash
# cgminer for ASICs
cgminer -o stratum+tcp://pool.bitquan.org:3334 -u wallet.worker -p x --algo sha256d
```

### GPU Configuration

**NVIDIA GPUs:**
```bash
# PhoenixMiner optimized settings
PhoenixMiner.exe -pool pool.bitquan.org:3335 -wal wallet.rig -proto stratum -mi 12 -mc 1500

# lolMiner settings
lolMiner --pool pool.bitquan.org:3335 --user wallet.rig --algo ethash --dualmode off
```

**AMD GPUs:**
```bash
# TeamRedMiner settings
teamredminer -a ethash -o stratum+tcp://pool.bitquan.org:3335 -u wallet.rig -p x --eth_config 1

# lolMiner AMD settings
lolMiner --pool pool.bitquan.org:3335 --user wallet.rig --algo ethash --amd
```

### CPU Configuration

**XMRig settings:**
```bash
# Basic RandomX mining
xmrig -o pool.bitquan.org:3336 -u wallet.rig -a rx/0 --donate-level=1

# Advanced configuration
xmrig -o pool.bitquan.org:3336 -u wallet.rig -a rx/0 \
  --cpu-max-threads-hint 100 \
  --cpu-priority 2 \
  --huge-pages true \
  --donate-level=1
```

## Performance Optimization

### Hardware Optimization

**ASIC Optimization:**
- Maintain proper cooling (target: 60-75°C)
- Use high-quality power supplies
- Update firmware regularly
- Monitor power efficiency (J/TH)

**GPU Optimization:**
```bash
# NVIDIA overclocking
nvidia-smi -pl 300W  # Power limit
nvidia-smi -ac 877,1215  # Memory/Core clocks

# AMD memory timings
# Use AMD Memory Tweak for better performance
```

**CPU Optimization:**
- Disable hyperthreading for RandomX
- Set high performance power plan
- Enable XMP/DOCP for RAM
- Use fast RAM (3200MHz+ DDR4)

### Software Optimization

**Mining Software Flags:**
```bash
# cgminer optimization
cgminer --intensity 20 --gpu-engine 1150 --gpu-memclock 1500

# PhoenixMiner optimization
PhoenixMiner -mi 12 -mc 1500 -mt 1

# XMRig optimization
xmrig --cpu-max-threads-hint 100 --cpu-priority 2
```

**System Optimization:**
- Use Linux for better performance
- Disable unnecessary services
- Use SSD for OS and mining software
- Monitor system temperatures

## Monitoring and Troubleshooting

### Key Metrics to Monitor

1. **Hashrate**: Maintain consistent hashrate
2. **Rejected Shares**: Should be < 1%
3. **Temperature**: Keep hardware in optimal range
4. **Power Consumption**: Monitor efficiency
5. **Pool Latency**: Lower is better

### Common Issues and Solutions

**Low Hashrate:**
```bash
# Check hardware status
nvidia-smi
cgminer --stats

# Verify pool connection
telnet pool.bitquan.org 3334
```

**High Rejected Shares:**
- Check network connectivity
- Reduce overclocking
- Verify pool URL and port
- Update mining software

**Connection Issues:**
```bash
# Test pool connectivity
ping pool.bitquan.org
nslookup pool.bitquan.org

# Check firewall settings
ufw status
iptables -L
```

### Monitoring Tools

**Built-in monitoring:**
```bash
# Pool statistics
curl http://pool.bitquan.org/api/stats

# Worker stats
curl http://pool.bitquan.org/api/workers/wallet_address
```

**Third-party tools:**
- Awesome Miner (Windows)
- Foreman (cross-platform)
- custom scripts with Prometheus/Grafana

## Profitability Analysis

### Factors Affecting Profitability

1. **Block Reward**: Current BitQuan block reward
2. **Network Difficulty**: Adjusts based on total hashrate
3. **Pool Fees**: Typically 1-3%
4. **Electricity Cost**: Major operational expense
5. **Hardware Cost**: Initial investment
6. **Market Price**: BitQuan market value

### Profitability Calculators

Use online calculators or create your own:

```python
# Simple profitability calculation
def calculate_profitability(hashrate_ths, power_watts, electricity_cost_usd, block_reward, difficulty):
    # Network stats (example values)
    network_hashrate = 1000000  # TH/s
    blocks_per_day = 1440  # 60-second blocks

    # Daily earnings
    daily_blocks = blocks_per_day * (hashrate_ths / network_hashrate)
    daily_bq = daily_blocks * block_reward

    # Daily costs
    daily_power_kwh = (power_watts / 1000) * 24
    daily_electricity_cost = daily_power_kwh * electricity_cost_usd

    # Net profit
    daily_profit = daily_bq * bq_price_usd - daily_electricity_cost

    return daily_profit
```

### ROI Calculation

```bash
# Example ROI calculation
Hardware cost: $5000
Daily profit: $10
ROI days: 5000 / 10 = 500 days
ROI months: 500 / 30 ≈ 17 months
```

## Security Best Practices

### Wallet Security

1. **Use secure wallets**: Hardware wallets recommended
2. **Separate wallets**: Use different wallets for mining and storage
3. **Regular backups**: Backup wallet files and seeds
4. **Two-factor authentication**: Enable where available

### Mining Security

1. **Secure mining rigs**: Use strong passwords
2. **Network security**: Use VPNs for remote access
3. **Software updates**: Keep mining software updated
4. **Monitor for malware**: Regular security scans

### Pool Security

1. **Reputable pools**: Use well-established pools
2. **SSL connections**: Use secure connections where available
3. **Payout verification**: Verify received payments
4. **Diversify**: Use multiple pools for redundancy

## Advanced Topics

### Solo Mining

For advanced users with significant hashrate:

```bash
# Solo mining configuration
bitquan-node --solo-mining --wallet-address your_wallet_address
```

**Requirements:**
- Full node setup
- Significant hashrate (>1% network)
- Technical expertise
- Risk tolerance

### Mining Pool Operations

Running your own mining pool:

1. **Server requirements**: High-performance server
2. **Network connectivity**: Low latency, high bandwidth
3. **Software setup**: BitQuan pool software
4. **Security**: DDoS protection, SSL certificates
5. **Monitoring**: Pool performance and user metrics

### Algorithm Development

For developers interested in new PoW algorithms:

1. **Research**: Study existing algorithms
2. **Implementation**: Create PowEngine trait
3. **Testing**: Comprehensive testing required
4. **Community review**: Peer review process
5. **Network upgrade**: Coordination for deployment

## Community and Support

### Getting Help

- **Discord**: https://discord.gg/bitquan
- **Telegram**: https://t.me/bitquan_mining
- **Reddit**: r/BitQuan
- **GitHub**: https://github.com/bitquan/bitquan/issues

### Contributing

1. **Bug reports**: Submit detailed bug reports
2. **Feature requests**: Propose new features
3. **Documentation**: Improve documentation
4. **Testing**: Help test new releases
5. **Translation**: Help translate documentation

### Resources

- **Official website**: https://bitquan.org
- **Documentation**: https://docs.bitquan.org
- **Block explorer**: https://explorer.bitquan.org
- **Pool list**: https://pools.bitquan.org
- **API reference**: https://api.bitquan.org

---

*Last updated: 2025-11-09*
