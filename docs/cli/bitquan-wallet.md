# bitquan-wallet Command Reference

**Last Updated: 2025-01-07**

The `bitquan-wallet` binary provides dedicated wallet management separate from the full node. It supports hierarchical deterministic (HD) wallets, multi-signature, and post-quantum Dilithium3 signatures.

## Usage

```bash
bitquan-wallet <COMMAND> [OPTIONS]
```

## Core Commands

### Wallet Creation

```bash
# Create new wallet with random seed
bitquan-wallet create --name my-wallet --output ~/.bitquan/wallets/

# Create from BIP39 mnemonic
bitquan-wallet create --mnemonic "word1 word2 ... word12" --name recovery-wallet

# Create watch-only wallet (no private keys)
bitquan-wallet create --watch-only --pubkey <hex> --name observer
```

### Address Management

```bash
# Generate new receiving address
bitquan-wallet address new --wallet my-wallet

# List all addresses
bitquan-wallet address list --wallet my-wallet

# Show address details
bitquan-wallet address show --address bq1...
```

### Balance & UTXOs

```bash
# Check wallet balance
bitquan-wallet balance --wallet my-wallet

# List unspent outputs
bitquan-wallet utxo list --wallet my-wallet

# Show specific UTXO
bitquan-wallet utxo show --txid <hash> --vout <index>
```

### Sending Transactions

```bash
# Send to single recipient
bitquan-wallet send \
  --wallet my-wallet \
  --to bq1recipient... \
  --amount 10.5 \
  --fee 0.001

# Send with custom fee rate (sats/byte)
bitquan-wallet send \
  --wallet my-wallet \
  --to bq1... \
  --amount 5.0 \
  --fee-rate 10

# Send to multiple recipients
bitquan-wallet send \
  --wallet my-wallet \
  --recipients recipients.json

# Build unsigned transaction (for offline signing)
bitquan-wallet build-tx \
  --wallet my-wallet \
  --to bq1... \
  --amount 10.0 \
  --output unsigned.tx
```

### Signing

```bash
# Sign transaction offline
bitquan-wallet sign \
  --wallet cold-wallet \
  --tx unsigned.tx \
  --output signed.tx

# Broadcast signed transaction
bitquan-wallet broadcast --tx signed.tx

# Verify signature
bitquan-wallet verify \
  --tx signed.tx \
  --pubkey <hex>
```

## Multi-Signature Wallets

```bash
# Create 2-of-3 multisig wallet
bitquan-wallet multisig create \
  --name team-wallet \
  --required 2 \
  --pubkeys pubkey1.hex,pubkey2.hex,pubkey3.hex

# Show multisig address
bitquan-wallet multisig address --wallet team-wallet

# Create multisig transaction
bitquan-wallet multisig send \
  --wallet team-wallet \
  --to bq1... \
  --amount 100.0 \
  --output partial.tx

# Sign with first key
bitquan-wallet multisig sign \
  --wallet team-wallet \
  --keyfile key1.pem \
  --tx partial.tx \
  --output partial-1sig.tx

# Sign with second key (completes 2-of-3)
bitquan-wallet multisig sign \
  --wallet team-wallet \
  --keyfile key2.pem \
  --tx partial-1sig.tx \
  --output final.tx

# Broadcast when threshold reached
bitquan-wallet broadcast --tx final.tx
```

See also: [Multi-Signature Guide](../guides/MULTISIG_GUIDE.md)

## Backup & Recovery

```bash
# Export wallet backup
bitquan-wallet backup --wallet my-wallet --output backup.enc

# Show recovery phrase (CAREFUL!)
bitquan-wallet show-mnemonic --wallet my-wallet

# Restore from backup
bitquan-wallet restore --backup backup.enc --output ~/.bitquan/wallets/

# Restore from mnemonic
bitquan-wallet restore-mnemonic \
  --phrase "word1 word2 ... word12" \
  --name restored-wallet
```

## Key Management

```bash
# Export public key
bitquan-wallet export-pubkey --wallet my-wallet --output pubkey.hex

# Import watching key
bitquan-wallet import-pubkey --pubkey pubkey.hex --name watch-wallet

# Rotate keys (generate new HD chain)
bitquan-wallet rotate-keys --wallet my-wallet --gap-limit 20

# Show key derivation path
bitquan-wallet show-path --wallet my-wallet --address bq1...
```

## Configuration

Default config location: `~/.bitquan/wallet.toml`

```toml
# Network selection
network = "mainnet"  # mainnet, testnet, regtest

# Node connection
node_rpc = "https://127.0.0.1:28332"
jwt_secret = "/path/to/jwt.secret"

# Wallet defaults
default_fee_rate = 10  # sats/byte
gap_limit = 20
```

## Environment Variables

- `BITQUAN_NETWORK` - Network override (mainnet/testnet/regtest)
- `BITQUAN_WALLET_DIR` - Wallet storage directory
- `BITQUAN_NODE_RPC` - Node RPC endpoint
- `BITQUAN_JWT_SECRET` - Path to JWT secret file

## Security Best Practices

1. **Never share your mnemonic phrase** - Store offline in secure location
2. **Use hardware wallets** for large amounts (when supported)
3. **Encrypt wallet files** with strong passphrase
4. **Test recovery** before storing significant funds
5. **Use multisig** for team/organizational funds
6. **Keep offline backup** of encrypted wallet + mnemonic
7. **Verify addresses** on separate device before sending

## See Also

- [bitquan-node](./bitquan-node.md) - Full node operations
- [Multi-Signature Guide](../guides/MULTISIG_GUIDE.md)
- [Security Policy](../security/SECURITY.md)

---

*Updated on: 2025-01-07*
