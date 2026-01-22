# Send a Transaction

This example shows you how to send BitQuan coins from one wallet to another.

## Prerequisites

- Two wallets with addresses
- Funds in sender wallet (mined or received)
- BitQuan node running
- 10 minutes

**Before starting:** Ensure you have mined blocks to your wallet address (see [Mine Blocks](mine-blocks.md)).

## Example 1: Send Transaction

### Step 1: Check Sender Balance

```bash
./target/release/bitquan-node balance \
  --address bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q \
  --datadir ./data/chainstate
```

### Expected Output

```
=== BitQuan Balance ===
Chain height: 116
Decoded address: bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q
Pubkey hash: 610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a7
Script: a820610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a787

Scanning blockchain for UTXOs...
 Block #6 TX ... vout=0 amount=50000000000000000000
...

UTXO count: 100
Balance: 5000000000000000000000 qbits
Balance: 50.000000000000000000 BQ
```

**Note:** You need mature UTXOs (100+ blocks old) to spend.

### Step 2: Send Transaction

Send 1 BQ (1,000,000,000,000,000,000 qbits) to recipient:

```bash
./target/release/bitquan-node wallet-send \
  --keystore sender-wallet.keystore \
  --to bq1q8f82c7w3u8fmrzvng8h0xpka2pg9nfhs04d3ylvdgjwvnaz7eqyjl5n8jn \
  --amount 1000000000000000000 \
  --fee-rate 1000 \
  --datadir ./data/chainstate
```

### Expected Output

```
Enter password: ********

Scanning for UTXOs...
Found 100 UTXOs worth 50 BQ
Selected 1 UTXO(s) worth 50 BQ

Creating transaction:
  From: bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q
  To: bq1q8f82c7w3u8fmrzvng8h0xpka2pg9nfhs04d3ylvdgjwvnaz7eqyjl5n8jn
  Amount: 1.000000000000000000 BQ
  Fee: 0.000000001 BQ
  Change: 48.999999999 BQ

Signing transaction with Dilithium5...
Transaction created successfully!

TXID: b6a327f6490e48eaff9ec30bb6c3876244ce44704a1e9345f45da040189f1b5c

Transaction saved to pending pool.
Mine a block to include this transaction in the blockchain.
```

### Step 3: Mine Block to Include Transaction

```bash
./target/release/bitquan-node mine \
  --pow mock \
  --payout-script-hex a820610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a787 \
  --limit-blocks 1 \
  --datadir ./data/chainstate
```

### Expected Output

```
Mining started with algorithm: mock
Starting block #117...

Processing 1 pending transactions...
Including transaction: b6a327f6490e48eaff9ec30bb6c3876244ce44704a1e9345f45da040189f1b5c

FOUND Block #117 | Nonce: 0 | Hash: 0abc123... | 0.00s
Block submitted to chainstate.
```

### Step 4: Verify Transaction

```bash
# Check recipient balance
./target/release/bitquan-node balance \
  --address bq1q8f82c7w3u8fmrzvng8h0xpka2pg9nfhs04d3ylvdgjwvnaz7eqyjl5n8jn \
  --datadir ./data/chainstate
```

### Expected Output

```
=== BitQuan Balance ===
Chain height: 117
...
Balance: 1000000000000000000 qbits
Balance: 1.000000000000000000 BQ
```

## Example 2: Send with Maximum Amount

### Calculate Maximum Sendable

Balance minus fee:

```bash
# If balance is 50 BQ and fee is 0.001 BQ
# Max send = 50 - 0.001 = 49.999 BQ

# In qbits (18 decimals):
# 50 BQ = 50000000000000000000 qbits
# 0.001 BQ fee = 1000000000000000 qbits
# Max send = 49999999999999999000 qbits

./target/release/bitquan-node wallet-send \
  --keystore sender-wallet.keystore \
  --to bq1q8f82c7w3u8fmrzvng8h0xpka2pg9nfhs04d3ylvdgjwvnaz7eqyjl5n8jn \
  --amount 49999999999999999000 \
  --fee-rate 1000 \
  --datadir ./data/chainstate
```

## Example 3: Send Multiple Transactions

```bash
#!/bin/bash
# Send to multiple recipients

RECIPIENTS=(
  "bq1q8f82c7w3u8fmrzvng8h0xpka2pg9nfhs04d3ylvdgjwvnaz7eqyjl5n8jn:1000000000000000000"
  "bq1qabc123...xyz789:5000000000000000000"
  "bq1qdef456...uvw012:2000000000000000000"
)

for recipient in "${RECIPIENTS[@]}"; do
  address=$(echo $recipient | cut -d: -f1)
  amount=$(echo $recipient | cut -d: -f2)

  echo "Sending to $address..."
  ./target/release/bitquan-node wallet-send \
    --keystore sender-wallet.keystore \
    --to "$address" \
    --amount "$amount" \
    --fee-rate 1000 \
    --datadir ./data/chainstate
done
```

## Amount Units

BitQuan uses 18 decimal places (qbits):

| Unit | Qbits | Example |
|------|-------|---------|
| 1 BQ | 10^18 qbits | `1000000000000000000` |
| 0.1 BQ | 10^17 qbits | `100000000000000000` |
| 0.001 BQ | 10^15 qbits | `1000000000000000` |
| 0.000001 BQ | 10^12 qbits | `1000000000000` |

**Quick conversion:**
- 1 BQ = 1,000,000,000,000,000,000 qbits (18 zeros)
- To send X BQ: Multiply X by 10^18

## Common Errors

### Error: Insufficient Funds

```
Error: Invalid("Insufficient funds: found 0 qbits, need 1000000000000000000 qbits")
```

**Cause:** No mature UTXOs available.

**Solution:**
1. Mine more blocks (need 100 blocks for maturity)
2. Or wait for existing blocks to mature

```bash
# Check block height
./target/release/bitquan-node info --datadir ./data/chainstate

# If you mined at block #50, funds unlock at #150
```

### Error: Invalid Address

```
Error: Invalid("Invalid address format")
```

**Cause:** Recipient address malformed.

**Solution:** Verify address format:
- Must start with `bq1q`
- Must be Bech32m encoded
- Must be correct length

```bash
# Verify your own address first
./target/release/bitquan-node wallet-address --keystore my-wallet.keystore
```

### Error: Invalid Password

```
Error: Invalid("Invalid password")
```

**Solution:** Double-check password, try copy-paste.

### Error: Transaction Not in Mempool

```
Transaction created but not yet in blockchain.
```

**Normal behavior:** Transaction saved to `pending_transactions.jsonl`.

**Solution:** Mine a block to include it.

```bash
./target/release/bitquan-node mine --pow mock --limit-blocks 1
```

## Transaction Flow

```
1. wallet-send
   ├─ Scan UTXOs
   ├─ Select inputs
   ├─ Build transaction
   ├─ Sign with Dilithium5
   └─ Save to pending_transactions.jsonl

2. mine
   ├─ Read pending transactions
   ├─ Validate transaction
   ├─ Include in block
   └─ Submit to blockchain

3. Transaction confirmed
   ├─ UTXOs marked spent
   ├─ New UTXOs created
   └─ Balances updated
```

## Fee Calculation

Fees depend on:
- Transaction size (bytes)
- Fee rate (qbits per byte)

**Formula:**
```
Fee = Transaction Size × Fee Rate

Example:
- Transaction size: 500 bytes
- Fee rate: 1000 qbits/byte
- Fee = 500 × 1000 = 500,000 qbits = 0.0000005 BQ
```

**Default fee rate:** 1000 qbits/byte (minimal)

## Security Best Practices

### Before Sending

- [ ] Verify recipient address (copy-paste, don't type)
- [ ] Double-check amount (count zeros!)
- [ ] Ensure sufficient funds (including fee)
- [ ] Test with small amount first
- [ ] Verify network (devnet vs mainnet)

### After Sending

- [ ] Save TXID for reference
- [ ] Mine block to confirm transaction
- [ ] Verify recipient received funds
- [ ] Check sender balance updated

### NEVER

- Send to wrong address (no recovery!)
- Send mainnet funds to testnet address
- Forget to account for fees
- Send without testing first

## Complete Example Script

```bash
#!/bin/bash
# send-coin.sh - Send BitQuan coins

set -e

# Configuration
KEYSTORE="sender-wallet.keystore"
RECIPIENT="bq1q8f82c7w3u8fmrzvng8h0xpka2pg9nfhs04d3ylvdgjwvnaz7eqyjl5n8jn"
AMOUNT="1000000000000000000"  # 1 BQ
DATADIR="./data/chainstate"

# Check balance
echo "Checking sender balance..."
./target/release/bitquan-node balance \
  --address "$(./target/release/bitquan-node wallet-address --keystore $KEYSTORE | grep 'Decoded address' | awk '{print $3}')" \
  --datadir "$DATADIR"

# Send transaction
echo ""
echo "Sending transaction..."
TXID=$(./target/release/bitquan-node wallet-send \
  --keystore "$KEYSTORE" \
  --to "$RECIPIENT" \
  --amount "$AMOUNT" \
  --fee-rate 1000 \
  --datadir "$DATADIR" | grep "TXID:" | awk '{print $2}')

echo "Transaction created: $TXID"

# Mine block
echo ""
echo "Mining block to include transaction..."
./target/release/bitquan-node mine \
  --pow mock \
  --limit-blocks 1 \
  --datadir "$DATADIR"

# Verify
echo ""
echo "Verifying transaction..."
./target/release/bitquan-node balance \
  --address "$RECIPIENT" \
  --datadir "$DATADIR"

echo "Transaction complete!"
```

## What's Next?

- [Run Node](run-node.md) - Start your own node
- [Mine Blocks](mine-blocks.md) - Mine more coins
- [RPC Calls](rpc-calls.md) - Use JSON-RPC API
- [Troubleshooting](../troubleshooting/wallet-issues.md) - Wallet problems

## Related Documentation

- [Transaction Format](../specifications/transaction.md) - Technical details
- [Wallet Issues](../troubleshooting/wallet-issues.md) - Troubleshooting
- [FAQ](../troubleshooting/faq.md) - Transaction questions
