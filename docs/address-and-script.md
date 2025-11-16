# Address & Script Conversion Guide

This guide shows how to turn a BitQuan Bech32m address (`bq1...`) into the script
hex expected by mining and balance tooling, with a real example taken from an
interactive wallet session.

## 1. Validate the address

Confirm the checksum, network, and derived pubkey hash before broadcasting:

```bash
./target/release/bitquan-node validateaddress \
  --address bq1qxhdsk5ragya62kkdasmmzw8z8k4h228lkxaafskap0vevy9k4swxddc75y
```

Sample output:

```
BitQuan Address Validation
Input      : bq1qxhdsk5ragya62kkdasmmzw8z8k4h228lkxaafskap0vevy9k4swxddc75y
Network     : mainnet
HRP         : bq
Checksum    : OK (Bech32m)
Payload size: 32 bytes
Pubkey hash : aed85a83ea09dd2ad66f61bd89c711ed5ba947fd8ddea616e85eccb085b560e3
Script hex  : a820aed85a83ea09dd2ad66f61bd89c711ed5ba947fd8ddea616e85eccb085b560e387
```

## 2. Emit script hex for shell pipelines

When you need only the `script_pubkey` (for `mine`, `balance`, or external
tooling), use `script-from-address`. It prints the hex on stdout so it can be
captured directly, while metadata is written to stderr.

```bash
SCRIPT_HEX=$(
  ./target/release/bitquan-node script-from-address \
    --address bq1qxhdsk5ragya62kkdasmmzw8z8k4h228lkxaafskap0vevy9k4swxddc75y 2>/dev/null
)
echo "$SCRIPT_HEX"
# a820aed85a83ea09dd2ad66f61bd89c711ed5ba947fd8ddea616e85eccb085b560e387
```

## 3. Put the script to work

### Mining payout

```bash
./target/release/bitquan-node mine \
  --payout-script-hex "$SCRIPT_HEX" \
  --threads 4
```

### Balance lookup (with RocksDB backend enabled)

```bash
./target/release/bitquan-node balance \
  --address bq1qxhdsk5ragya62kkdasmmzw8z8k4h228lkxaafskap0vevy9k4swxddc75y
```

## Tips

- Legacy `q1...` addresses are still accepted; the CLI will label them as
  “mainnet (legacy q1)” and normalize casing before conversion.
- Testnet addresses use the `bqt1...` prefix and are also supported by the new
  commands.
- The script builder uses the canonical BitQuan template
  `OP_HASH256 <32-byte hash> OP_EQUAL` (`a8 20 <hash> 87`).
