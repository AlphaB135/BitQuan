# BQIP-0003: Wallet & Ecosystem Standards

```
BQIP: 0003
Title: Wallet & Ecosystem Standards
Author: BitQuan Maintainers
Status: Draft
Type: Standards Track
Created: 2025-11-15
```

## Abstract

This BQIP defines comprehensive standards for BitQuan wallet ecosystem, including post-quantum PSBT (PQ-PSBT), address generation, mnemonic phrase support with quantum-resistant features, and wallet SDK specifications.

## Motivation

Current wallet ecosystem lacks:
- Standardized post-quantum transaction format
- Consistent address generation across implementations
- Quantum-resistant mnemonic phrase support
- Unified SDK interface for developers
- Hardware wallet integration standards

## Specification

### 1. Post-Quantum PSBT (PQ-PSBT)

#### 1.1 Format Definition

PQ-PSBT extends Bitcoin PSBT with Dilithium signature support:

```
0x70: PQ_PSBT_MAGIC (4 bytes)
0x71: VERSION (1 byte) - Currently 0x00
0x72: FLAGS (1 byte) - Bit field for features
0x73: GLOBAL_DATA (variable) - Key-value pairs
0x74: INPUT_COUNT (CompactSize)
0x75: INPUTS (variable) - Array of input data
0x76: OUTPUT_COUNT (CompactSize)
0x77: OUTPUTS (variable) - Array of output data
```

#### 1.2 Global Keys

| Key | Type | Description |
|-----|------|-------------|
| 0x01 | CompactSize | Transaction version |
| 0x02 | 32 bytes | Fallback fingerprint (for mixed signatures) |
| 0x03 | CompactSize | Locktime |
| 0x80 | Variable | Proprietary data (prefix + key) |

#### 1.3 Input Keys

| Key | Type | Description |
|-----|------|-------------|
| 0x01 | 32 bytes | Previous TXID |
| 0x02 | CompactSize | Previous output index |
| 0x03 | 8 bytes | Sequence |
| 0x04 | Variable | ScriptSig |
| 0x05 | 1952 bytes | Dilithium public key |
| 0x06 | 3293 bytes | Dilithium signature |
| 0x07 | 32 bytes | ECDSA fallback signature |
| 0x80 | Variable | Proprietary data |

#### 1.4 Output Keys

| Key | Type | Description |
|-----|------|-------------|
| 0x01 | 8 bytes | Amount |
| 0x02 | Variable | ScriptPubkey |
| 0x80 | Variable | Proprietary data |

#### 1.5 Signature Algorithm Flags

```
Bit 0: Has Dilithium signature
Bit 1: Has ECDSA fallback signature
Bit 2: Requires both signatures (hybrid mode)
Bit 3-7: Reserved for future algorithms
```

### 2. Address Generation Standards

#### 2.1 Bech32m Format

BitQuan uses Bech32m with human-readable parts:

- Mainnet: `bq`
- Testnet: `tbq`
- Regtest: `rbq`

#### 2.2 Address Types

| Type | Version | Prefix | Description |
|------|---------|--------|-------------|
| P2PKH | 0x00 | bq1... | Pay-to-Public-Key-Hash |
| P2SH | 0x01 | bq1... | Pay-to-Script-Hash |
| P2WPKH | 0x02 | bq1... | Native SegWit |
| P2WSH | 0x03 | bq1... | Native SegWit Script |
| PQ-P2PKH | 0x10 | bq1... | Post-Quantum P2PKH |
| PQ-P2WSH | 0x11 | bq1... | Post-Quantum SegWit |

#### 2.3 Address Generation Process

```rust
// Post-Quantum P2PKH address generation
fn generate_pq_p2pkh_address(public_key: &[u8; 1952]) -> String {
    // 1. Hash Dilithium public key with SHA-256
    let hash = sha256(public_key);

    // 2. RIPEMD-160 hash of SHA-256 result
    let pkh = ripemd160(&hash);

    // 3. Encode with Bech32m
    bech32m_encode("bq", 0x10, &pkh)
}
```

### 3. Quantum-Resistant BIP39

#### 3.1 Enhanced Mnemonic Generation

Standard BIP39 with quantum-resistant enhancements:

```rust
// Enhanced entropy with quantum randomness
struct QuantumEntropy {
    // 256 bits of standard entropy
    standard_entropy: [u8; 32],

    // 128 bits of quantum-resistant entropy
    quantum_entropy: [u8; 16],

    // 32 bits checksum
    checksum: [u8; 4],
}
```

#### 3.2 Wordlist Extensions

Extended wordlist with quantum-themed words:

- Standard 2048 BIP39 words
- +256 quantum-themed words for enhanced security
- Total: 2304 words (11 bits per word)

#### 3.3 Mnemonic to Seed

```rust
fn quantum_mnemonic_to_seed(
    mnemonic: &str,
    passphrase: &str,
    quantum_salt: Option<&str>
) -> [u8; 64] {
    let base_seed = bip39_mnemonic_to_seed(mnemonic, passphrase);

    if let Some(q_salt) = quantum_salt {
        // Mix with quantum salt using Argon2id
        let quantum_mix = argon2id_mix(&base_seed, q_salt.as_bytes());
        xor_bytes(&base_seed, &quantum_mix)
    } else {
        base_seed
    }
}
```

### 4. Wallet SDK Standards

#### 4.1 Core Traits

```rust
/// Core wallet operations trait
pub trait Wallet {
    type Error: std::error::Error;

    /// Generate new wallet with quantum-resistant keys
    fn generate(config: &WalletConfig) -> Result<Self, Self::Error>;

    /// Restore from mnemonic
    fn from_mnemonic(mnemonic: &str, config: &WalletConfig) -> Result<Self, Self::Error>;

    /// Get address at derivation path
    fn get_address(&self, path: &DerivationPath) -> Result<Address, Self::Error>;

    /// Sign PQ-PSBT
    fn sign_psbt(&mut self, psbt: &mut PQPSBT) -> Result<(), Self::Error>;

    /// Get public key for address
    fn get_public_key(&self, address: &Address) -> Result<Vec<u8>, Self::Error>;
}

/// Post-Quantum PSBT operations
pub trait PQPSBT {
    /// Create new PQ-PSBT from transaction
    fn from_transaction(tx: Transaction) -> Self;

    /// Add input with UTXO data
    fn add_input(&mut self, input: PSBTInput) -> Result<(), PSBTError>;

    /// Add output
    fn add_output(&mut self, output: PSBTOutput) -> Result<(), PSBTError>;

    /// Sign with Dilithium key
    fn sign_dilithium(&mut self, private_key: &[u8]) -> Result<(), PSBTError>;

    /// Add ECDSA fallback signature
    fn sign_ecdsa(&mut self, private_key: &[u8]) -> Result<(), PSBTError>;

    /// Finalize and extract transaction
    fn finalize(self) -> Result<Transaction, PSBTError>;
}
```

#### 4.2 Configuration Standards

```rust
pub struct WalletConfig {
    /// Network (mainnet/testnet/regtest)
    pub network: Network,

    /// Signature algorithms to support
    pub signature_algorithms: Vec<SignatureAlgorithm>,

    /// Key derivation strategy
    pub derivation: DerivationConfig,

    /// Security settings
    pub security: SecurityConfig,

    /// Performance settings
    pub performance: PerformanceConfig,
}

pub struct SecurityConfig {
    /// Require both PQC and ECDSA signatures
    pub hybrid_signatures: bool,

    /// Memory locking for private keys
    pub memory_locking: bool,

    /// Cache timeout for decrypted keys
    pub cache_timeout: Duration,

    /// Quantum entropy source
    pub quantum_entropy: bool,
}

pub struct DerivationConfig {
    /// Use BIP32 standard paths
    pub bip32_standard: bool,

    /// Custom derivation path
    pub custom_path: Option<DerivationPath>,

    /// Account gap limit
    pub gap_limit: u32,
}
```

### 5. Hardware Wallet Integration

#### 5.1 Communication Protocol

Standardized USB/HID protocol for hardware wallets:

```
Command Structure:
- 0x01: Get Info
- 0x02: Get Public Key
- 0x03: Sign Transaction
- 0x04: Sign Message
- 0x05: Backup Wallet
- 0x06: Restore Wallet
```

#### 5.2 Device Capabilities

```rust
pub struct DeviceCapabilities {
    /// Supports Dilithium signatures
    pub supports_dilithium: bool,

    /// Supports ECDSA fallback
    pub supports_ecdsa: bool,

    /// Has secure display
    pub has_display: bool,

    /// Has physical buttons
    pub has_buttons: bool,

    /// Maximum message size
    pub max_message_size: usize,

    /// Firmware version
    pub firmware_version: String,
}
```

### 6. Address Validation

#### 6.1 Validation Rules

```rust
pub fn validate_address(address: &str, network: Network) -> ValidationResult {
    // 1. Check Bech32m format
    let (hrp, version, data) = bech32m_decode(address)?;

    // 2. Check human-readable part
    if hrp != network.hrp() {
        return Err(ValidationError::WrongNetwork);
    }

    // 3. Check version
    if !VALID_VERSIONS.contains(&version) {
        return Err(ValidationError::InvalidVersion);
    }

    // 4. Check data length
    if data.len() != 20 && data.len() != 32 {
        return Err(ValidationError::InvalidLength);
    }

    // 5. Check checksum
    verify_checksum(address)?;

    Ok(ValidationResult::Valid)
}
```

### 7. Transaction Builder Standards

#### 7.1 Builder Interface

```rust
pub struct TransactionBuilder {
    network: Network,
    version: u32,
    inputs: Vec<TxInput>,
    outputs: Vec<TxOutput>,
    locktime: u32,
    signature_algorithm: SignatureAlgorithm,
}

impl TransactionBuilder {
    pub fn new(network: Network) -> Self;

    pub fn add_input(&mut self, txid: &[u8; 32], vout: u32) -> &mut Self;
    pub fn add_output(&mut self, address: &str, amount: u64) -> Result<&mut Self, BuilderError>;
    pub fn set_locktime(&mut self, locktime: u32) -> &mut Self;
    pub fn set_signature_algorithm(&mut self, algo: SignatureAlgorithm) -> &mut Self;

    pub fn build(self) -> Result<Transaction, BuilderError>;
    pub fn build_psbt(self) -> Result<PQPSBT, BuilderError>;
}
```

## Implementation

### Rust SDK Structure

```
crates/bq-sdk/
├── src/
│   ├── lib.rs              # Main exports
│   ├── address/
│   │   ├── mod.rs          # Address operations
│   │   ├── bech32m.rs      # Bech32m encoding
│   │   └── validation.rs   # Address validation
│   ├── psbt/
│   │   ├── mod.rs          # PQ-PSBT implementation
│   │   ├── builder.rs      # PSBT builder
│   │   ├── signer.rs       # PSBT signing
│   │   └── serializer.rs   # Binary format
│   ├── wallet/
│   │   ├── mod.rs          # Wallet trait
│   │   ├── hd.rs           # HD wallet
│   │   ├── mnemonic.rs     # Quantum BIP39
│   │   └── config.rs       # Configuration
│   ├── crypto/
│   │   ├── mod.rs          # Crypto utilities
│   │   ├── dilithium.rs    # Dilithium wrapper
│   │   └── quantum.rs     # Quantum entropy
│   └── hardware/
│       ├── mod.rs          # Hardware wallet
│       ├── protocol.rs     # Communication
│       └── devices.rs      # Device support
├── tests/
│   ├── address_tests.rs
│   ├── psbt_tests.rs
│   └── wallet_tests.rs
└── Cargo.toml
```

### TypeScript SDK Structure

```
bindings/ts/
├── src/
│   ├── index.ts            # Main exports
│   ├── address/
│   │   ├── index.ts        # Address utilities
│   │   ├── bech32m.ts      # Bech32m encoding
│   │   └── validation.ts   # Validation
│   ├── psbt/
│   │   ├── index.ts        # PQ-PSBT
│   │   ├── builder.ts      # Builder
│   │   └── signer.ts       # Signing
│   ├── wallet/
│   │   ├── index.ts        # Wallet interface
│   │   ├── hd.ts           # HD wallet
│   │   └── mnemonic.ts     # Quantum BIP39
│   ├── crypto/
│   │   ├── index.ts        # Crypto utilities
│   │   └── dilithium.ts    # WebAssembly Dilithium
│   └── hardware/
│       ├── index.ts        # Hardware wallet
│       └── usb.ts         # USB communication
├── tests/
└── package.json
```

## Security Considerations

### 1. Post-Quantum Security

- **Dilithium3**: NIST-selected PQC algorithm
- **Hybrid Mode**: Support both PQC and ECDSA for transition
- **Key Sizes**: Account for larger PQC keys (1952 bytes)
- **Signature Sizes**: Handle 3293-byte Dilithium signatures

### 2. Memory Safety

- **Zeroization**: All secrets cleared from memory
- **Memory Locking**: Prevent swapping to disk
- **Constant-Time**: Prevent timing attacks
- **Secure Allocation**: Use secure allocators for keys

### 3. Address Security

- **Bech32m**: Error-correcting encoding
- **Checksum**: Strong error detection
- **Versioning**: Future-proof address types
- **Network Separation**: Prevent cross-network use

### 4. Mnemonic Security

- **Quantum Entropy**: Enhanced randomness sources
- **Passphrase Support**: Additional security layer
- **Wordlist Validation**: Prevent invalid words
- **Checksum Verification**: Detect corruption

## Testing

### 1. Conformance Tests

```rust
#[cfg(test)]
mod conformance_tests {
    use super::*;

    #[test]
    fn test_address_generation() {
        // Test vectors for all address types
    }

    #[test]
    fn test_psbt_serialization() {
        // Round-trip PSBT serialization
    }

    #[test]
    fn test_mnemonic_validation() {
        // Test mnemonic phrase validation
    }
}
```

### 2. Interoperability Tests

- Cross-wallet compatibility
- Hardware wallet integration
- Network protocol compliance
- Address format validation

## Migration Path

### Phase 1: Foundation (Q1 2025)
- Basic address generation
- Simple PSBT format
- Core wallet traits

### Phase 2: Post-Quantum (Q2 2025)
- Dilithium integration
- PQ-PSBT standard
- Quantum BIP39

### Phase 3: Ecosystem (Q3 2025)
- Hardware wallet support
- SDK implementations
- Developer tools

### Phase 4: Production (Q4 2025)
- Full conformance suite
- Documentation
- Security audits

## References

- [BIP-174] Partially Signed Bitcoin Transaction
- [BIP-32] Hierarchical Deterministic Wallets
- [BIP-39] Mnemonic Code for Generating Deterministic Keys
- [BIP-173] Base32 address format for native v0-16 witness outputs
- [BIP-350] Bech32m format for v1+ witness addresses
- [NIST PQC] Post-Quantum Cryptography Standardization

## Copyright

This document is placed in the public domain.
