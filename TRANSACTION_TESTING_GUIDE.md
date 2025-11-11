# 💸 BitQuan Transaction Testing Guide

Complete guide to test transactions on BitQuan testnet

---

## 🎯 **Prerequisites:**

You need:
1. ✅ Running testnet node
2. ✅ Two wallets (sender & receiver)
3. ✅ Some testnet coins (from mining or faucet)

---

## 📝 **Transaction Testing Checklist:**

### **Test 1: Basic Transaction** ✅
```bash
# Send coins from wallet A to wallet B
./bitquan-node wallet-send \
  --keystore wallet-a.keystore \
  --to bq1q... \
  --amount 1000000 \
  --fee-rate 1

Expected: Transaction broadcast successful
```

### **Test 2: Check Balance** ✅
```bash
# Check sender balance
./bitquan-node balance --address SENDER_ADDRESS

# Check receiver balance
./bitquan-node balance --address RECEIVER_ADDRESS

Expected: Balances updated correctly
```

### **Test 3: Transaction Confirmation** ✅
```bash
# Mine a block to confirm transaction
./bitquan-node mine-once --network testnet

Expected: Transaction included in block
```

### **Test 4: Multiple Outputs** ✅
```bash
# Send to multiple recipients
./bitquan-node build-tx \
  --input UTXO_1 \
  --output ADDRESS_1:500000 \
  --output ADDRESS_2:500000

Expected: Multiple outputs created
```

### **Test 5: Fee Calculation** ✅
```bash
# Test different fee rates
./bitquan-node wallet-send \
  --keystore wallet.keystore \
  --to ADDRESS \
  --amount 100000 \
  --fee-rate 10

Expected: Higher fee = faster confirmation
```

### **Test 6: Max Amount** ✅
```bash
# Send maximum possible (balance - fee)
# Calculate: total_balance - estimated_fee

./bitquan-node wallet-send \
  --keystore wallet.keystore \
  --to ADDRESS \
  --amount <calculated_max>

Expected: Entire balance transferred
```

### **Test 7: Insufficient Funds** ❌
```bash
# Try to send more than balance
./bitquan-node wallet-send \
  --keystore wallet.keystore \
  --to ADDRESS \
  --amount 999999999999

Expected: Error "insufficient funds"
```

### **Test 8: Invalid Address** ❌
```bash
# Send to invalid address
./bitquan-node wallet-send \
  --keystore wallet.keystore \
  --to invalid_address \
  --amount 1000

Expected: Error "invalid address format"
```

### **Test 9: Zero Amount** ❌
```bash
# Send zero coins
./bitquan-node wallet-send \
  --keystore wallet.keystore \
  --to ADDRESS \
  --amount 0

Expected: Error "amount must be positive"
```

### **Test 10: Mempool Testing** ✅
```bash
# Send multiple unconfirmed transactions
for i in {1..10}; do
  ./bitquan-node wallet-send \
    --keystore wallet.keystore \
    --to ADDRESS \
    --amount 10000
done

# Check mempool
curl http://localhost:8334/mempool/stats

Expected: All transactions in mempool
```

---

## 🔬 **Advanced Transaction Tests:**

### **Multi-Signature Transactions:**
```bash
# Create 2-of-3 multisig
./bitquan-node wallet-gen-multisig \
  --required 2 \
  --total 3 \
  --pubkey1 PUBKEY1 \
  --pubkey2 PUBKEY2 \
  --pubkey3 PUBKEY3

# Sign with first key
./bitquan-node tx-sign-partial \
  --tx UNSIGNED_TX \
  --keystore wallet1.keystore

# Sign with second key
./bitquan-node tx-sign-partial \
  --tx PARTIAL_TX \
  --keystore wallet2.keystore

# Combine signatures
./bitquan-node tx-combine-signatures \
  --signatures SIG1,SIG2
```

### **Time-Locked Transactions:**
```bash
# Create tx that can't be spent until block 1000
./bitquan-node wallet-send \
  --keystore wallet.keystore \
  --to ADDRESS \
  --amount 100000 \
  --locktime 1000

Expected: Transaction valid only after block 1000
```

### **Replace-By-Fee (RBF):**
```bash
# Send with low fee
TX1=$(./bitquan-node wallet-send \
  --keystore wallet.keystore \
  --to ADDRESS \
  --amount 100000 \
  --fee-rate 1)

# Replace with higher fee
./bitquan-node wallet-send \
  --keystore wallet.keystore \
  --to ADDRESS \
  --amount 100000 \
  --fee-rate 10 \
  --replace-tx $TX1

Expected: Second tx replaces first in mempool
```

---

## 📊 **Performance Tests:**

### **High Volume Test:**
```bash
# Send 1000 transactions
for i in {1..1000}; do
  ./bitquan-node wallet-send \
    --keystore wallet.keystore \
    --to $(generate_random_address) \
    --amount 1000 &
done

wait
echo "Sent 1000 transactions"
```

### **Large Transaction Test:**
```bash
# Create transaction with 100 inputs
./bitquan-node build-tx \
  --input UTXO_1 \
  --input UTXO_2 \
  ... \
  --input UTXO_100 \
  --output ADDRESS:TOTAL_AMOUNT

Expected: Large transaction handled correctly
```

---

## ✅ **Test Results Template:**

```markdown
## Transaction Test Results

### Environment:
- Node version: v1.0.0
- Network: testnet
- Date: YYYY-MM-DD

### Test 1: Basic Transaction
- Status: ✅ PASS / ❌ FAIL
- Time: XX seconds
- Fee: XX qbits
- Notes: ___________

### Test 2: Check Balance
- Status: ✅ PASS / ❌ FAIL
- Sender balance: Correct / Incorrect
- Receiver balance: Correct / Incorrect
- Notes: ___________

[Continue for all tests...]

### Summary:
- Tests passed: XX/10
- Tests failed: XX/10
- Critical issues: X
- Minor issues: X

### Issues Found:
1. [Issue description]
2. [Issue description]
```

---

## 🐛 **Common Issues & Solutions:**

### **Issue: "Insufficient funds"**
```bash
# Solution: Check balance first
./bitquan-node balance --address YOUR_ADDRESS

# Make sure you account for fees
Amount to send = Balance - Estimated Fee
```

### **Issue: "Transaction not confirming"**
```bash
# Solution: Mine a block
./bitquan-node mine-once --network testnet

# Or wait for pool to mine
```

### **Issue: "Invalid signature"**
```bash
# Solution: Make sure you're using correct wallet
./bitquan-node wallet-address --keystore YOUR_WALLET.keystore

# Verify it matches the sender address
```

---

## 📈 **Success Criteria:**

Transaction testing is complete when:
- ✅ All basic tests pass (1-10)
- ✅ At least 100 successful transactions
- ✅ Multi-sig transactions work
- ✅ Time-locked transactions work
- ✅ High volume test completed
- ✅ All bugs documented

---

## 📝 **Current Status:**

```
Testing Status: 🟡 IN PROGRESS

Completed:
✅ Wallet creation
✅ Basic mining
✅ Balance checking

In Progress:
🔄 Basic transactions
🔄 Fee calculation
🔄 Error handling

Not Started:
⏳ Multi-sig transactions
⏳ Time-locked transactions
⏳ High volume testing
```

---

**Note:** Full transaction testing requires a working UTXO database and blockchain. Current local setup is ready for basic testing once we have mined blocks with proper coinbase outputs!

---

**Ready to test transactions? Start with Test 1! 💸**
