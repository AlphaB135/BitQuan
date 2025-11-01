# Multi-Signature Wallet Guide

## Overview

BitQuan supports **M-of-N multi-signature** wallets, where **M** signatures are required from a set of **N** public keys to authorize a transaction.

This provides enhanced security by distributing control among multiple parties, preventing single points of failure.

## Features

- ✅ **Flexible configurations**: 2-of-3, 3-of-5, 4-of-7, etc.
- ✅ **Dilithium3 PQC signatures**: Quantum-resistant security
- ✅ **Duplicate prevention**: Each signer can only sign once
- ✅ **Progress tracking**: Monitor signature collection status
- ✅ **Transaction finalization**: Automatic validation before completion
- ✅ **Wallet management**: Manage multiple multisig wallets

## Quick Start

### 1. Create a Multisig Configuration

```rust
use wallet::multisig::{MultisigConfig, MultisigWallet};

// Create a 2-of-3 multisig wallet
let public_keys = vec![
    "alice_dilithium3_pubkey".to_string(),
    "bob_dilithium3_pubkey".to_string(),
    "charlie_dilithium3_pubkey".to_string(),
];

let config = MultisigConfig::new(
    2,  // Required signatures
    public_keys,
    Some("Team Wallet".to_string()),
)?;

println!("Multisig address: {}", config.address());
```

### 2. Create a Pending Transaction

```rust
let wallet = MultisigWallet::new(config);

// Create transaction to be signed
let tx_data = b"Transfer 100 BQ to recipient";
let mut pending = wallet.create_pending_tx(tx_data);

println!("Transaction ID: {}", pending.tx_id);
println!("Signatures needed: {}", pending.signatures_needed());
```

### 3. Collect Signatures

```rust
// Alice signs
let alice_sig = alice_keypair.sign(tx_data)?;
wallet.add_signature(&mut pending, "alice_pubkey", &alice_sig)?;

println!("Progress: {:.0}%", pending.progress_percentage());

// Bob signs
let bob_sig = bob_keypair.sign(tx_data)?;
wallet.add_signature(&mut pending, "bob_pubkey", &bob_sig)?;

// Check if complete
if pending.is_complete() {
    println!("✓ Transaction ready to finalize!");
}
```

### 4. Finalize Transaction

```rust
// Finalize when enough signatures collected
let finalized = wallet.finalize_transaction(&pending)?;

// Verify all signatures
finalized.verify()?;

println!("✓ Transaction finalized and verified!");
```

## Common Configurations

### Single Signature (1-of-1)
- **Use case**: Personal wallet with backup key
- **Security**: Basic
- **Example**: `MultisigConfig::new(1, vec![pubkey], None)?`

### Dual Control (2-of-2)
- **Use case**: Business partnerships, joint accounts
- **Security**: Both parties must agree
- **Example**: `MultisigConfig::new(2, vec![pk1, pk2], None)?`

### Standard Multisig (2-of-3)
- **Use case**: Corporate treasury, DAOs, escrow
- **Security**: Any 2 of 3 signers
- **Example**: `MultisigConfig::new(2, vec![pk1, pk2, pk3], None)?`

### High Security (3-of-5)
- **Use case**: Large organizations, high-value custody
- **Security**: Majority approval required
- **Example**: `MultisigConfig::new(3, vec![pk1, pk2, pk3, pk4, pk5], None)?`

### Enterprise (4-of-7)
- **Use case**: Enterprise treasury, board approval
- **Security**: Super-majority required
- **Example**: `MultisigConfig::new(4, vec![...7 keys], None)?`

## Advanced Usage

### Using MultisigWalletManager

```rust
use wallet::multisig::MultisigWalletManager;

let mut manager = MultisigWalletManager::new();

// Add wallets
manager.add_wallet(wallet1);
manager.add_wallet(wallet2);

// Track pending transactions
manager.add_pending_tx(pending_tx);

// List all wallets
let addresses = manager.list_addresses();

// List incomplete transactions
let incomplete = manager.list_incomplete_txs();

// Get specific wallet
if let Some(wallet) = manager.get_wallet("bqms1abc...") {
    println!("Found wallet: {}", wallet.config().label.as_ref().unwrap());
}
```

### Progress Tracking

```rust
let pending = wallet.create_pending_tx(tx_data);

println!("Signature count: {}/{}", 
    pending.signature_count(), 
    pending.config.required_sigs
);

println!("Progress: {:.1}%", pending.progress_percentage());

println!("Pending signers: {:?}", pending.pending_signers());

println!("Complete: {}", pending.is_complete());
```

### Error Handling

```rust
use wallet::multisig::MultisigError;

match wallet.add_signature(&mut pending, pubkey, sig) {
    Ok(()) => println!("✓ Signature added"),
    Err(MultisigError::DuplicateSignature(pk)) => {
        println!("Already signed by: {}", pk);
    }
    Err(MultisigError::UnknownSigner(pk)) => {
        println!("Not authorized: {}", pk);
    }
    Err(MultisigError::AlreadyComplete) => {
        println!("Transaction already has enough signatures");
    }
    Err(e) => println!("Error: {}", e),
}
```

## Security Best Practices

### 1. Key Distribution
- **Never** store all keys in one location
- Use hardware wallets for cold storage keys
- Distribute keys geographically
- Use different devices for each signer

### 2. Configuration Selection
- **2-of-3**: Good balance of security and convenience
- **3-of-5**: Better security, allows 2 key loss
- **4-of-7**: Enterprise-grade, allows 3 key loss
- Avoid **1-of-N** (defeats multisig purpose)
- Avoid **N-of-N** (too risky, any loss = funds locked)

### 3. Signer Identity Verification
- Verify public keys through multiple channels
- Use checksums to prevent typos
- Maintain a secure registry of authorized signers
- Regular audits of signer list

### 4. Transaction Verification
- All signers should verify transaction data before signing
- Use secure out-of-band communication
- Implement spending limits for different approval levels
- Keep audit logs of all signature events

### 5. Key Rotation
- Periodically rotate signers (quarterly/annually)
- Have emergency backup procedures
- Document key recovery processes
- Test recovery procedures regularly

## Integration with BitQuan

### Creating Multisig Addresses

```rust
// Generate address from config
let address = config.address();
// Format: bqms1{hash} (BitQuan MultiSig)
```

### Transaction Signing Flow

```rust
// 1. Create transaction
let tx = Transaction::new(inputs, outputs);
let tx_bytes = tx.serialize();

// 2. Create pending multisig transaction
let mut pending = wallet.create_pending_tx(&tx_bytes);

// 3. Each signer signs independently
for (signer, keypair) in signers {
    let signature = keypair.sign(&tx_bytes)?;
    wallet.add_signature(&mut pending, &signer.pubkey, &signature)?;
}

// 4. Finalize when enough signatures
if pending.is_complete() {
    let finalized = wallet.finalize_transaction(&pending)?;
    // Broadcast to network
}
```

### Storage and Persistence

```rust
use serde_json;

// Save pending transaction to file
let json = serde_json::to_string_pretty(&pending)?;
std::fs::write("pending_tx.json", json)?;

// Load pending transaction
let json = std::fs::read_to_string("pending_tx.json")?;
let pending: PendingMultisigTx = serde_json::from_str(&json)?;
```

## API Reference

### MultisigConfig

```rust
pub struct MultisigConfig {
    pub required_sigs: u8,
    pub total_signers: u8,
    pub public_keys: Vec<String>,
    pub label: Option<String>,
    pub created_at: u64,
}

impl MultisigConfig {
    pub fn new(required_sigs: u8, public_keys: Vec<String>, label: Option<String>) -> Result<Self>;
    pub fn address(&self) -> String;
    pub fn is_signer(&self, public_key: &str) -> bool;
    pub fn config_type(&self) -> String;
}
```

### MultisigWallet

```rust
pub struct MultisigWallet {
    config: MultisigConfig,
}

impl MultisigWallet {
    pub fn new(config: MultisigConfig) -> Self;
    pub fn config(&self) -> &MultisigConfig;
    pub fn create_pending_tx(&self, tx_data: &[u8]) -> PendingMultisigTx;
    pub fn add_signature(&self, pending: &mut PendingMultisigTx, public_key: &str, signature: &[u8]) -> Result<()>;
    pub fn verify_signatures(&self, pending: &PendingMultisigTx) -> Result<()>;
    pub fn finalize_transaction(&self, pending: &PendingMultisigTx) -> Result<FinalizedMultisigTx>;
}
```

### PendingMultisigTx

```rust
pub struct PendingMultisigTx {
    pub tx_id: String,
    pub tx_data: String,
    pub config: MultisigConfig,
    pub signatures: Vec<PartialSignature>,
    pub created_at: u64,
}

impl PendingMultisigTx {
    pub fn is_complete(&self) -> bool;
    pub fn has_signature_from(&self, public_key: &str) -> bool;
    pub fn signature_count(&self) -> u8;
    pub fn signatures_needed(&self) -> u8;
    pub fn pending_signers(&self) -> Vec<String>;
    pub fn progress_percentage(&self) -> f64;
}
```

## Testing

Run the test suite:

```bash
cargo test --package wallet --lib multisig
```

Run the demo:

```bash
cargo run --example multisig_demo
```

## Examples

See [`examples/multisig_demo.rs`](../crates/wallet/examples/multisig_demo.rs) for a complete working example.

## Future Enhancements

- [ ] Time-locked transactions
- [ ] Spending limits per signer
- [ ] Weighted signatures (different signers have different voting power)
- [ ] Emergency recovery procedures
- [ ] Integration with hardware wallets
- [ ] Mobile app support
- [ ] Web interface for signature collection

## Support

For questions or issues:
- Open an issue on GitHub
- Check the [SECURITY.md](../SECURITY.md) for security concerns
- See [CONTRIBUTING.md](../CONTRIBUTING.md) for development guidelines

---

**Status**: ✅ Production Ready (requires integration with Dilithium3 keypair signing)

**Last Updated**: 2025-11-01
