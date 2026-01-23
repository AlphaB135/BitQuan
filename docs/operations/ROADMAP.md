# BitQuan Roadmap

## Current Status: v0.0.2-alpha (Development Build)

**Production Readiness**: ~50%
**Build Status**: Passing ✅
**Tests**: 82+ tests passing
**Version**: Development / Alpha

---

## Version Timeline

| Version | Status | Target Date | Description |
|----------|--------|-------------|-------------|
| v0.0.1-alpha | ✅ Done | Oct 2025 | Initial devnet release |
| v0.0.2-alpha | ✅ Done | Nov 2025 | Testnet development build |
| v0.1.0-beta | 🔜 Planned | Q1 2026 | Public testnet |
| v1.0.0 | 🔜 Planned | Q2 2026 | Mainnet (after audits) |

---

## Current Implementation Status

### ✅ Completed

- **Post-Quantum Cryptography**: CRYSTALS-Dilithium5 signatures (2592/4864 bytes)
- **Consensus Rules**: PoW validation, ASERT difficulty, block weight enforcement
- **P2P Networking**: Async network layer with Noise Protocol encryption
- **UTXO Model**: Transaction validation, double-spend prevention
- **Mining**: SHA-256d + RandomX support, Stratum protocol
- **Wallet**: BIP39 mnemonic support, encrypted keystore
- **RPC**: JSON-RPC 2.0 with JWT authentication
- **Storage**: RocksDB backend

### 🚧 In Progress

- **P2P Identity**: Ephemeral keys (V1) - persistent keys planned for V2
- **Testnet**: In development, not ready for public use
- **Security Audits**: Baseline complete, external audits planned

### 🔜 Planned (Before Mainnet)

- External security audits (2+ vendors)
- Penetration testing
- Public bug bounty program
- 6-month public beta period
- Production infrastructure setup

---

## Technical Specifications

### Consensus Parameters

| Parameter | Value |
|-----------|-------|
| Block time | 10 minutes (600 seconds) |
| Block weight limit | 4,000,000 WU |
| Signature weight | 384 WU per Dilithium5 signature |
| Coinbase maturity | 100 blocks |
| Max supply | 21,000,000 BQ |
| Halving interval | 210,000 blocks (~4 years) |

### Cryptography

| Component | Algorithm |
|-----------|-----------|
| Hash | SHA-256d |
| Signature | CRYSTALS-Dilithium5 |
| Key sizes | Pubkey: 2592 bytes, Privkey: 4864 bytes |
| Address | Bech32m (HRP: "q") |

### Network

| Network | Magic | P2P Port | RPC Port |
|----------|-------|----------|----------|
| Mainnet | `0xe8f3e1e3` | 8333 | 8332 |
| Testnet | `0x...` | 19444 | 19443 |
| Devnet | `0x...` | 18333 | 18332 |

---

## Blocking Issues for Mainnet

1. **Security Audits**: External audits not yet completed
2. **Testnet Stability**: Public testnet not yet launched
3. **Persistent P2P Keys**: Currently using ephemeral keys (IP-based bans only)
4. **Production Infrastructure**: Monitoring, alerting, backups not deployed

---

## Development Phases

### Phase 1-6: Core Protocol ✅
- Consensus, P2P, Mining, Wallet, RPC, Storage
- Status: Complete

### Phase 7: Security Hardening 🚧
- Internal audits: Complete
- External audits: Pending
- Fuzzing: Infrastructure ready, targets needed

### Phase 8: Testnet 🚧
- Private testnet: Running
- Public testnet: In development
- Faucet: Not implemented

### Phase 9: Mainnet Preparation 🔜
- External audits
- Penetration testing
- Bug bounty
- Production infrastructure
- 6-month beta

### Phase 10: Mainnet Launch 🔜
- Genesis block
- Seed nodes
- DNS seeds
- Explorer integration

---

## Remaining Work

### High Priority
- [ ] Complete external security audits
- [ ] Fix persistent P2P identity (V2)
- [ ] Launch public testnet
- [ ] Deploy production monitoring

### Medium Priority
- [ ] Implement testnet faucet
- [ ] Add more fuzzing targets
- [ ] Improve test coverage (>80%)
- [ ] Performance optimization

### Low Priority
- [ ] Mobile wallet
- [ ] Hardware wallet support
- [ ] Multi-signature transactions
- [ ] Advanced scripting features

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

**Quick Start**:
```bash
cargo build --release
cargo test --all
./target/release/bitquan-node --network devnet
```

---

## License

Apache License 2.0 - See [LICENSE](LICENSE)

---

*Last Updated: 2026-01-03*
*Version: 0.0.2-alpha*
*Status: Active Development*
