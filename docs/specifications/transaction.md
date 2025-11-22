# Transaction Specification

Version: 0.0.1-alpha
Status: Draft

## Overview

BitQuan transactions follow a UTXO model similar to Bitcoin, with post-quantum
signature support via Dilithium.

## Transaction Structure

```rust
struct Transaction {
    version: u16,           // Protocol version
    inputs: Vec<TxIn>,      // Inputs spending previous outputs
    outputs: Vec<TxOut>,    // New outputs created
    lock_time: u32,         // Block height or timestamp
    sig_algo: SigAlgorithm, // Signature algorithm for witnesses
    witnesses: Vec<Witness>,// Segregated signature data
}
```

### Version
- Current version: 2
- Version 1: Legacy (deprecated)
- Version 2: Witness support (current)

### Inputs

```rust
struct TxIn {
    prev_txid: [u8; 32],    // Previous transaction ID
    prev_vout: u32,         // Output index in previous tx
    script_sig: Vec<u8>,    // Script signature (legacy)
    sequence: u32,          // Sequence number (0xffffffff = final)
}
```

Coinbase input (first tx in block):
- prev_txid: all zeros
- prev_vout: 0xffffffff
- script_sig: arbitrary data (block height + coinbase message)

### Outputs

```rust
struct TxOut {
    value: u64,             // Amount in qbits (1 BQ = 100,000,000 qbits)
    script_pubkey: Vec<u8>, // Locking script
}
```

Standard output types:
- P2PKH: Pay to public key hash
- P2SH: Pay to script hash (future)
- OP_RETURN: Provably unspendable (data storage)

### Signature Algorithm

```rust
enum SigAlgorithm {
    Dilithium3 = 1,  // CRYSTALS-Dilithium level 3 (default)
    Falcon512 = 2,   // Falcon-512 (future)
    SPHINCS = 3,     // SPHINCS+ (future)
}
```

### Witnesses

```rust
struct Witness {
    signatures: Vec<SignaturePayload>,
}

struct SignaturePayload {
    signer_index: u16,      // Input index this signature satisfies
    signature: Vec<u8>,     // Raw signature bytes
    public_key: Vec<u8>,    // Signer public key
    aux: Option<AuxData>,   // Optional auxiliary data
}
```

## Transaction ID

TXID = SHA256(SHA256(serialized_tx_without_witness))

```
txid = double_sha256(
    version ||
    compact_uint(inputs.len) ||
    inputs[0..n] ||
    compact_uint(outputs.len) ||
    outputs[0..n] ||
    lock_time ||
    sig_algo
)
```

Witness data excluded from TXID (BIP141-style).

## Signature Hash

For Dilithium signing:

```
sighash = SHA256(
    version ||
    network_id ||          // Replay protection
    inputs_hash ||
    outputs_hash ||
    lock_time ||
    input_index            // Index being signed
)

inputs_hash = SHA256(prev_txid[0] || prev_vout[0] || ... || prev_txid[n] || prev_vout[n])
outputs_hash = SHA256(value[0] || script[0] || ... || value[n] || script[n])
```

### Network ID
- Mainnet: 0x01
- Testnet: 0x02
- Devnet: 0x03
- Regtest: 0x04

Prevents replay attacks across networks.

## Validation Rules

### Structure
1. Version must be 2
2. At least one input (except coinbase)
3. At least one output
4. Input count < 2^16
5. Output count < 2^16
6. Total output value < 21,000,000 * 10^8 qbits
7. No duplicate inputs

### Value
1. All output values > 0
2. Sum of outputs <= sum of inputs (fee >= 0)
3. No overflow in value arithmetic

### Witnesses
1. Number of witnesses == number of inputs
2. Each signature validates against corresponding input
3. Public key hash matches script_pubkey from prev output

### Coinbase
1. Only first transaction in block can be coinbase
2. Coinbase input: prev_txid = 0, prev_vout = 0xffffffff
3. Coinbase value <= subsidy + fees
4. Coinbase maturity: 100 blocks

## Size Limits

- Max transaction size: 400,000 bytes
- Max signature size: 4,000 bytes (Dilithium3)
- Max script size: 10,000 bytes
- Max witness stack: 100 items

## Examples

### Standard Transfer

```json
{
  "version": 2,
  "inputs": [{
    "prev_txid": "a1b2c3...",
    "prev_vout": 0,
    "script_sig": "",
    "sequence": 4294967295
  }],
  "outputs": [{
    "value": 50000000,
    "script_pubkey": "76a914..."
  }],
  "lock_time": 0,
  "sig_algo": 1,
  "witnesses": [{
    "signatures": [{
      "signer_index": 0,
      "signature": "d1b2c3...",
      "public_key": "e4f5a6..."
    }]
  }]
}
```

### Coinbase

```json
{
  "version": 2,
  "inputs": [{
    "prev_txid": "0000...0000",
    "prev_vout": 4294967295,
    "script_sig": "03e80300",
    "sequence": 4294967295
  }],
  "outputs": [{
    "value": 5000000000,
    "script_pubkey": "76a914..."
  }],
  "lock_time": 0,
  "sig_algo": 1,
  "witnesses": []
}
```

## References

- BIP141: Segregated Witness
- BIP143: Transaction Signature Verification
- NIST FIPS 204: CRYSTALS-Dilithium
