# BQIP-0003: Wallet & Ecosystem Standards (Extended)

```
BQIP: 0003
Title: Wallet & Ecosystem Standards (Extended Specification)
Author: BitQuan Maintainers
Status: Draft
Type: Standards Track
Created: 2026-03-17
Supersedes: BQIP-0003-wallet.md
```

## Abstract

This document provides extended specifications for BitQuan wallet ecosystem, focusing on practical implementation details for PQC PSBT, multi-signature flows with Dilithium5, address format specifications, and SDK design patterns.

---

## Table of Contents

1. [PQC PSBT (Post-Quantum Partially Signed Bitcoin Transactions)](#1-pqc-psbt)
2. [Address Format Specification](#2-address-format-specification)
3. [Multi-Signature Flow with Dilithium5](#3-multi-signature-flow-with-dilithium5)
4. [SDK Design Patterns](#4-sdk-design-patterns)
5. [Hardware Wallet Integration](#5-hardware-wallet-integration)
6. [Security Considerations](#6-security-considerations)

---

## 1. PQC PSBT

### 1.1 Overview

PQ-PSBT adapts Bitcoin's BIP-174 (Partially Signed Bitcoin Transactions) for post-quantum signatures. The key differences from standard PSBT are:

| Aspect | Bitcoin PSBT | BitQuan PQ-PSBT |
|--------|--------------|-----------------|
| **Signature Algorithm** | ECDSA/Schnorr | Dilithium5 |
| **Public Key Size** | 33 bytes (compressed) | 1,952 bytes |
| **Signature Size** | 64-73 bytes | 4,595 bytes |
| **Per-Input Overhead** | ~100 bytes | ~6,500 bytes |

### 1.2 Binary Format

#### Global Structure

```
+----------------+------------------+---------------------------------+
| Offset         | Size             | Description                     |
+----------------+------------------+---------------------------------+
| 0x00           | 5 bytes          | Magic bytes: "bqpsb"           |
| 0x05           | 1 byte           | Version (0x00)                  |
| 0x06           | 1 byte           | Flags (bit field)               |
| 0x07+          | Variable         | Global key-value map            |
| Variable       | Variable         | Input key-value maps            |
| Variable       | Variable         | Output key-value maps           |
| End            | 1 byte           | Separator: 0x00                 |
+----------------+------------------+---------------------------------+
```

#### Flag Definitions

```
Bit 0 (0x01): Has Dilithium signature
Bit 1 (0x02): Has ECDSA fallback signature (hybrid mode)
Bit 2 (0x04): Requires witness data
Bit 3 (0x08): Contains proprietary data
Bit 4-7: Reserved (must be 0)
```

#### Key-Value Map Format

Each map follows Bitcoin PSBT conventions:

```
<keylength> <key> <valuelength> <value>
```

Keys are prefixed with a type byte:

| Key Type | Name | Value Format |
|----------|------|--------------|
| `0x00` | Separator | None |
| `0x01` | TX Version | 4 bytes (little-endian) |
| `0x02` | Locktime | 4 bytes (little-endian) |
| `0x03` | Fallback Fingerprint | 4 bytes |
| `0x04` | TXID | 32 bytes |
| `0x10` | Dilithium Public Key | 1,952 bytes |
| `0x11` | Dilithium Signature | 4,595 bytes |
| `0x12` | Dilithium Signature Hash | 32 bytes |
| `0x20` | ECDSA Public Key | 33 bytes (fallback) |
| `0x21` | ECDSA Signature | 71-73 bytes (fallback) |
| `0x30` | UTXO Data | Variable |
| `0x31` | Witness UTXO | Variable |
| `0x40` | Redeem Script | Variable |
| `0x41` | Witness Script | Variable |
| `0x50` | Bip32 Derivation Path | Variable |
| `0x60` | Proprietary | Variable |
| `0x70` | Amount | 8 bytes (little-endian) |

### 1.3 PQ-PSBT Transaction Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                        PQ-PSBT Transaction Flow                      │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. CREATION                                                         │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐          │
│  │ Creator │───▶│ Add     │───▶│ Add     │───▶│ Unsigned│          │
│  │         │    │ Inputs  │    │ Outputs │    │ PQ-PSBT │          │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘          │
│                                                                      │
│  2. SIGNING (can be distributed)                                    │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐          │
│  │ Signer  │───▶│ Add     │───▶│ Add     │───▶│ Partial │          │
│  │ 1       │    │ PubKey  │    │ Sig     │    │ PQ-PSBT │          │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘          │
│       │                                                              │
│       ▼       (repeat for each signer in multisig)                  │
│  ┌─────────┐                                                         │
│  │ Signer  │───▶ ...                                                 │
│  │ N       │                                                         │
│  └─────────┘                                                         │
│                                                                      │
│  3. FINALIZATION                                                     │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐          │
│  │ Combine │───▶│ Verify  │───▶│ Build   │───▶│ Final   │          │
│  │ Sigs    │    │ All     │    │ Script  │    │ TX      │          │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘          │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.4 Implementation Example

```rust
use bq_sdk::{PQPSBT, PSBTInput, PSBTOutput, Network};
use bq_sdk::wallet::SimpleWallet;

// Create unsigned PQ-PSBT
let mut psbt = PQPSBT::builder()
    .network(Network::Mainnet)
    .version(1)
    .locktime(0)
    .add_input(PSBTInput {
        txid: previous_txid,
        vout: 0,
        sequence: 0xFFFFFFFF,
        amount: 10_000_000, // 0.1 BQ
    })?
    .add_output(PSBTOutput {
        address: "bq1qyqsq9q5z5...".to_string(),
        amount: 9_900_000, // 0.099 BQ (0.001 BQ fee)
    })?
    .build()?;

// Sign with wallet
let wallet = SimpleWallet::from_mnemonic(mnemonic, &config)?;
wallet.sign_psbt(&mut psbt)?;

// Finalize and extract transaction
let tx = psbt.finalize()?;

// Broadcast
node.broadcast_transaction(&tx)?;
```

### 1.5 Hybrid Mode (Dilithium + ECDSA)

For transition periods, PQ-PSBT supports hybrid signatures:

```rust
// Create hybrid transaction
let mut psbt = PQPSBT::builder()
    .hybrid_mode(true)  // Enable ECDSA fallback
    .add_input(input)?
    .add_output(output)?
    .build()?;

// Sign with Dilithium (primary)
wallet.sign_dilithium(&mut psbt)?;

// Optionally add ECDSA fallback (for legacy compatibility)
wallet.sign_ecdsa_fallback(&mut psbt)?;
```

---

## 2. Address Format Specification

### 2.1 Bech32m Encoding

BitQuan uses **Bech32m** (BIP-350) for all address types:

| Network | HRP | Example |
|---------|-----|---------|
| Mainnet | `bq` | `bq1qyqsq9q5z5khxv8y2w3...` |
| Testnet | `bqt` | `bqt1qyqsq9q5z5khxv8y2w...` |
| Regtest | `bqr` | `bqr1qyqsq9q5z5khxv8y2w...` |

### 2.2 Address Structure

```
bq1 [witness_version] [pubkey_hash] [checksum]
│   │                  │             │
│   │                  │             └── 6 characters (Bech32m)
│   │                  └── 32 bytes (SHA-256 + RIPEMD-160 or direct)
│   └── 1 byte witness version (0x01 for P2WPKH)
└── Human-readable part
```

### 2.3 Address Types

| Type | Version | Data Length | Description |
|------|---------|-------------|-------------|
| P2WPKH | `0x01` | 20 bytes | Pay-to-Witness-Public-Key-Hash |
| P2WSH | `0x01` | 32 bytes | Pay-to-Witness-Script-Hash |
| PQ-P2WPKH | `0x10` | 32 bytes | Post-Quantum P2WPKH (Dilithium) |
| PQ-P2WSH | `0x11` | 32 bytes | Post-Quantum P2WSH (Dilithium multisig) |

### 2.4 Address Generation

#### Single-Signature (PQ-P2WPKH)

```rust
use bq_crypto::dilithium::Keypair;
use sha2::{Sha256, Digest};
use ripemd::Ripemd160;
use bech32::{Bech32m, Hrp};

fn generate_pq_p2wpkh_address(keypair: &Keypair) -> String {
    // 1. Extract public key (1,952 bytes for Dilithium5)
    let public_key = keypair.public();

    // 2. Hash: SHA-256 -> RIPEMD-160
    let sha256_hash = Sha256::digest(public_key);
    let pkh = Ripemd160::digest(&sha256_hash);

    // 3. Build witness program: version + hash
    let mut data = vec![0x10]; // PQ-P2WPKH version
    data.extend_from_slice(&pkh);

    // 4. Encode with Bech32m
    let hrp = Hrp::parse("bq").expect("valid HRP");
    bech32::encode::<Bech32m>(hrp, &data).expect("encoding succeeds")
}
```

#### Multi-Signature (PQ-P2WSH)

```rust
fn generate_pq_p2wsh_address(public_keys: &[Vec<u8>], required: u8) -> String {
    // 1. Build redeem script: M <pubkey1> <pubkey2> ... <pubkeyN> N OP_CHECKMULTISIG
    let mut script = Vec::new();
    script.push(0x50 + required); // OP_M (e.g., 0x52 for 2-of-3)

    for pk in public_keys {
        script.push(pk.len() as u8);
        script.extend_from_slice(pk);
    }

    script.push(0x50 + public_keys.len() as u8); // OP_N
    script.push(0xae); // OP_CHECKMULTISIG

    // 2. Hash script: SHA-256
    let script_hash = Sha256::digest(&script);

    // 3. Build witness program
    let mut data = vec![0x11]; // PQ-P2WSH version
    data.extend_from_slice(&script_hash);

    // 4. Encode with Bech32m
    let hrp = Hrp::parse("bq").expect("valid HRP");
    bech32::encode::<Bech32m>(hrp, &data).expect("encoding succeeds")
}
```

### 2.5 Address Validation

```rust
use crate::address::{AddressNetwork, AddressInfo};

pub fn validate_address(address: &str) -> Result<AddressInfo, AddressError> {
    let trimmed = address.trim();

    // 1. Decode Bech32m
    let (hrp, data) = bech32::decode(trimmed)
        .map_err(|e| AddressError::InvalidEncoding(e.to_string()))?;

    // 2. Validate HRP
    let network = match hrp.as_str() {
        "bq" => AddressNetwork::Mainnet,
        "bqt" => AddressNetwork::Testnet,
        "bqr" => AddressNetwork::Regtest,
        other => return Err(AddressError::UnknownNetwork(other.to_string())),
    };

    // 3. Validate witness version
    if data.is_empty() {
        return Err(AddressError::MissingWitnessData);
    }

    let version = data[0];
    if version != 0x01 && version != 0x10 && version != 0x11 {
        return Err(AddressError::UnsupportedVersion(version));
    }

    // 4. Validate payload length
    let payload_len = data.len() - 1;
    match version {
        0x01 if payload_len != 20 && payload_len != 32 =>
            Err(AddressError::InvalidLength(payload_len)),
        0x10 | 0x11 if payload_len != 32 =>
            Err(AddressError::InvalidLength(payload_len)),
        _ => Ok(())
    }?;

    // 5. Extract payload
    let mut payload = [0u8; 32];
    payload[..payload_len].copy_from_slice(&data[1..]);

    Ok(AddressInfo {
        hrp,
        network,
        witness_version: version,
        payload,
    })
}
```

---

## 3. Multi-Signature Flow with Dilithium5

### 3.1 M-of-N Multisig Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                    2-of-3 Multisig Example                          │
├────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   Participants:  Alice, Bob, Charlie                                │
│   Required:      2 signatures                                       │
│   Total Keys:    3 Dilithium5 public keys                          │
│                                                                     │
│   ┌──────────┐   ┌──────────┐   ┌──────────┐                      │
│   │  Alice   │   │   Bob    │   │ Charlie  │                      │
│   │ 1,952 B  │   │ 1,952 B  │   │ 1,952 B  │ (pubkey sizes)      │
│   │ 4,595 B  │   │ 4,595 B  │   │ 4,595 B  │ (sig sizes)         │
│   └────┬─────┘   └────┬─────┘   └────┬─────┘                      │
│        │              │              │                              │
│        └──────────────┼──────────────┘                              │
│                       ▼                                              │
│              ┌────────────────┐                                     │
│              │ Redeem Script  │                                     │
│              │ (~5.9 KB)      │                                     │
│              │                │                                     │
│              │ 2 <pk1> <pk2>  │                                     │
│              │ <pk3> 3 CHECK- │                                     │
│              │ MULTISIG       │                                     │
│              └────────┬───────┘                                     │
│                       │                                              │
│                       ▼                                              │
│              ┌────────────────┐                                     │
│              │  PQ-P2WSH      │                                     │
│              │  Address       │                                     │
│              │  bq1p...       │                                     │
│              └────────────────┘                                     │
│                                                                     │
└────────────────────────────────────────────────────────────────────┘
```

### 3.2 Size Analysis

| Component | Size | Notes |
|-----------|------|-------|
| Dilithium5 Public Key | 1,952 bytes | Per signer |
| Dilithium5 Signature | 4,595 bytes | Per signature |
| 2-of-3 Redeem Script | ~5,900 bytes | 2 + 3×1,952 + 2 + 1 |
| 2-of-3 Transaction Input | ~14,500 bytes | Script + 2 signatures |
| **Total 2-of-3 TX** | ~15-20 KB | vs ~500 bytes for Bitcoin |

### 3.3 Multisig Transaction Flow

```rust
use bq_sdk::wallet::{MultisigWallet, MultisigConfig, PartialSignature};
use bq_sdk::psbt::PQPSBT;

// Step 1: Create multisig wallet (coordinator)
let config = MultisigConfig {
    required_sigs: 2,
    total_signers: 3,
    public_keys: vec![
        alice_pubkey.to_hex(),
        bob_pubkey.to_hex(),
        charlie_pubkey.to_hex(),
    ],
    label: Some("Treasury Wallet".to_string()),
    created_at: current_timestamp(),
};

let multisig = MultisigWallet::new(config)?;

// Step 2: Create pending transaction
let pending_tx = multisig.create_pending_tx(
    inputs,    // UTXOs to spend
    outputs,   // Recipients
)?;

// Step 3: Distribute for signing (can be offline)
// Alice signs
let alice_sig = alice_wallet.sign_partial(&pending_tx)?;
pending_tx.add_signature(alice_sig)?;

// Bob signs
let bob_sig = bob_wallet.sign_partial(&pending_tx)?;
pending_tx.add_signature(bob_sig)?;

// Step 4: Finalize (needs 2 signatures)
if pending_tx.signature_count() >= 2 {
    let final_tx = multisig.finalize(pending_tx)?;
    node.broadcast(&final_tx)?;
}
```

### 3.4 Coordination Protocol

```
┌──────────────────────────────────────────────────────────────────┐
│                   Multisig Coordination Flow                      │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  Coordinator                    Signers                           │
│  ───────────                    ───────                           │
│       │                                                           │
│       │  1. Create PQ-PSBT                                        │
│       │  ─────────────────▶                                       │
│       │     (contains unsigned TX,                               │
│       │      redeem script,                                       │
│       │      amount info)                                         │
│       │                                                           │
│       │                    2. Verify & Sign                       │
│       │  ◀─────────────────                                       │
│       │     (return partial signature)                            │
│       │                                                           │
│       │  [repeat for each required signer]                        │
│       │                                                           │
│       │  3. Collect signatures until M reached                    │
│       │  ─────────────────                                        │
│       │                                                           │
│       │  4. Combine & Finalize                                    │
│       │  ─────────────────                                        │
│       │                                                           │
│       │  5. Broadcast                                             │
│       │  ─────────────────▶  Network                              │
│       │                                                           │
└──────────────────────────────────────────────────────────────────┘
```

### 3.5 Implementation

```rust
// crates/wallet/src/multisig.rs (extended)

impl MultisigWallet {
    /// Create a new pending multisig transaction
    pub fn create_pending_tx(
        &self,
        inputs: Vec<TxInput>,
        outputs: Vec<TxOutput>,
    ) -> Result<PendingMultisigTx, MultisigError> {
        // Validate inputs belong to this multisig
        for input in &inputs {
            self.verify_input_ownership(input)?;
        }

        // Build unsigned transaction
        let tx_data = self.build_unsigned_tx(&inputs, &outputs)?;
        let tx_id = self.compute_tx_id(&tx_data);

        Ok(PendingMultisigTx {
            tx_id,
            tx_data: tx_data.to_hex(),
            config: self.config.clone(),
            signatures: Vec::new(),
            created_at: current_timestamp(),
        })
    }

    /// Add a partial signature to pending transaction
    pub fn add_signature(
        &mut self,
        pending: &mut PendingMultisigTx,
        partial: PartialSignature,
    ) -> Result<(), MultisigError> {
        // Verify signer is authorized
        if !self.config.public_keys.contains(&partial.public_key) {
            return Err(MultisigError::UnknownSigner(partial.public_key));
        }

        // Check for duplicate
        if pending.signatures.iter().any(|s| s.public_key == partial.public_key) {
            return Err(MultisigError::DuplicateSignature(partial.public_key));
        }

        // Verify signature is valid
        self.verify_partial_signature(&pending.tx_data, &partial)?;

        pending.signatures.push(partial);
        Ok(())
    }

    /// Finalize transaction with collected signatures
    pub fn finalize(
        &self,
        pending: PendingMultisigTx,
    ) -> Result<Transaction, MultisigError> {
        // Check we have enough signatures
        if pending.signatures.len() < self.config.required_sigs as usize {
            return Err(MultisigError::InsufficientSignatures {
                required: self.config.required_sigs,
                actual: pending.signatures.len() as u8,
            });
        }

        // Build final scriptSig with signatures + redeem script
        let script_sig = self.build_script_sig(&pending.signatures)?;

        // Construct final transaction
        let tx = self.construct_final_tx(&pending.tx_data, script_sig)?;

        // Verify final transaction
        self.verify_final_transaction(&tx)?;

        Ok(tx)
    }
}
```

---

## 4. SDK Design Patterns

### 4.1 Core Traits

```rust
// crates/bq-sdk/src/wallet/mod.rs

/// Core wallet operations trait
pub trait Wallet: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Generate a new wallet with random seed
    fn generate(config: &WalletConfig) -> Result<Self, Self::Error>;

    /// Restore wallet from mnemonic phrase
    fn from_mnemonic(mnemonic: &str, config: &WalletConfig) -> Result<Self, Self::Error>;

    /// Restore wallet from seed bytes
    fn from_seed(seed: &[u8; 64], config: &WalletConfig) -> Result<Self, Self::Error>;

    /// Get address at derivation path
    fn get_address(&self, path: &DerivationPath) -> Result<Address, Self::Error>;

    /// Get public key at derivation path
    fn get_public_key(&self, path: &DerivationPath) -> Result<Vec<u8>, Self::Error>;

    /// Sign a PQ-PSBT
    fn sign_psbt(&mut self, psbt: &mut PQPSBT) -> Result<(), Self::Error>;

    /// Sign a message (for authentication)
    fn sign_message(&mut self, message: &[u8], path: &DerivationPath)
        -> Result<Vec<u8>, Self::Error>;

    /// Verify a signature
    fn verify_signature(
        public_key: &[u8],
        message: &[u8],
        signature: &[u8]
    ) -> Result<bool, Self::Error>;

    /// Export wallet for backup
    fn export(&self) -> Result<WalletBackup, Self::Error>;
}

/// HD wallet derivation support
pub trait HDWallet: Wallet {
    /// Get master fingerprint for BIP32 identification
    fn master_fingerprint(&self) -> [u8; 4];

    /// Derive a child key at the specified path
    fn derive(&self, path: &DerivationPath) -> Result<DerivedKey, Self::Error>;

    /// List all addresses up to gap limit
    fn list_addresses(&self, account: u32) -> Result<Vec<(DerivationPath, Address)>, Self::Error>;
}
```

### 4.2 Configuration Pattern

```rust
// crates/bq-sdk/src/wallet/config.rs

/// Wallet configuration builder
#[derive(Clone, Debug)]
pub struct WalletConfig {
    /// Target network
    pub network: Network,

    /// Signature algorithm configuration
    pub signatures: SignatureConfig,

    /// Key derivation configuration
    pub derivation: DerivationConfig,

    /// Security settings
    pub security: SecurityConfig,

    /// Performance settings
    pub performance: PerformanceConfig,
}

impl WalletConfig {
    /// Create config for mainnet with defaults
    pub fn mainnet() -> Self {
        Self {
            network: Network::Mainnet,
            ..Self::default()
        }
    }

    /// Create config for testnet
    pub fn testnet() -> Self {
        Self {
            network: Network::Testnet,
            ..Self::default()
        }
    }

    /// Server configuration: maximum security, no caching
    pub fn server() -> Self {
        Self {
            security: SecurityConfig {
                memory_locking: true,
                cache_enabled: false,
                ..SecurityConfig::default()
            },
            performance: PerformanceConfig {
                kdf_profile: KDFProfile::Tight,
                ..PerformanceConfig::default()
            },
            ..Self::default()
        }
    }

    /// Mobile configuration: battery-optimized
    pub fn mobile() -> Self {
        Self {
            security: SecurityConfig {
                cache_timeout: Duration::from_secs(300),
                ..SecurityConfig::default()
            },
            performance: PerformanceConfig {
                kdf_profile: KDFProfile::Mobile,
                ..PerformanceConfig::default()
            },
            ..Self::default()
        }
    }
}

/// Signature algorithm configuration
#[derive(Clone, Debug)]
pub struct SignatureConfig {
    /// Primary signature algorithm
    pub primary: SignatureAlgorithm,

    /// Enable ECDSA fallback (hybrid mode)
    pub hybrid_mode: bool,

    /// Require both signatures in hybrid mode
    pub require_both: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    /// Dilithium5 (primary, quantum-resistant)
    Dilithium5,
    /// Dilithium3 (faster, lower security level)
    Dilithium3,
}

/// Key derivation configuration
#[derive(Clone, Debug)]
pub struct DerivationConfig {
    /// BIP32 derivation path template
    pub path_template: DerivationPath,

    /// Account gap limit (how many empty accounts before stopping)
    pub account_gap_limit: u32,

    /// Address gap limit (how many empty addresses before stopping)
    pub address_gap_limit: u32,
}

/// Security configuration
#[derive(Clone, Debug)]
pub struct SecurityConfig {
    /// Lock private keys in memory (prevent swap)
    pub memory_locking: bool,

    /// Enable decrypted key caching
    pub cache_enabled: bool,

    /// Cache timeout duration
    pub cache_timeout: Duration,

    /// Require passphrase for every operation
    pub require_passphrase: bool,
}

/// Performance configuration
#[derive(Clone, Debug)]
pub struct PerformanceConfig {
    /// KDF computation profile
    pub kdf_profile: KDFProfile,

    /// Parallelism for Argon2id
    pub parallelism: u32,

    /// Memory cost for Argon2id (KiB)
    pub memory_cost: u32,
}

#[derive(Clone, Copy, Debug)]
pub enum KDFProfile {
    /// Maximum security: 256 MiB, 4 iterations
    Tight,
    /// Balanced: 128 MiB, 3 iterations
    Medium,
    /// Older hardware: 64 MiB, 2 iterations
    Light,
    /// Battery-constrained: 32 MiB, 2 iterations
    Mobile,
    /// Auto-detect based on hardware
    Adaptive,
}
```

### 4.3 Builder Pattern for Transactions

```rust
// crates/bq-sdk/src/psbt/builder.rs

/// PQ-PSBT builder with fluent API
pub struct PQPSBTBuilder {
    network: Network,
    version: u32,
    locktime: u32,
    inputs: Vec<PSBTInput>,
    outputs: Vec<PSBTOutput>,
    hybrid_mode: bool,
}

impl PQPSBTBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            network: Network::Mainnet,
            version: 1,
            locktime: 0,
            inputs: Vec::new(),
            outputs: Vec::new(),
            hybrid_mode: false,
        }
    }

    /// Set network
    pub fn network(mut self, network: Network) -> Self {
        self.network = network;
        self
    }

    /// Set transaction version
    pub fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// Set locktime
    pub fn locktime(mut self, locktime: u32) -> Self {
        self.locktime = locktime;
        self
    }

    /// Add input
    pub fn add_input(mut self, input: PSBTInput) -> Result<Self, PSBTError> {
        self.validate_input(&input)?;
        self.inputs.push(input);
        Ok(self)
    }

    /// Add output
    pub fn add_output(mut self, output: PSBTOutput) -> Result<Self, PSBTError> {
        self.validate_output(&output)?;
        self.outputs.push(output);
        Ok(self)
    }

    /// Enable hybrid mode (Dilithium + ECDSA)
    pub fn hybrid_mode(mut self, enabled: bool) -> Self {
        self.hybrid_mode = enabled;
        self
    }

    /// Build the PQ-PSBT
    pub fn build(self) -> Result<PQPSBT, PSBTError> {
        // Validate we have inputs and outputs
        if self.inputs.is_empty() {
            return Err(PSBTError::NoInputs);
        }
        if self.outputs.is_empty() {
            return Err(PSBTError::NoOutputs);
        }

        // Validate input amounts >= output amounts
        let input_sum: u64 = self.inputs.iter().map(|i| i.amount).sum();
        let output_sum: u64 = self.outputs.iter().map(|o| o.amount).sum();
        if input_sum < output_sum {
            return Err(PSBTError::InsufficientFunds {
                available: input_sum,
                required: output_sum,
            });
        }

        Ok(PQPSBT {
            network: self.network,
            version: self.version,
            locktime: self.locktime,
            inputs: self.inputs,
            outputs: self.outputs,
            signatures: Vec::new(),
            hybrid_mode: self.hybrid_mode,
        })
    }
}
```

### 4.4 Error Handling Pattern

```rust
// crates/bq-sdk/src/error.rs

/// Comprehensive SDK error type
#[derive(Debug, thiserror::Error)]
pub enum SDKError {
    // Address errors
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Wrong network: expected {expected}, got {actual}")]
    WrongNetwork { expected: String, actual: String },

    // PSBT errors
    #[error("PSBT error: {0}")]
    PSBT(String),

    #[error("Missing signature for input {index}")]
    MissingSignature { index: usize },

    // Wallet errors
    #[error("Wallet error: {0}")]
    Wallet(String),

    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),

    // Crypto errors
    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Signature verification failed")]
    InvalidSignature,

    // I/O errors
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),

    // Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Result alias for SDK operations
pub type Result<T> = std::result::Result<T, SDKError>;
```

---

## 5. Hardware Wallet Integration

### 5.1 Communication Protocol

BitQuan hardware wallets communicate via USB HID with a standardized protocol:

```
┌──────────────────────────────────────────────────────────────┐
│                   Hardware Wallet Protocol                    │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  Message Structure:                                           │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ Channel (2) │ Tag (1) │ Length (2) │ Payload (variable) ││
│  └─────────────────────────────────────────────────────────┘│
│                                                               │
│  Commands:                                                    │
│  ──────────                                                   │
│  0x01: INITIALIZE      - Get device info                    │
│  0x02: GET_PUBLIC_KEY  - Derive and return public key       │
│  0x03: SIGN_TX         - Sign transaction                    │
│  0x04: SIGN_MESSAGE    - Sign arbitrary message              │
│  0x05: GET_ADDRESS     - Display and return address          │
│  0x06: BACKUP          - Initiate backup flow               │
│  0x07: RESTORE         - Initiate recovery flow             │
│  0x08: WIPE            - Factory reset (requires confirm)   │
│                                                               │
│  Responses:                                                   │
│  ──────────                                                   │
│  0x80: SUCCESS         - Operation completed                 │
│  0x81: FAILURE         - Operation failed (error in payload)│
│  0x82: BUTTON_REQUEST  - Waiting for user button press      │
│  0x83: PIN_REQUEST     - Waiting for PIN entry              │
│  0x84: PASSPHRASE_REQ  - Waiting for passphrase entry       │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### 5.2 Device Capabilities

```rust
// crates/bq-sdk/src/hardware/protocol.rs

/// Hardware wallet capabilities
#[derive(Clone, Debug)]
pub struct DeviceCapabilities {
    /// Supports Dilithium5 signatures
    pub dilithium5: bool,

    /// Supports ECDSA fallback
    pub ecdsa: bool,

    /// Has display for address verification
    pub display: bool,

    /// Has physical buttons for confirmation
    pub buttons: bool,

    /// Has touchscreen
    pub touchscreen: bool,

    /// Maximum message size (bytes)
    pub max_message_size: usize,

    /// Firmware version string
    pub firmware_version: String,

    /// Device model
    pub model: String,

    /// Supports multisig
    pub multisig: bool,

    /// Maximum N for M-of-N multisig
    pub max_multisig_n: u8,
}

/// Initialize connection and get device info
pub async fn initialize(device: &mut HardwareDevice) -> Result<DeviceInfo, HardwareError> {
    let response = device.send_command(Command::Initialize).await?;

    Ok(DeviceInfo {
        capabilities: DeviceCapabilities {
            dilithium5: response.get_bool("dilithium5")?,
            ecdsa: response.get_bool("ecdsa")?,
            display: response.get_bool("display")?,
            buttons: response.get_bool("buttons")?,
            touchscreen: response.get_bool("touchscreen")?,
            max_message_size: response.get_u32("max_message_size")? as usize,
            firmware_version: response.get_string("firmware_version")?,
            model: response.get_string("model")?,
            multisig: response.get_bool("multisig")?,
            max_multisig_n: response.get_u8("max_multisig_n")?,
        },
        device_id: response.get_bytes("device_id")?,
        initialized: response.get_bool("initialized")?,
    })
}
```

### 5.3 Signing Flow

```rust
/// Sign transaction on hardware wallet
pub async fn sign_transaction(
    device: &mut HardwareDevice,
    psbt: &PQPSBT,
) -> Result<Vec<PartialSignature>, HardwareError> {
    // 1. Begin signing session
    device.send_command(Command::BeginSign {
        tx_version: psbt.version,
        num_inputs: psbt.inputs.len() as u8,
        num_outputs: psbt.outputs.len() as u8,
    }).await?;

    // 2. Send each input
    for (i, input) in psbt.inputs.iter().enumerate() {
        device.send_command(Command::TxInput {
            index: i as u8,
            txid: input.txid,
            vout: input.vout,
            amount: input.amount,
        }).await?;

        // Wait for user confirmation
        device.wait_for_confirmation().await?;
    }

    // 3. Send each output
    for (i, output) in psbt.outputs.iter().enumerate() {
        device.send_command(Command::TxOutput {
            index: i as u8,
            address: output.address.clone(),
            amount: output.amount,
        }).await?;

        // Wait for user confirmation
        device.wait_for_confirmation().await?;
    }

    // 4. Request signature for each input
    let mut signatures = Vec::new();
    for (i, _input) in psbt.inputs.iter().enumerate() {
        let response = device.send_command(Command::SignInput {
            index: i as u8,
        }).await?;

        signatures.push(PartialSignature {
            input_index: i,
            public_key: response.get_bytes("public_key")?,
            signature: response.get_bytes("signature")?,
        });
    }

    // 5. End signing session
    device.send_command(Command::EndSign).await?;

    Ok(signatures)
}
```

---

## 6. Security Considerations

### 6.1 Key Storage

| Aspect | Requirement | Implementation |
|--------|-------------|----------------|
| **Memory Locking** | Keys must not swap to disk | `mlock()` on key buffers |
| **Zeroization** | Clear keys on drop | `Zeroize` trait |
| **Encryption at Rest** | Keys encrypted with KDF | AES-256-GCM + Argon2id |
| **Cache Isolation** | Per-password cache entries | Hash-based partitioning |

### 6.2 Signature Security

```rust
/// Constant-time signature verification
pub fn verify_dilithium_signature(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<bool, CryptoError> {
    // Use constant-time comparison
    let expected = dilithium5_sign(public_key, message)?;
    let valid = constant_time_compare(&expected, signature);

    Ok(valid)
}
```

### 6.3 Address Security

- **Bech32m**: Strong error detection (1 in 10^9 random errors detected)
- **Network Separation**: Different HRPs prevent mainnet/testnet confusion
- **Version Validation**: Reject unknown witness versions
- **Length Validation**: Strict payload length requirements

### 6.4 PSBT Security

| Threat | Mitigation |
|--------|------------|
| **Signature Replay** | Include TXID in signature hash |
| **Input Substitution** | Sign specific outpoints |
| **Output Tampering** | Sign complete output vector |
| **Fee Manipulation** | Explicit fee calculation in PSBT |

---

## References

- [BIP-174] Partially Signed Bitcoin Transaction Format
- [BIP-32] Hierarchical Deterministic Wallets
- [BIP-39] Mnemonic Code for Generating Deterministic Keys
- [BIP-173] Base32 Address Format for Native v0-16 Witness Outputs
- [BIP-350] Bech32m Format for v1+ Witness Addresses
- [NIST FIPS 205] CRYSTALS-Dilithium Digital Signature Standard

---

## Copyright

This document is placed in the public domain.

---

*Last Updated: 2026-03-17*
*Author: BitQuan Core Team*
