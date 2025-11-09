# Emergency Response Quick Reference

This is a quick reference guide for BitQuan emergency response procedures. For detailed information, see [EMERGENCY_PROCEDURES.md](EMERGENCY_PROCEDURES.md).

## 🚨 Immediate Actions

### 1. Mining Bug / Consensus Failure
```bash
# Pause processing
EmergencyAction::PauseProcessing

# Create checkpoint at last known good height
manager.create_emergency_checkpoint(height, hash, reason, operator_id)

# Enable checkpoint validation
EmergencyAction::EnableCheckpoints
```

### 2. Network Attack
```bash
# Ban malicious peers
EmergencyAction::BanPeers { peer_ids: ["peer1", "peer2"] }

# Send alert
EmergencyAction::SendAlert { message: "Attack detected - update required" }
```

### 3. Chain Reorganization
```bash
# Rollback to safe height
EmergencyAction::RollbackTo { height: safe_height }

# Create checkpoint on legitimate chain
manager.create_emergency_checkpoint(height, hash, reason, operator_id)
```

## 📋 Decision Matrix

| Situation | Action Required | Time Critical |
|-----------|----------------|---------------|
| Mining bug | Pause + Checkpoint + Enable | ⚡ Immediate |
| Network attack | Ban peers + Alert | ⚡ Immediate |
| Software vulnerability | Checkpoint + Update | 🕐 Within hours |
| Chain reorg | Rollback + Checkpoint | ⚡ Immediate |
| Infrastructure failure | Assess + Coordinate | 🕐 As needed |

## 🔐 Authorization Requirements

- **Minimum signatures**: 3 (production), 1 (development)
- **Authorized operators**: Pre-configured list
- **Response window**: 1 hour (production), 5 minutes (dev)

## 🛡️ Security Limits

- **Max checkpoints**: 100
- **Min checkpoint interval**: 1000 blocks
- **No genesis checkpoints**
- **No future checkpoints**

## 📞 Emergency Contacts

- **Primary Coordinator**: [Contact]
- **Security Team**: [Contact]
- **Dev Team**: [Contact]

## 🧪 Testing Commands

```bash
# Test checkpoint system
cargo test -p bitquan-consensus checkpoint

# Test emergency system
cargo test -p bitquan-consensus emergency

# Run all emergency tests
cargo test -p bitquan-consensus test_emergency
```

## 📊 Monitoring Checklist

- [ ] Block propagation times normal
- [ ] Orphan rate < 5%
- [ ] No validation failures
- [ ] Peer connectivity stable
- [ ] Emergency system status normal

## 🚨 Alert Levels

- **🔴 Critical**: Multiple validation failures
- **🟡 High**: Orphan rate spike >5%
- **🟠 Medium**: Peer connectivity issues >10%
- **🟢 Low**: Isolated issues

---

**Remember**: Document all actions, communicate with team, and prioritize network stability.