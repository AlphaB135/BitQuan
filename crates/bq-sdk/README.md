# BitQuan SDK (bq-sdk)

Developer SDK for building on top of the BitQuan blockchain.

## Installation

```toml
[dependencies]
bq-sdk = { path = "../bq-sdk" }
```

## Modules

| Module | Description |
|--------|-------------|
| `address` | BitQuan address generation and validation |
| `crypto` | Post-quantum signatures (Dilithium5), hashing, KDF |
| `wallet` | Wallet operations: create, sign, verify, backup |
| `psbt` | Partially Signed Bitcoin Transactions (PQC-PSBT) |
| `hardware` | Hardware wallet integration layer |

## Quick Start

```rust
use bq_sdk::wallet::Wallet;
use bq_sdk::address::Address;
use bq_sdk::crypto;

// Generate a new post-quantum keypair
let wallet = Wallet::generate(bq_sdk::NetworkId::Testnet)?;

// Get the BitQuan address
let addr = wallet.address()?;
println!("Address: {}", addr);

// Export public key
let pub_key = wallet.public_key();
```

## Address Format

BitQuan addresses use bech32 encoding with "bq" prefix:

```
bq1q... (P2PKH - Pay to Public Key Hash)
bq1p... (P2SH  - Pay to Script Hash)
```

## Post-Quantum Signatures

BitQuan uses CRYSTALS-Dilithium5 (NIST PQC Level 5):

```rust
use bq_sdk::crypto::dilithium;

// Generate keypair
let (public_key, secret_key) = dilithium::keypair()?;

// Sign message
let signature = dilithium::sign(&secret_key, &message)?;

// Verify signature
let valid = dilithium::verify(&public_key, &message, &signature)?;
assert!(valid);
```

## Wallet Operations

```rust
use bq_sdk::wallet::Wallet;

// Create wallet with encryption
let wallet = Wallet::create("password123", bq_sdk::NetworkId::Testnet)?;

// Save to file
wallet.save_to_file("my_wallet.json")?;

// Load from file
let wallet = Wallet::load_from_file("my_wallet.json", "password123")?;

// Sign a transaction
let signed_tx = wallet.sign_transaction(&unsigned_tx)?;
```

## Transaction Building

```rust
use bq_sdk::psbt::Psbuilder;

let tx = Psbuilder::new()
    .add_input(&prev_txid, prev_vout)?
    .add_output(&recipient_address, amount)?
    .build()?;

let signed = wallet.sign_transaction(&tx)?;
```

## Network Identifiers

| Network | ID |
|---------|----|
| Mainnet | `NetworkId::Mainnet` |
| Testnet | `NetworkId::Testnet` |
| Devnet | `NetworkId::Devnet` |
| Regtest | `NetworkId::Regtest` |
