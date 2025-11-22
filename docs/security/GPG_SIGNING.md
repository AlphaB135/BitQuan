# GPG Commit Signing Guide for BitQuan

## Why GPG Signing?

GPG-signed commits ensure:
- **Authentication**: Verify commits are from legitimate maintainers
- **Integrity**: Detect tampering with commit history
- **Accountability**: Non-repudiation of code changes
- **Security**: Prevent malicious code injection

**All commits to BitQuan MUST be GPG-signed.**

## Setup GPG Signing

### 1. Generate GPG Key (If Needed)

```bash
# Generate new key
gpg --full-generate-key

# Choose:
# - Type: RSA and RSA
# - Size: 4096 bits
# - Expiry: 2 years (renewable)
# - Real name: Your Name
# - Email: your-email@example.com
```

### 2. List Your Keys

```bash
# List keys
gpg --list-secret-keys --keyid-format LONG

# Output example:
# sec   rsa4096/ABCD1234EFGH5678 2025-10-25 [SC] [expires: 2027-10-25]
#       1234567890ABCDEF1234567890ABCDEF12345678
# uid                 [ultimate] Your Name <your-email@example.com>
```

Your key ID is: `ABCD1234EFGH5678`

### 3. Configure Git to Use GPG

```bash
# Set your GPG key
git config --global user.signingkey ABCD1234EFGH5678

# Enable automatic signing
git config --global commit.gpgSign true
git config --global tag.gpgSign true

# Set GPG program (if needed)
git config --global gpg.program gpg
```

### 4. Export Public Key

```bash
# Export to file
gpg --armor --export ABCD1234EFGH5678 > your-name.asc

# Submit to key server
gpg --keyserver keys.openpgp.org --send-keys ABCD1234EFGH5678
```

### 5. Add to GitHub

```bash
# Copy public key
gpg --armor --export ABCD1234EFGH5678 | pbcopy  # macOS
# or
gpg --armor --export ABCD1234EFGH5678 | xclip -sel clip  # Linux

# Go to GitHub.com → Settings → SSH and GPG keys → New GPG key
# Paste the key
```

## Signing Commits

### Normal Commit (Auto-signed)

```bash
git commit -m "feat: add transaction validation"
# Automatically signed due to commit.gpgSign=true
```

### Manual Signing

```bash
# If auto-sign is disabled
git commit -S -m "feat: add transaction validation"
```

### Verify Signature

```bash
# Verify last commit
git log --show-signature -1

# Output should show:
# gpg: Signature made Fri Oct 25 18:00:00 2025 UTC
# gpg:                using RSA key ABCD1234EFGH5678
# gpg: Good signature from "Your Name <your-email@example.com>"
```

## Signing Tags

### Create Signed Tag

```bash
# Annotated tag (automatically signed)
git tag -s v1.0.0 -m "Release v1.0.0"

# Or explicitly
git tag -s v1.0.0 -m "Release v1.0.0"
```

### Verify Tag

```bash
# Verify tag signature
git tag -v v1.0.0

# Output should show:
# object <commit-hash>
# type commit
# tag v1.0.0
# tagger Your Name <your-email@example.com>
# gpg: Signature made ...
# gpg: Good signature from "Your Name <your-email@example.com>"
```

### Push Signed Tags

```bash
# Push tags with signatures
git push origin v1.0.0

# Verify on GitHub
# Tag will show "Verified" badge
```

## GPG Key Management

### Key Rotation

```bash
# Extend expiration (every 2 years)
gpg --edit-key ABCD1234EFGH5678
> expire
> (choose new expiration)
> save

# Re-export and update
gpg --armor --export ABCD1234EFGH5678 > your-name-renewed.asc
gpg --keyserver keys.openpgp.org --send-keys ABCD1234EFGH5678
```

### Backup Keys

```bash
# Backup private key (KEEP SECURE!)
gpg --export-secret-keys --armor ABCD1234EFGH5678 > private-key-backup.asc

# Store in encrypted location (password manager, encrypted USB)
```

### Revocation Certificate

```bash
# Generate revocation certificate (IMPORTANT!)
gpg --gen-revoke ABCD1234EFGH5678 > revoke-cert.asc

# Store securely - use if key is compromised
```

## Maintainer Requirements

### For Core Maintainers

1. **GPG Key Published**:
   - Public key in `docs/security/keys/maintainers/`
   - Uploaded to key servers
   - Added to GitHub account

2. **Key Specifications**:
   - RSA 4096 bits minimum
   - Valid email address
   - 2-year expiration (renewable)
   - Backed up securely

3. **Signing Policy**:
   - ALL commits signed
   - ALL tags signed
   - Release artifacts signed

### Key Registry

Maintainer keys are stored in:
```
docs/security/keys/maintainers/
├── lead-maintainer.asc
├── alice-smith.asc
├── bob-jones.asc
└── README.md
```

## CI/CD Integration

### GitHub Actions Verification

```yaml
name: Verify GPG Signatures

on: [pull_request, push]

jobs:
  verify-signatures:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0

      - name: Import Maintainer Keys
        run: |
          for key in docs/security/keys/maintainers/*.asc; do
            gpg --import "$key"
          done

      - name: Verify Commits
        run: |
          # Get commits in PR
          commits=$(git log origin/main..HEAD --format="%H")

          for commit in $commits; do
            if ! git verify-commit "$commit" 2>/dev/null; then
              echo "❌ Commit $commit is not signed!"
              exit 1
            fi
            echo "✅ Commit $commit is signed"
          done
```

### Pre-push Hook

```bash
#!/bin/bash
# .git/hooks/pre-push

# Verify all commits are signed
while read local_ref local_sha remote_ref remote_sha; do
    if [ "$local_sha" != "0000000000000000000000000000000000000000" ]; then
        commits=$(git rev-list "$remote_sha..$local_sha")
        for commit in $commits; do
            if ! git verify-commit "$commit" 2>/dev/null; then
                echo "❌ ERROR: Commit $commit is not signed!"
                echo "All commits must be GPG-signed."
                exit 1
            fi
        done
    fi
done

echo "✅ All commits are properly signed"
exit 0
```

## Troubleshooting

### "gpg: signing failed: Inappropriate ioctl for device"

```bash
export GPG_TTY=$(tty)
echo 'export GPG_TTY=$(tty)' >> ~/.bashrc  # or ~/.zshrc
```

### "error: gpg failed to sign the data"

```bash
# Test GPG
echo "test" | gpg --clearsign

# Check pinentry
echo "GETPIN" | gpg-connect-agent

# Reset GPG agent
gpgconf --kill gpg-agent
```

### Commit Signing on macOS

```bash
# Install GPG Suite
brew install gnupg pinentry-mac

# Configure pinentry
echo "pinentry-program /usr/local/bin/pinentry-mac" >> ~/.gnupg/gpg-agent.conf

# Restart agent
gpgconf --kill gpg-agent
```

## Verification for Users

### Clone and Verify Repository

```bash
# Clone repository
git clone https://github.com/bitquan/bitquan.git
cd bitquan

# Import maintainer keys
for key in docs/security/keys/maintainers/*.asc; do
    gpg --import "$key"
done

# Verify latest tag
git tag -v $(git describe --tags --abbrev=0)

# Verify recent commits
git log --show-signature -10
```

### Automated Verification Script

```bash
#!/bin/bash
# scripts/verify-signatures.sh

echo "Verifying BitQuan repository signatures..."

# Import keys
for key in docs/security/keys/maintainers/*.asc; do
    gpg --import "$key" 2>/dev/null
done

# Verify last 100 commits
unsigned=0
for commit in $(git log -100 --format="%H"); do
    if ! git verify-commit "$commit" 2>/dev/null; then
        echo "⚠️  Unsigned commit: $commit"
        unsigned=$((unsigned + 1))
    fi
done

if [ $unsigned -eq 0 ]; then
    echo "✅ All commits are signed!"
else
    echo "❌ Found $unsigned unsigned commits"
    exit 1
fi
```

## Security Best Practices

### DO:
- ✅ Keep private key secure (encrypted disk)
- ✅ Use strong passphrase (20+ characters)
- ✅ Backup key and revocation certificate
- ✅ Set expiration date (2 years)
- ✅ Renew before expiration
- ✅ Use hardware token (YubiKey) for extra security

### DON'T:
- ❌ Share private key
- ❌ Commit private key to repository
- ❌ Use weak passphrase
- ❌ Skip key backup
- ❌ Ignore expiration warnings
- ❌ Sign commits from untrusted machines

## Hardware Token (Optional but Recommended)

### YubiKey Setup

```bash
# Install required tools
brew install ykman  # macOS
apt install yubikey-manager  # Linux

# Move GPG key to YubiKey
gpg --edit-key ABCD1234EFGH5678
> keytocard
> save

# Now GPG key requires physical YubiKey to sign
```

## References

- [GitHub GPG Documentation](https://docs.github.com/en/authentication/managing-commit-signature-verification)
- [GnuPG Documentation](https://gnupg.org/documentation/)
- [YubiKey GPG Guide](https://github.com/drduh/YubiKey-Guide)

---

**For BitQuan Security Questions**: security@bitquan.org
**Last Updated**: 2025-10-25
**Next Review**: 2026-01-25
