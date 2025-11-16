# Wallet Key Generation

**Last Updated: 2025-01-07**

This document describes key generation procedures for BitQuan wallets using post-quantum Dilithium3 signatures.

## Overview

BitQuan uses CRYSTALS-Dilithium3 for digital signatures, providing quantum resistance. Wallets support both random generation and BIP39 mnemonic recovery.

## Random Key Generation

```bash
# Generate new wallet with random seed
bitquan-wallet create --name my-wallet --output ~/.bitquan/wallets/

# With custom entropy source
bitquan-wallet create --entropy-file /dev/urandom --name secure-wallet
```

### Entropy Sources

BitQuan uses ChaCha20-based CSPRNG seeded from:

1. Operating system RNG (`/dev/urandom` on Unix, `BCryptGenRandom` on Windows)
2. Current timestamp (nanosecond precision)
3. Process ID and thread ID
4. Hardware RNG if available (RDRAND on x86-64)

All sources are mixed using BLAKE2b hashing before seeding ChaCha20.

## BIP39 Mnemonic Generation

```bash
# Generate 12-word mnemonic
bitquan-wallet gen-mnemonic --words 12

# Generate 24-word mnemonic
bitquan-wallet gen-mnemonic --words 24

# Create wallet from mnemonic
bitquan-wallet create --mnemonic "word1 word2 ... word12" --name recovery-wallet
```

### Mnemonic Security

- Mnemonics use standard BIP39 wordlist (2048 words)
- 12 words = 128 bits entropy
- 24 words = 256 bits entropy
- Store offline in secure location
- Never share or transmit electronically

## Hierarchical Deterministic (HD) Derivation

BitQuan wallets use HD key derivation:

```
m / purpose' / coin_type' / account' / change / address_index
m / 44'      / 0'         / 0'       / 0      / 0
```

- **Purpose**: 44 (BIP44)
- **Coin type**: 0 (BitQuan)
- **Account**: 0 (default)
- **Change**: 0 (receiving), 1 (change addresses)
- **Index**: 0, 1, 2, ... (address index)

```bash
# Derive address at index 5
bitquan-wallet derive --path "m/44'/0'/0'/0/5"

# Show derivation path for address
bitquan-wallet show-path --address bq1...
```

## Key Storage

Keys are encrypted at rest using AES-256-GCM:

```bash
# Set password during creation
bitquan-wallet create --name my-wallet
# (prompts for password)

# Change password
bitquan-wallet passwd --wallet my-wallet
```

### Keystore Format

```json
{
  "version": 1,
  "crypto": {
    "cipher": "aes-256-gcm",
    "ciphertext": "...",
    "cipherparams": {
      "iv": "..."
    },
    "kdf": "argon2id",
    "kdfparams": {
      "memory": 65536,
      "iterations": 3,
      "parallelism": 4,
      "salt": "..."
    },
    "mac": "..."
  },
  "id": "uuid",
  "address": "bq1..."
}
```

## Hardware Wallet Support

*Coming soon*: Hardware wallet integration for offline signing.

## Security Best Practices

1. **Never reuse seeds** - Generate unique seed for each wallet
2. **Backup immediately** - Write down mnemonic before using wallet
3. **Test recovery** - Restore from mnemonic to verify backup
4. **Secure storage** - Store mnemonic in fireproof safe or safety deposit box
5. **Split secrets** - Consider Shamir's Secret Sharing for high-value wallets
6. **Verify checksums** - Always verify BIP39 checksum when entering mnemonic

## See Also

- [Wallet Backup](./backup.md) - Backup and recovery procedures
- [BIP39 Mnemonic](./mnemonic.md) - Mnemonic phrase guide
- [Multi-Signature](./multisig.md) - Multi-signature wallets
- [Security Policy](../security/) - Security guidelines

---

*Updated on: 2025-01-07*
