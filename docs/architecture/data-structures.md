# Transaction and Block Data Structures

## Transaction Structure

### Transaction Format

```
Transaction {
    version: u16,                  // Protocol version (current: 2)
    lock_time: u32,                // Block height/timestamp lock
    inputs: Vec<TxIn>,             // Transaction inputs
    outputs: Vec<TxOut>,           // Transaction outputs
    sig_algo: SigAlgorithm,        // Signature algorithm enum
    witnesses: Vec<Witness>,       // Witness data (PQC signatures)
}
```

### TxIn (Input)

```
TxIn {
    prev_txid: [u8; 32],           // Previous transaction hash
    prev_vout: u32,                // Output index in prev TX
    script_sig: Vec<u8>,           // Script signature (legacy)
    sequence: u32,                 // Sequence number (0xffffffff)
}
```

### TxOut (Output)

```
TxOut {
    value: u64,                    // Amount in qbits (quantum bits)
    script_pubkey: Vec<u8>,        // Locking script/address
}
```

### Witness (Segregated Witness)

```
Witness {
    signatures: Vec<SignaturePayload>,  // PQC signatures
}

SignaturePayload {
    signer_index: u16,             // Index of signer
    signature: Vec<u8>,            // PQC signature bytes
    public_key: Vec<u8>,           // PQC public key
    aux: Option<Vec<u8>>,          // Auxiliary data
}
```

### SigAlgorithm Enum

```rust
enum SigAlgorithm {
    Dilithium3 = 1,    // Default (NIST Level 3)
    Falcon512 = 2,     // Alternative (NIST Level 1)
    SphincsPlus = 3,   // Hash-based (future)
}
```

## Transaction Weight Calculation

### Formula

```
tx_weight = base_size * 4 + sig_count * 384

where:
  base_size = size(version + lock_time + inputs + outputs)
  sig_count = total number of SignaturePayload in witnesses

Constants:
  WITNESS_SCALE_FACTOR = 4 (Bitcoin-compatible)
  SIGNATURE_WEIGHT = 384 WU (per PQC signature)
```

### Example

```
TX with:
  - 2 inputs (64 bytes each) = 128 bytes
  - 2 outputs (32 bytes each) = 64 bytes
  - Header (version + lock_time) = 8 bytes
  - 2 signatures (1 per input)

base_size = 128 + 64 + 8 = 200 bytes
sig_count = 2
tx_weight = 200 * 4 + 2 * 384 = 800 + 768 = 1568 WU
```

## Transaction Lifecycle

```
┌──────────────────────────────────────────────────────────────┐
│ 1. CREATION                                                  │
│    Wallet selects UTXOs (coin selection)                     │
│    Constructs inputs/outputs                                 │
│    Signs with Dilithium3                                     │
└────────────────┬─────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────┐
│ 2. BROADCAST                                                 │
│    Submit to local node (RPC/P2P)                            │
│    Node validates syntax                                     │
└────────────────┬─────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────┐
│ 3. MEMPOOL ENTRY                                             │
│    Check fee rate (≥ 10 qbits/WU)                           │
│    Verify signature (Dilithium3)                             │
│    Check UTXO availability                                   │
│    Compute weight and fee density                            │
│    Insert into mempool (sorted by fee/weight)                │
└────────────────┬─────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────┐
│ 4. MINING                                                    │
│    Miner calls getblocktemplate                              │
│    Select TXs by fee density (greedy)                        │
│    Build merkle tree                                         │
│    PoW search                                                │
└────────────────┬─────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────┐
│ 5. CONFIRMATION                                              │
│    Block found and broadcast                                 │
│    TX included in block                                      │
│    UTXO set updated (remove inputs, add outputs)             │
│    Mempool removes TX                                        │
└──────────────────────────────────────────────────────────────┘
```

## Block Structure

### Block Format

```
Block {
    header: BlockHeader,
    transactions: Vec<Transaction>,
}
```

### BlockHeader

```
BlockHeader {
    version: u32,                  // Block version
    prev_block: [u8; 32],          // Parent block hash
    merkle_root: [u8; 32],         // Merkle root of TXs
    pqc_agg_hint: [u8; 32],        // PQC aggregation hint (future)
    time: u64,                     // Block timestamp (Unix epoch)
    bits: u32,                     // Difficulty target (compact)
    nonce: u64,                    // PoW nonce
}
```

### Merkle Tree Construction

```
       merkle_root
           /  \
          /    \
         H01   H23
        /  \   /  \
       H0  H1 H2  H3
       |   |  |   |
      TX0 TX1 TX2 TX3

where:
  Hi = SHA-256d(TXi)
  Hij = SHA-256d(Hi || Hj)

SHA-256d = SHA-256(SHA-256(data))
```

## Block Weight Calculation

### Formula

```
block_weight = Σ tx_weight for all transactions

Max: 4,000,000 WU
```

### Example

```
Block with:
  - Coinbase: 250 bytes (no sigs) → 250*4 = 1000 WU
  - TX1: 200 bytes, 2 sigs → 200*4 + 2*384 = 1568 WU
  - TX2: 300 bytes, 1 sig → 300*4 + 1*384 = 1584 WU
  - TX3: 150 bytes, 1 sig → 150*4 + 1*384 = 984 WU

Total weight = 1000 + 1568 + 1584 + 984 = 5136 WU
```

## Block Validation Pipeline

```
┌──────────────────────────────────────────────────────────────┐
│ 1. HEADER VALIDATION                                         │
│    ✓ PoW: SHA-256d(header) < target                          │
│    ✓ Parent exists in chain                                  │
│    ✓ Timestamp > median(last 11)                             │
│    ✓ Version ≥ minimum                                       │
└────────────────┬─────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────┐
│ 2. WEIGHT VALIDATION                                         │
│    ✓ Calculate block weight                                  │
│    ✓ Check weight ≤ 4,000,000 WU                             │
└────────────────┬─────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────┐
│ 3. TRANSACTION VALIDATION                                    │
│    For each TX:                                              │
│      ✓ Inputs exist in UTXO set                              │
│      ✓ No double-spends                                      │
│      ✓ Sum(outputs) ≤ Sum(inputs) + coinbase_reward          │
│      ✓ Verify PQC signatures (Dilithium3)                    │
└────────────────┬─────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────┐
│ 4. MERKLE ROOT VALIDATION                                    │
│    ✓ Compute merkle_root from TXs                            │
│    ✓ Compare with header.merkle_root                         │
└────────────────┬─────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────┐
│ 5. UTXO UPDATE                                               │
│    - Remove spent UTXOs (TX inputs)                          │
│    + Add new UTXOs (TX outputs)                              │
│    ✓ Update chain tip                                        │
└──────────────────────────────────────────────────────────────┘
```

## Coinbase Transaction

### Format

```
Coinbase {
    version: 2,
    inputs: [
        TxIn {
            prev_txid: [0; 32],           // Null hash
            prev_vout: 0xffffffff,        // Max u32
            script_sig: height || nonce,  // Block height + arbitrary
            sequence: 0xffffffff,
        }
    ],
    outputs: [
        TxOut {
            value: block_reward + total_fees,
            script_pubkey: miner_address,
        }
    ],
    sig_algo: Dilithium3,
    witnesses: [],                        // No signatures for coinbase
}
```

### Reward Schedule

```
Block Height Range     | Reward (qbits)
-----------------------|---------------
0 - 210,000           | 50.0
210,001 - 420,000     | 25.0
420,001 - 630,000     | 12.5
...                   | ...
(halving every 210,000 blocks)
```

## Transaction Signing

### Signature Hash (sighash)

```
sighash = SHA-256d(
    version ||
    chain_id ||                    // Replay protection
    inputs[].prev_txid ||
    inputs[].prev_vout ||
    inputs[].sequence ||
    outputs[].value ||
    outputs[].script_pubkey ||
    lock_time ||
    sig_algo
)
```

### Dilithium3 Signing

```
1. Construct sighash
2. Sign with Dilithium3:
   signature = dilithium_sign(secret_key, sighash)
3. Attach to witness:
   witness.signatures.push(SignaturePayload {
       signer_index: input_index,
       signature: signature,
       public_key: public_key,
       aux: None,
   })
```

### Verification

```
1. Reconstruct sighash from TX
2. For each witness.signatures:
   dilithium_verify(public_key, sighash, signature)
3. All signatures must be valid
```

## Serialization Format

### Binary Encoding

```
Transaction:
  [u16: version]
  [u32: lock_time]
  [varint: num_inputs]
    For each input:
      [32 bytes: prev_txid]
      [u32: prev_vout]
      [varint: script_sig_len]
      [bytes: script_sig]
      [u32: sequence]
  [varint: num_outputs]
    For each output:
      [u64: value]
      [varint: script_pubkey_len]
      [bytes: script_pubkey]
  [u8: sig_algo]
  [varint: num_witnesses]
    For each witness:
      [varint: num_signatures]
        For each signature:
          [u16: signer_index]
          [varint: signature_len]
          [bytes: signature]
          [varint: public_key_len]
          [bytes: public_key]
          [bool: has_aux]
          [optional varint: aux_len]
          [optional bytes: aux]
```

### BlockHeader Encoding

```
BlockHeader (fixed 120 bytes):
  [u32: version]           // 4 bytes
  [32 bytes: prev_block]   // 32 bytes
  [32 bytes: merkle_root]  // 32 bytes
  [32 bytes: pqc_agg_hint] // 32 bytes
  [u64: time]              // 8 bytes
  [u32: bits]              // 4 bytes
  [u64: nonce]             // 8 bytes
```

## Data Storage

### UTXO Set (RocksDB)

```
Key: txid:vout (40 bytes: 32+4+4)
Value: TxOut {
    value: u64,
    script_pubkey: Vec<u8>,
    height: u32,              // Block height (for age tracking)
}

Index: ~O(1) lookup
Size: ~1-10 GB (depends on usage)
```

### Block Index

```
Key: height (u64, 8 bytes)
Value: BlockHash ([u8; 32])

Reverse index:
Key: hash ([u8; 32])
Value: height (u64)
```

### Block Data

```
Key: hash ([u8; 32])
Value: Block {
    header: BlockHeader,
    transactions: Vec<Transaction>,
}

Compression: optional (snappy/lz4)
```

## Network Serialization

### P2P Message Format

```
Message {
    magic: u32,                    // Network identifier
    command: [u8; 12],             // Command name (ASCII)
    payload_size: u32,             // Payload length
    checksum: u32,                 // SHA-256d(payload)[0..4]
    payload: Vec<u8>,              // Actual data
}

Example commands:
  - "version"     (handshake)
  - "inv"         (inventory announcement)
  - "getdata"     (request data)
  - "block"       (block data)
  - "tx"          (transaction data)
```

## Replay Protection

### Chain ID

```
Network    | Chain ID | Net-Magic
-----------|----------|----------
Mainnet    | 1        | 0xBQA1
Testnet    | 2        | 0xBQT1
Devnet     | 3        | 0xBQD1
Regtest    | 4        | 0xBQR1
```

Chain ID is included in sighash to prevent cross-chain replay attacks.

## References

- Bitcoin Transactions: https://en.bitcoin.it/wiki/Transaction
- BIP141 (SegWit): https://github.com/bitcoin/bips/blob/master/bip-0141.mediawiki
- Dilithium: https://pq-crystals.org/dilithium/
- BQIP-0001: PQC Signature Standard
- BQIP-0002: Block Weight and Fee Market
