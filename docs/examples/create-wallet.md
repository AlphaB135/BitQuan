# Create Your First Wallet

This example shows you how to create a post-quantum wallet on BitQuan using Dilithium5 cryptography.

## Prerequisites

- BitQuan built from source
- 5 minutes

**Build if needed:**
```bash
cd BitQuan
cargo build --release
```

## Example 1: Generate New Wallet

### Step 1: Generate Keystore

```bash
./target/release/bitquan-node wallet-gen --output my-wallet.keystore
```

### Expected Output

```
Enter password (min 8 chars): ********
Confirm password: ********

Generating post-quantum keypair using CRYSTALS-Dilithium5...
Keypair generated successfully!

Keystore saved to: my-wallet.keystore

IMPORTANT: Write down your mnemonic phrase if shown!
Without it, you cannot recover your wallet.
```

### Step 2: Get Wallet Address

```bash
./target/release/bitquan-node wallet-address --keystore my-wallet.keystore
```

### Expected Output

```
Enter password: ********

Decoded address: bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q

Pubkey hash: 610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a7

Script: a820610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a787

SAVE THIS ADDRESS for receiving funds!
```

### Step 3: Verify Keystore File

```bash
ls -la my-wallet.keystore
```

### Expected Output

```
-rw------- 1 user user 244K Jan 22 10:00 my-wallet.keystore
```

**Note:** File size should be ~240KB (encrypted Dilithium5 keypair).

## Example 2: Generate from Mnemonic

### Step 1: Generate Wallet with Mnemonic

```bash
./target/release/bitquan-node wallet-gen-mnemonic --output mnemonic-wallet.keystore
```

### Expected Output

```
Enter password (min 8 chars): ********
Confirm password: ********

Generating mnemonic phrase...
WRITE THIS DOWN ON PAPER:

word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12

NEVER share this phrase with anyone!
NEVER store it digitally or take a photo!

Keystore saved to: mnemonic-wallet.keystore
```

**CRITICAL:** Write the mnemonic on paper immediately. If you lose it, you cannot recover your wallet!

### Step 2: Restore from Mnemonic (Test)

```bash
./target/release/bitquan-node wallet-from-mnemonic \
  --phrase "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12" \
  --output restored-wallet.keystore
```

### Expected Output

```
Enter new password: ********
Confirm password: ********

Restoring wallet from mnemonic phrase...
Wallet restored successfully!

Address: bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q

Verify this matches your original address!
```

## Example 3: Backup Wallet

### Step 1: Backup Keystore

```bash
# Create backup directory
mkdir -p backups

# Copy keystore
cp my-wallet.keystore backups/my-wallet.keystore.backup

# Set secure permissions
chmod 600 backups/my-wallet.keystore.backup
```

### Step 2: Verify Backup

```bash
# Test backup works
./target/release/bitquan-node wallet-address \
  --keystore backups/my-wallet.keystore.backup
```

### Expected Output

```
Enter password: ********

Address: bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q

Address matches original! Backup is valid.
```

## Common Errors

### Error: Password Too Short

```
Error: Invalid("Password must be at least 8 characters")
```

**Solution:** Use password with 8+ characters.

```bash
# Bad (7 characters)
./target/release/bitquan-node wallet-gen --output wallet.keystore
# Enter: pass123

# Good (8+ characters)
./target/release/bitquan-node wallet-gen --output wallet.keystore
# Enter: pass1234
```

### Error: File Already Exists

```
Error: FileAlreadyExists
```

**Solution:** Use different filename or delete existing file.

```bash
# Option 1: Use different name
./target/release/bitquan-node wallet-gen --output wallet2.keystore

# Option 2: Delete existing (BE CAREFUL!)
rm my-wallet.keystore
./target/release/bitquan-node wallet-gen --output my-wallet.keystore
```

### Error: Invalid Mnemonic

```
Error: Invalid("Invalid BIP39 mnemonic")
```

**Solution:** Check for:
- Typos in words
- Wrong word count (must be 12 or 24)
- Wrong word list language (use English)

## Security Checklist

### Before Using Wallet

- [ ] Password is 8+ characters, mixed case
- [ ] Mnemonic written on paper (not digital)
- [ ] Keystore backed up to secure location
- [ ] Address verified and written down
- [ ] Keystore file permissions set to 600

### NEVER

- Share keystore file with anyone
- Share mnemonic phrase with anyone
- Store mnemonic digitally (photo, screenshot, cloud)
- Enter mnemonic on websites
- Send keystore via email/chat
- Use same password as other sites

### ALWAYS

- Test backup restore before relying on it
- Store backups in separate physical locations
- Use strong, unique password
- Verify address before receiving funds
- Keep software updated

## Complete Example Script

```bash
#!/bin/bash
# wallet-setup.sh - Create and backup BitQuan wallet

set -e  # Exit on error

# Configuration
WALLET_NAME="my-wallet"
BACKUP_DIR="$HOME/bitquan-backups"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Generate wallet
echo "Creating wallet..."
./target/release/bitquan-node wallet-gen --output "${WALLET_NAME}.keystore"

# Get address
echo ""
echo "Your wallet address:"
./target/release/bitquan-node wallet-address --keystore "${WALLET_NAME}.keystore"

# Backup keystore
echo ""
echo "Backing up keystore..."
cp "${WALLET_NAME}.keystore" "$BACKUP_DIR/"
chmod 600 "$BACKUP_DIR/${WALLET_NAME}.keystore"

echo ""
echo "Wallet created and backed up!"
echo "Keystore: ${WALLET_NAME}.keystore"
echo "Backup: $BACKUP_DIR/${WALLET_NAME}.keystore"
echo ""
echo "IMPORTANT:"
echo "1. Write down your mnemonic phrase if shown"
echo "2. Store backups in secure location"
echo "3. NEVER share keystore or mnemonic"
```

**Usage:**
```bash
chmod +x wallet-setup.sh
./wallet-setup.sh
```

## What's Next?

After creating your wallet:

1. [Run a Node](run-node.md) - Start BitQuan node
2. [Mine Blocks](mine-blocks.md) - Mine coins to your address
3. [Send Transaction](send-transaction.md) - Send coins to others
4. [Check Balance](run-node.md) - Monitor your balance

## Related Documentation

- [Wallet Generation](../wallet/generation.md) - Full wallet guide
- [BIP39 Mnemonic](../wallet/mnemonic.md) - Mnemonic details
- [Wallet Backup](../wallet/backup.md) - Backup strategies
- [Wallet Issues](../troubleshooting/wallet-issues.md) - Troubleshooting
