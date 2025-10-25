# BitQuan Maintainer GPG Keys

This directory contains public GPG keys of BitQuan maintainers for verifying signed commits and releases.

## Current Maintainers

### Lead Maintainer
- **Name**: TBD (To be announced at launch)
- **Key ID**: TBD
- **Fingerprint**: TBD
- **File**: `lead-maintainer.asc`

### Core Maintainers
- Positions to be filled during initial governance setup
- Minimum 3 maintainers required
- All keys will be published here upon appointment

## Importing Keys

```bash
# Import all maintainer keys
cd docs/security/keys/maintainers/
for key in *.asc; do
    gpg --import "$key"
done

# Or import from key server
gpg --keyserver keys.openpgp.org --recv-keys <KEY_ID>
```

## Verifying Signatures

```bash
# Verify commit
git verify-commit abc1234

# Verify tag
git verify-tag v1.0.0

# Verify release
gpg --verify SHA256SUMS.asc SHA256SUMS
```

## Key Requirements

All maintainer keys must:
- RSA 4096 bits minimum
- 2-year expiration (renewable)
- Valid email address
- Uploaded to 2+ key servers
- Backed up securely

## Trust Model

- **Commits**: 1+ Core Maintainer signature
- **Release Tags**: 2+ Core Maintainer signatures  
- **Consensus Changes**: Lead + 2 Core Maintainers

---

**Security Contact**: security@bitquan.org
