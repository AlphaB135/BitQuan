# Test Vectors (Draft)

These vectors are illustrative and will be expanded as implementations mature.

## RNG
- Derive stream label "wallet-seed": HKDF-SHA256(master=00..00, label) must produce deterministic 32-byte seed.

## Transaction/Block
- txid = SHA256d(tx without witness); wtxid = SHA256d(tx with witness).
- Merkle root over txid; witness_root over wtxid.

TBD: Concrete hex fixtures for Transaction v1/v2, BlockHeader v1/v2, and ASERT parameters.
