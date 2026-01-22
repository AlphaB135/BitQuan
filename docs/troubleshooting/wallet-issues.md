# Wallet Issues Troubleshooting

Keystore or mnemonic problems? Can't access funds? Balance showing 0? This guide helps diagnose and fix wallet-related issues.

## Symptoms

- "Invalid password" when unlocking keystore
- Can't decrypt keystore file
- Mnemonic not generating correct address
- Balance showing 0 after mining
- "Insufficient funds" when trying to send
- "File not found" errors

## IMPORTANT: Backup Before Troubleshooting

**Before attempting any fixes:**

```bash
# Backup your keystore
cp my-wallet.keystore my-wallet.keystore.backup

# If using encrypted wallet, backup entire directory
tar -czf wallet-backup-$(date +%Y%m%d).tar.gz *.keystore

# Store backup in safe location (USB drive, encrypted cloud, etc.)
# NEVER share your keystore or mnemonic with anyone!
```

## Diagnostic Steps

### 1. Verify Keystore Exists

```bash
# Check file exists and is readable
ls -la my-wallet.keystore

# Should show file size (~240KB for encrypted keystore)
# If size is 0 bytes, file is corrupted
```

### 2. Check Keystore Format

```bash
# Verify it's valid JSON
cat my-wallet.keystore | jq .

# Should show structure like:
# {
#   "version": 1,
#   "algorithm": "argon2id",
#   "salt": "...",
#   "nonce": "...",
#   "ciphertext": "..."
# }
```

### 3. Test Password

```bash
# Try to get address (tests password)
./target/release/bitquan-node wallet-address \
  --keystore my-wallet.keystore

# Will prompt for password
# If "Invalid password", password is wrong
# If address shown, password is correct
```

## Common Issues and Solutions

### Issue: "Invalid Password"

**Symptoms:**
- "Invalid password" or "Wrong password" error
- "Password too short" (need 8+ characters)

**Possible Causes:**

#### A. Actually Wrong Password

**What's happening:** Password entered incorrectly.

**Solution:**
- Check caps lock
- Try copy-paste instead of typing
- Try common variations (with/without spaces, etc.)
- If absolutely lost, see "Lost Password" below

#### B. Encoding Issues

**What's happening:** Special characters in password not handled correctly.

**Solution:**
```bash
# Try password with single quotes
./target/release/bitquan-node wallet-address \
  --keystore my-wallet.keystore \
  --password 'p@ssw0rd!123'

# Or use environment variable (not recommended for production)
export WALLET_PASSWORD='your-password'
# (If supported)
```

#### C. Keystore Version Mismatch

**What's happening:** Keystore created with different BitQuan version.

**Solution:**
```bash
# Update to latest version
cd BitQuan
git pull origin main
cargo build --release

# Try again with new version
```

**LOST PASSWORD?**

**Unfortunately, there is NO password recovery.**

BitQuan uses Argon2id encryption specifically designed to prevent brute-force attacks. If you've lost your password:

- Try all possible variations carefully
- Check if you wrote it down somewhere
- If you have the mnemonic, you can restore wallet
- See "Restore from Mnemonic" below

### Issue: "Keystore File Not Found"

**Symptoms:**
- "File not found" or "No such file" error

**Solution:**
```bash
# Check current directory
ls -la *.keystore

# Try absolute path
./target/release/bitquan-node wallet-address \
  --keystore /full/path/to/wallet.keystore

# Search for keystore files
find ~ -name "*.keystore" 2>/dev/null
```

### Issue: "Corrupted Keystore"

**Symptoms:**
- File size is 0 bytes or very small
- JSON parse error
- "Invalid keystore format"

**Solution:**

**If you have backup:**
```bash
# Restore from backup
cp my-wallet.keystore.backup my-wallet.keystore
```

**If NO backup and you have mnemonic:**
- See "Restore from Mnemonic" below
- Generate new keystore from mnemonic

**If NO backup and NO mnemonic:**
- Funds are LOST (this is why backups are critical!)
- This is the harsh reality of cryptocurrency self-custody

### Issue: Mnemonic Not Generating Correct Address

**Symptoms:**
- Restored wallet has different address
- "Address mismatch" error

**Possible Causes:**

#### A. Wrong Mnemonic Word List

**What's happening:** Using incorrect BIP39 word list language.

**Solution:**
- Verify you're using English BIP39 wordlist
- Check for typos in mnemonic words
- Try with/without spaces
- Ensure 12 or 24 words (correct count)

#### B. Wrong Derivation Path

**What's happening:** Wallet using different BIP44/BIP84 path.

**Solution:**
```bash
# Try with explicit derivation path (if supported)
./target/release/bitquan-node wallet-from-mnemonic \
  --phrase "word1 word2 ... word12" \
  --derivation "m/44'/0'/0'/0/0"
```

#### C. Typo in Mnemonic

**What's happening:** One or more words entered incorrectly.

**Solution:**
- Double-check each word carefully
- BIP39 has checksum - invalid words will be rejected
- Try word variations (e.g., "built" vs "build")

### Issue: Balance Showing 0 After Mining

**Symptoms:**
- You mined blocks but balance is 0
- Balance check shows no UTXOs

**Possible Causes:**

#### A. Coinbase Not Mature Yet

**What's happening:** Mined coins need 100 blocks to mature.

**Solution:**
- Wait for 100 more blocks to be mined
- Or mine 100 more blocks yourself
- Then check balance again

```bash
# Check current block height
./target/release/bitquan-node info --datadir ./data/chainstate

# If you mined at block 50, funds unlock at block 150
```

#### B. Wrong Payout Script

**What's happening:** Mined to different address than your wallet.

**Solution:**
```bash
# Check what address you mined to
grep "payout" ./data/chainstate/mining.log

# Verify that address matches your wallet
./target/release/bitquan-node wallet-address \
  --keystore my-wallet.keystore
```

#### C. Different Datadir

**What's happening:** Checking balance in different database than where you mined.

**Solution:**
```bash
# Always use same datadir
./target/release/bitquan-node balance \
  --address <your-address> \
  --datadir ./data/chainstate
```

### Issue: "Insufficient Funds" When Sending

**Symptoms:**
- Want to send X coins but error says insufficient funds
- Balance shows Y coins where Y > X

**Possible Causes:**

#### A. Coinbase Maturity

**What's happening:** Trying to spend immature coinbase.

**Solution:**
- Wait for 100-block maturity
- Only spend matured UTXOs
- See [FAQ](faq.md) for coinbase explanation

#### B. Fee Calculation

**What's happening:** Balance includes fee needed for transaction.

**Solution:**
```bash
# Account for fees in calculation
Available = Balance - Fee

# If balance is 50 BQ and fee is 0.001 BQ
# Max send = 49.999 BQ
```

#### C. UTXO Fragmentation

**What's happening:** Many small UTXOs, fee becomes expensive.

**Solution:**
- Consolidate small UTXOs (send to yourself)
- Use higher fee rate to get confirmed
- Wait for more blocks to mine

### Issue: Can't Backup Keystore

**Symptoms:**
- Copy fails or corrupted backup
- Permission denied

**Solution:**
```bash
# Check file permissions
ls -la my-wallet.keystore

# Fix permissions if needed
chmod 600 my-wallet.keystore

# Backup with cp
cp -a my-wallet.keystore backup/keystore.backup

# Or create tar archive
tar -czf wallet-backup.tar.gz my-wallet.keystore
```

## Restore from Mnemonic

**If you have your BIP39 mnemonic phrase:**

```bash
# Generate wallet from mnemonic
./target/release/bitquan-node wallet-from-mnemonic \
  --phrase "word1 word2 word3 ... word12" \
  --output restored-wallet.keystore

# Set password for new keystore
# (Enter same password as original if possible)

# Verify address matches
./target/release/bitquan-node wallet-address \
  --keystore restored-wallet.keystore
```

**IMPORTANT:**
- Write mnemonic on paper (NEVER digital)
- Store in secure location (safe, fireproof box)
- Never share with anyone
- Never take photo of mnemonic
- Consider steel backup for fire resistance

## Create New Wallet

**If all else fails, create new wallet:**

```bash
# Generate new keystore
./target/release/bitquan-node wallet-gen \
  --output new-wallet.keystore

# Set strong password (8+ characters)
# Write down password and store securely

# Get new address
./target/release/bitquan-node wallet-address \
  --keystore new-wallet.keystore

# Save mnemonic if shown
# (Some wallet-gen modes may show mnemonic)
```

## Security Best Practices

### Password Security

1. **Length:** Minimum 8 characters (recommended: 16+)
2. **Complexity:** Mix of upper, lower, numbers, symbols
3. **Unique:** Never reuse password from other sites
4. **Storage:** Use password manager (Bitwarden, 1Password, etc.)
5. **NEVER:** Share password, enter on website, or give to support

### Keystore Security

1. **Permissions:** `chmod 600 wallet.keystore` (owner read/write only)
2. **Location:** Store in encrypted directory or USB drive
3. **Backup:** Multiple backups in different locations
4. **Encryption:** Encrypt backup with GPG or strong password
5. **NEVER:** Email keystore, store in cloud unencrypted, share

### Mnemonic Security

1. **Write on paper:** NEVER type into computer or phone
2. **Fireproof:** Consider steel backup (Cryptosteel, etc.)
3. **Multiple copies:** One with you, one in safe location
4. **Verification:** Test restore from mnemonic before relying on it
5. **NEVER:** Take photo, store digitally, share with anyone

## Recovery Checklist

If you've lost access to wallet:

- [ ] Try all password variations
- [ ] Check for backup files
- [ ] Search for written mnemonic
- [ ] Try restore from mnemonic
- [ ] Check if funds actually exist (block explorer, if available)
- [ ] Contact support (they CANNOT recover funds, but can help diagnose)

## Still Having Issues?

1. **Gather Diagnostic Info:**
   ```bash
   # Keystore info
   ls -la *.keystore > wallet-diag.txt
   wc -l *.keystore >> wallet-diag.txt

   # Try to decode (will show if JSON is valid)
   cat my-wallet.keystore | jq . >> wallet-diag.txt 2>&1

   # Version info
   bitquan-node --version >> wallet-diag.txt
   ```

2. **DO NOT Share:**
   - Your keystore file
   - Your mnemonic phrase
   - Your password
   - Your private keys

3. **Safe to Share:**
   - First 4 characters of address (for identification)
   - Error messages (sanitized)
   - BitQuan version
   - Operating system

## Related Guides

- [Wallet Generation](../wallet/generation.md) - Creating wallets
- [Wallet Backup](../wallet/backup.md) - Backup strategies
- [Mnemonic Guide](../wallet/mnemonic.md) - BIP39 details
- [FAQ](faq.md) - "How do I backup my wallet?"
