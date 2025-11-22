# Mainnet Hybrid Mining Update

## Overview

BitQuan mainnet now supports hybrid mining with multiple Proof-of-Work algorithms to enable inclusive participation from different types of mining hardware.

## Algorithm Activation Schedule

| Block Height | SHA-256d | RandomX | Ethash |
|--------------|-----------|---------|--------|
| 0 - 9,999    | ✅ Active | ❌ Disabled | ❌ Disabled |
| 10,000+      | ✅ Active | ✅ Active | ✅ Active |

## Mining Algorithms

### 1. SHA-256d (ASIC-friendly)
- **Hardware**: Bitcoin ASIC miners (Antminer S19, WhatsMiner M30S, etc.)
- **Availability**: From genesis block (block 0)
- **Characteristics**: Highest efficiency, proven security
- **Pool Port**: 3334

### 2. RandomX (CPU-friendly)
- **Hardware**: Modern CPUs (Ryzen 9, Xeon, Threadripper)
- **Availability**: From block 10,000
- **Characteristics**: Memory-hard, ASIC-resistant, quantum-resistant
- **Pool Port**: 3336
- **Memory Requirements**: 2GB+ RAM recommended

### 3. Ethash (GPU-friendly)
- **Hardware**: Modern GPUs (RTX 4090, RX 7900 XTX)
- **Availability**: From block 10,000
- **Characteristics**: Memory-hard, GPU-optimized
- **Pool Port**: 3335
- **Memory Requirements**: 4GB+ VRAM recommended

## Mining Pool Configuration

```bash
# SHA-256d (ASIC)
stratum+tcp://pool.bitquan.org:3334

# Ethash (GPU)
stratum+tcp://pool.bitquan.org:3335

# RandomX (CPU)
stratum+tcp://pool.bitquan.org:3336

# Hybrid (auto-detect)
stratum+tcp://pool.bitquan.org:3333
```

## Algorithm Weights

The network uses weighted difficulty adjustment to balance mining power:

- **SHA-256d**: 1.0x weight (baseline)
- **Ethash**: 2.0x weight (GPU-friendly bonus)
- **RandomX**: 1.5x weight (CPU-friendly bonus)

## Security Considerations

### BurstGuard Protection
- Prevents 51% attacks across all algorithms
- Monitors hashrate spikes and adjusts difficulty
- Cross-algorithm difficulty coordination

### Geographic Distribution
- Maximum 30% voting power per geographic region
- Applies to all mining algorithms equally

### Economic Safeguards
- Staking requirements apply to all miners
- Slashing for malicious behavior across algorithms

## Migration Guide

### For ASIC Miners
No changes required - continue mining SHA-256d as before.

### For CPU Miners
1. Wait for block 10,000 activation
2. Use RandomX-compatible mining software (xmrig, etc.)
3. Connect to RandomX pool port 3336

### For GPU Miners
1. Wait for block 10,000 activation
2. Use Ethash-compatible mining software (teamredminer, etc.)
3. Connect to Ethash pool port 3335

## Technical Implementation

### Consensus Changes
- `PowSetParams::mainnet()` updated with hybrid activation at block 10,000
- All algorithm feature flags removed from mainnet builds
- Cross-algorithm difficulty coordination via ASERT

### Network Compatibility
- Existing Bitcoin ASIC miners fully compatible
- Standard Stratum V1 protocol support
- Automatic algorithm detection in hybrid pools

## Monitoring

### Network Metrics
- Individual hashrate tracking per algorithm
- Cross-algorithm difficulty adjustment monitoring
- Geographic distribution verification

### Pool Operations
- Algorithm-specific hashrate reporting
- Automatic failover between algorithm pools
- Unified payment system across all algorithms

## Future Considerations

### Algorithm Adjustments
- Weights may be adjusted based on network participation
- New algorithms can be added via network upgrade
- Activation heights can be modified by consensus

### Economic Incentives
- Dynamic reward adjustments possible
- Algorithm-specific bonus mechanisms
- Long-term sustainability planning

---

**Note**: This update maintains full backward compatibility with existing SHA-256d miners while enabling broader participation through additional algorithms.
