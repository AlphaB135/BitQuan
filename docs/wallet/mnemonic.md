# BIP39 Mnemonic Recovery Phrases

**Last Updated: 2025-01-07**

Guide to using BIP39 mnemonic phrases for wallet backup and recovery in BitQuan.

## What is a Mnemonic Phrase?

A mnemonic phrase is a human-readable representation of your wallet's seed. It consists of 12 or 24 words from a standardized wordlist, making it easier to write down and transcribe than raw hexadecimal keys.

## Generating a Mnemonic

```bash
# Generate 12-word phrase
bitquan-wallet gen-mnemonic --words 12

# Example output:
# witch collapse practice feed shame open despair creek road again ice least

# Generate 24-word phrase (more secure)
bitquan-wallet gen-mnemonic --words 24
```

## Creating Wallet from Mnemonic

```bash
# Create wallet from existing mnemonic
bitquan-wallet create \
  --mnemonic "witch collapse practice feed shame open despair creek road again ice least" \
  --name recovered-wallet

# With custom passphrase (BIP39 extension)
bitquan-wallet create \
  --mnemonic "witch collapse ..." \
  --passphrase "my secret passphrase" \
  --name secure-wallet
```

## BIP39 Passphrase

Optional 13th/25th word adds extra security:

- Passphrase + mnemonic = different wallet
- Provides plausible deniability
- Must remember passphrase exactly
- Lost passphrase = lost funds

## Mnemonic Security

### ✅ DO:
- Write on paper immediately after generation
- Store in fireproof, waterproof location
- Consider metal backup plates
- Make multiple copies in secure locations
- Test recovery before storing funds
- Use 24 words for large amounts

### ❌ DON'T:
- Store digitally (photos, files, cloud)
- Share with anyone
- Enter on websites or untrusted software
- Split across multiple locations without proper technique
- Memorize as only backup
- Use predictable passphrases

## BIP39 Wordlist

Standard 2048-word English wordlist:
- Each word is 4+ letters
- First 4 letters are unique
- No offensive words
- Compatible with all BIP39 wallets

Full list: https://github.com/bitcoin/bips/blob/master/bip-0039/english.txt

## Checksum Verification

Last word contains checksum:

- 12 words: 128 bits entropy + 4 bits checksum
- 24 words: 256 bits entropy + 8 bits checksum

BitQuan automatically verifies checksum when importing.

## Recovery Examples

### Basic Recovery

```bash
bitquan-wallet create --mnemonic "abandon abandon ... art"
```

### With Passphrase

```bash
bitquan-wallet create \
  --mnemonic "abandon abandon ... art" \
  --passphrase "my secret"
```

### Verify Without Creating

```bash
bitquan-wallet verify-mnemonic \
  --mnemonic "abandon abandon ... art"
```

## Troubleshooting

### Invalid Mnemonic

**Error**: "Invalid mnemonic: checksum mismatch"

**Solution**: Check for typos. Last word contains checksum and must be exact.

### Wrong Balance

**Cause**: Missing passphrase or wrong passphrase

**Solution**: Ensure you're using the same passphrase (or none) as when wallet was created.

### Incompatible Wallet

**Issue**: Mnemonic from other wallet (Bitcoin, Ethereum)

**Note**: BIP39 is compatible but derivation paths differ. BitQuan uses `m/44'/0'/0'/0/x`. You can import but addresses won't match.

## Standards Compliance

BitQuan implements:

- **BIP39**: Mnemonic code for generating deterministic keys
- **BIP32**: Hierarchical Deterministic Wallets
- **BIP44**: Multi-Account Hierarchy for Deterministic Wallets

## See Also

- [Key Generation](./generation.md) - Wallet creation
- [Backup Guide](./backup.md) - Complete backup procedures
- [CLI Reference](../cli/bitquan-wallet.md) - Command reference

---

*Updated on: 2025-01-07*

**Warning**: Your mnemonic phrase IS your wallet. Anyone with your mnemonic can access your funds. Guard it like cash.
