# Multi-Signature Wallets

**Last Updated: 2025-01-07**

Guide to creating and using multi-signature (multisig) wallets in BitQuan for enhanced security through distributed key management.

## Overview

Multisig wallets require M-of-N signatures to authorize transactions:

- **N**: Total number of signers
- **M**: Minimum signatures required (threshold)
- **Common schemes**: 2-of-3, 3-of-5, 5-of-7

## Creating Multisig Wallet

### Generate Individual Keys

Each participant generates their key:

```bash
# Alice generates her key
bitquan-wallet create --name alice-multisig
bitquan-wallet export-pubkey --wallet alice-multisig --output alice.pub

# Bob generates his key
bitquan-wallet create --name bob-multisig
bitquan-wallet export-pubkey --wallet bob-multisig --output bob.pub

# Charlie generates his key
bitquan-wallet create --name charlie-multisig
bitquan-wallet export-pubkey --wallet charlie-multisig --output charlie.pub
```

### Create Multisig Address

Combine public keys to create 2-of-3 multisig:

```bash
bitquan-wallet multisig create \
  --name team-wallet \
  --required 2 \
  --pubkeys alice.pub,bob.pub,charlie.pub
```

Output:
```
Multisig address: bq1multisig_addr_here...
Redeem script: 522103...52ae
```

## Signing Transactions

### Create Transaction

Any participant can create the transaction template:

```bash
bitquan-wallet multisig send \
  --wallet team-wallet \
  --to bq1recipient... \
  --amount 100.0 \
  --output partial.tx
```

### Collect Signatures

Each signer adds their signature:

```bash
# Alice signs
bitquan-wallet multisig sign \
  --wallet alice-multisig \
  --tx partial.tx \
  --output partial-1sig.tx

# Bob signs (reaches 2-of-3 threshold)
bitquan-wallet multisig sign \
  --wallet bob-multisig \
  --tx partial-1sig.tx \
  --output final.tx
```

### Broadcast

Once threshold is reached:

```bash
bitquan-wallet broadcast --tx final.tx
```

## Coordination Strategies

### Offline Signing

For maximum security, keep some keys offline:

1. Create unsigned transaction on online machine
2. Transfer to offline machine (USB, QR code)
3. Sign on offline machine
4. Transfer signed transaction back
5. Broadcast from online machine

### Hardware Wallet Integration

*Coming soon*: Hardware wallet support for multisig signing.

### PSBT (Partially Signed Bitcoin Transactions)

BitQuan supports PSBT format for multisig coordination:

```bash
# Create PSBT
bitquan-wallet create-psbt \
  --wallet team-wallet \
  --to bq1... \
  --amount 50.0 \
  --output tx.psbt

# Sign PSBT
bitquan-wallet sign-psbt --wallet alice-multisig --psbt tx.psbt

# Finalize when threshold reached
bitquan-wallet finalize-psbt --psbt tx.psbt --broadcast
```

## Security Models

### 2-of-2: Joint Accounts

Both parties must approve every transaction.

**Use case**: Business partners, married couples

**Pros**: Maximum control for both parties
**Cons**: Both must be available; single key loss = funds locked

### 2-of-3: Backup Key

Two parties control funds, third key for backup.

**Use case**: Personal savings with recovery option

**Example**:
- Key 1: Personal hot wallet
- Key 2: Hardware wallet
- Key 3: Paper backup in safe

**Pros**: Lose one key and still access funds
**Cons**: Any two keys can access funds

### 3-of-5: Organization Treasury

Threshold approval required.

**Use case**: Company funds, DAO treasury

**Example**: 5 board members, 3 must approve spending

**Pros**: Prevents single point of failure
**Cons**: Coordination overhead

## Advanced Features

### Timelocked Multisig

Add timelock to multisig for recovery:

```bash
bitquan-wallet multisig create \
  --required 2 \
  --pubkeys alice.pub,bob.pub,charlie.pub \
  --timelock 144  # ~1 day
```

After timelock expires, alternative signing path activates.

### Weighted Multisig

*Planned feature*: Different weights for different signers.

## Key Management

### Backup

- Each participant backs up their own key
- Save redeem script (required for spending)
- Document who holds which key

### Key Rotation

To rotate keys:

1. Create new multisig address with new keys
2. Send funds from old to new address
3. Old keys no longer control funds

### Disaster Recovery

If M-1 keys are lost:
- Funds are permanently locked
- Always maintain M+1 keys for safety margin
- Consider distributed geographical storage

## See Also

- [Full Multisig Guide](../MULTISIG_GUIDE.md) - Detailed guide
- [Key Generation](./generation.md) - Key creation
- [Wallet Backup](./backup.md) - Backup procedures
- [CLI Reference](../cli/bitquan-wallet.md) - Command reference

---

*Updated on: 2025-01-07*

**Critical**: Losing M keys = permanent fund loss. Plan carefully and document key locations.
