# Build Attestations

This directory contains independent build attestations from community members verifying that official BitQuan releases match reproducible builds.

## What is a Build Attestation?

An attestation is a cryptographically signed statement that an independent builder:
1. Built BitQuan from source at a specific commit
2. Obtained identical binaries to the official release
3. Verified checksums match

## Format

```
-----BEGIN PGP SIGNED MESSAGE-----
Hash: SHA256

BitQuan Build Attestation

Version: v1.0.0
Commit: abc123def456...
Platform: Linux x86_64
Builder: John Doe <john@example.com>
Date: 2025-10-25
Official SHA256: abc123...
My Build SHA256: abc123...
Match: YES

I attest that I independently built BitQuan from the published
source code and verified the binary matches the official release.

-----BEGIN PGP SIGNATURE-----
iQIzBAEBCAAdFiEE...
-----END PGP SIGNATURE-----
```

## Submitting an Attestation

1. Build BitQuan following `docs/REPRODUCIBILITY.md`
2. Create attestation file: `v1.0.0-yourname.txt`
3. Sign with your GPG key
4. Submit PR to this directory
5. Include your public key in `docs/security/keys/community/`

## Verification

```bash
# Verify attestation signature
gpg --verify v1.0.0-alice.txt

# Check multiple attestations match
grep "My Build SHA256" v1.0.0-*.txt | sort | uniq -c
```

## Current Attestations

### v1.0.0 (Upcoming)
- None yet (Pre-release)

---

**How to Help**: Build and attest! See `docs/REPRODUCIBILITY.md`
