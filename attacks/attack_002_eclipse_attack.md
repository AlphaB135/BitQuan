# Attack Report #002: Eclipse Attack & Subnet Monopolization

**Date**: 2026-08-15 10:56:00 UTC  
**Attack Type**: P2P Network / Eclipse Attack  
**Severity**: High  
**Status**: Blocked (Mitigated & Verified)  
**Target Component**: `crates/network/src/peer.rs`, `crates/network/src/connection_manager.rs`

---

## 1. Attack Objective & Vector Description

The objective of an Eclipse Attack is to isolate a victim node from the legitimate blockchain network by monopolizing all of its inbound and outbound P2P peer connections.

### Attack Steps:
1. Attacker controls a botnet or VPS subnet (e.g., `198.51.100.0/24`).
2. Attacker launches 50+ BitQuan node instances across the same `/24` CIDR block.
3. Attacker floods the victim node with incoming TCP connection requests on P2P port `19444`.
4. If successful, victim connects only to attacker-controlled nodes, allowing the attacker to feed stale headers, obscure valid transactions, and mount double-spend attacks with 0% risk of detection.

---

## 2. Steps to Reproduce (PoC)

```bash
# Attacker script attempting to occupy all 25 peer slots from single /24 subnet
TARGET_IP="127.0.0.1"
TARGET_PORT="19444"

for i in {1..50}; do
  (
    # Spawn inbound connection with unique Noise static identity from local IP
    ./target/release/bitquan-node run \
      --p2p-bind 127.0.0.1:$((20000 + i)) \
      --peer "$TARGET_IP:$TARGET_PORT" \
      --datadir "/tmp/eclipse_test_$i" &
  )
done
```

---

## 3. Observed Behavior & Red Team Findings

1. **Subnet Diversity Enforcement**:
   - `PeerManager::add_peer_inbound` inspects the connecting peer's IP address.
   - For IPv4 addresses, `Self::get_subnet_24(&addr)` groups peers by their `/24` prefix.
   - If the number of connected peers in that `/24` bucket reaches `max_peers_per_subnet` (default: 2), subsequent inbound connections from that subnet are immediately dropped with error:
     ```text
     too many peers from same subnet: 2 (max: 2)
     ```
2. **Anchor Peers Protection**:
   - Hardcoded / configured anchor peers (`anchor_peers`) bypass eviction and remain persistent out-of-band links to prevent total isolation during node restart.
3. **Public Key Uniqueness**:
   - Sybil instances reusing the same Noise static key are rejected with:
     ```text
     duplicate peer connection: peer with key <HEX> is already connected
     ```

---

## 4. Impact Assessment

- **Availability**: Unaffected (Node maintains outbound diversity across distinct subnets).
- **Integrity**: Maintained (Target node cannot be tricked into a private isolated chain).
- **Confidentiality**: N/A.

---

## 5. Defense Verification

- Automated test executed: `cargo test --test eclipse_tests`
- Test Output:
  ```text
  running 4 tests
  test test_anchor_peers_config ... ok
  test test_subnet_diversity_enforcement ... ok
  test test_evict_no_peers ... ok
  test test_subnet_stats_empty ... ok
  test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
  ```
- **Red Team Verdict**: Defense is ACTIVE and functioning as intended.
