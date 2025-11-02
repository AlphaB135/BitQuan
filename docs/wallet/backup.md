# Wallet Backup & Restore

## Overview

BitQuan provides secure wallet backup with:
- **AES-256-GCM encryption** + separate backup password
- **HMAC-SHA256** tamper detection
- **Argon2id** key derivation (64 MiB, stronger than keystore)
- **Metadata** (network, timestamp, label)

## Creating Backup

```bash
bitquan-node wallet-backup \
  --keystore wallet.keystore \
  --output backup-2025-11-02.json \
  --network mainnet \
  --label "Main Wallet"
```

## Restoring Backup

```bash
bitquan-node wallet-restore \
  --backup backup-2025-11-02.json \
  --output restored.keystore
```

## Security

- **Two-layer encryption**: Keystore (wallet password) + Backup (backup password)
- **Tamper detection**: Any modification detected via HMAC
- **Strong KDF**: 64 MiB Argon2id

## Best Practices

1. **Regular backups** after significant changes
2. **Multiple locations** (cloud, USB, paper)
3. **Strong passwords** (different for wallet vs backup)
4. **Test restoration** periodically
5. **Secure deletion** of old backups

## FAQ

**Q: Backup password = wallet password?**  
A: No! Use different passwords for each layer.

**Q: Lost backup password?**  
A: Backup cannot be recovered. Use mnemonic instead.

**Q: Backup portable?**  
A: Yes, works across networks (metadata records network type).

## Related

- [Wallet Generation](./generation.md)
- [Mnemonic Recovery](./mnemonic.md)
- [Multisig Wallets](./multisig.md)

---
**Version**: 1.0 | **Updated**: 2025-11-02
