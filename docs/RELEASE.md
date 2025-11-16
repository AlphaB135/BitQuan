# Release Process

BitQuan follows a strict release process to ensure security, stability, and transparency.

## Release Cycle

- **Alpha**: Pre-release for devnet testing (v0.0.x-alpha)
- **Beta**: Feature-complete, testnet deployment (v0.x.x-beta)
- **RC**: Release candidate, final testing (v0.x.x-rc.N)
- **Stable**: Production-ready mainnet release (v1.x.x)

## Release Checklist

### Pre-Release (RC)

1. **Code Freeze**
   - Merge all features for the release
   - Branch from `main` to `release/vX.Y.Z`
   - Update version in `Cargo.toml`

2. **Testing**
   - Run full test suite: `cargo test --all --locked`
   - Run integration tests on devnet/testnet
   - Perform security audit if major release
   - Run fuzzing tests for at least 24 hours

3. **Documentation**
   - Update CHANGELOG.md with all changes
   - Update README.md if needed
   - Review all spec documents for accuracy
   - Generate release notes

4. **Build Artifacts**
   - Build reproducible binaries for all platforms
   - Generate checksums (SHA256 + SHA512)
   - Create SBOM (Software Bill of Materials)
   - Generate SLSA provenance

5. **Tag Release Candidate**
   ```bash
   git tag -s v0.1.0-rc.1 -m "Release candidate 0.1.0-rc.1"
   git push origin v0.1.0-rc.1
   ```

### Testing Period (RC → Final)

- **Minimum 7 days** for minor releases
- **Minimum 14 days** for major releases
- Monitor testnet for issues
- Collect feedback from community
- Fix critical bugs only (bump rc.N)

### Final Release

1. **Create Release Tag**
   ```bash
   git tag -s v0.1.0 -m "Release 0.1.0"
   git push origin v0.1.0
   ```

2. **Build Final Artifacts**
   - Rebuild all binaries with release tag
   - Regenerate all checksums
   - Update SBOM with final version

3. **Sign Artifacts**
   - Sign binaries with maintainer GPG keys (minimum 2/3 signatures)
   - Sign checksum files
   - Sign SBOM

4. **Publish**
   - Create GitHub Release with artifacts
   - Publish checksums and signatures
   - Announce on official channels
   - Update documentation site

5. **Post-Release**
   - Merge release branch back to main
   - Update version to next development version
   - Monitor for critical issues

## Version Numbering

BitQuan uses Semantic Versioning (SemVer):

- **MAJOR**: Incompatible consensus changes (hard fork)
- **MINOR**: Backward-compatible features (soft fork possible)
- **PATCH**: Backward-compatible bug fixes

Examples:
- `v0.0.1-alpha` - Early devnet release
- `v0.1.0-beta.1` - First beta for testnet
- `v0.1.0-rc.2` - Second release candidate
- `v1.0.0` - First stable mainnet release

## Artifact Checksums

All releases include:

1. **Binary artifacts** for Linux, macOS, Windows
2. **SHA256 checksums** (`checksums-sha256.txt`)
3. **SHA512 checksums** (`checksums-sha512.txt`)
4. **GPG signatures** (`*.asc`)
5. **SBOM** in CycloneDX JSON format
6. **SLSA Provenance** (Level 2+)

## Verification

Users can verify releases:

```bash
# Download release and checksums
curl -LO https://github.com/AlphaB135/BitQuan/releases/download/v0.1.0/bitquan-node-linux-x64
curl -LO https://github.com/AlphaB135/BitQuan/releases/download/v0.1.0/checksums-sha256.txt
curl -LO https://github.com/AlphaB135/BitQuan/releases/download/v0.1.0/checksums-sha256.txt.asc

# Verify GPG signature
gpg --verify checksums-sha256.txt.asc checksums-sha256.txt

# Verify checksum
sha256sum -c checksums-sha256.txt
```

## Signing Keys

Maintainer GPG keys are published:
- In repository: [docs/security/keys/maintainers/](docs/security/keys/maintainers/)
- On GitHub: https://github.com/AlphaB135.gpg
- On keyservers: keyserver.ubuntu.com

## Emergency Releases

For critical security fixes:

1. **Immediate patch** on affected versions
2. **Abbreviated RC period** (minimum 48 hours)
3. **Security advisory** published simultaneously
4. **Coordinated disclosure** with exchanges/pools

## Communication

Release announcements:
- GitHub Releases page
- Repository README.md
- Security mailing list (security@bitquan.org)

## Rollback Procedure

If critical issues discovered post-release:

1. Immediately publish security advisory
2. Recommend users stay on previous version
3. Prepare emergency patch release
4. Follow accelerated release process

## See Also

- [SECURITY.md](SECURITY.md) - Security policy
- [REPRODUCIBILITY.md](REPRODUCIBILITY.md) - Reproducible builds
- [CHANGELOG.md](CHANGELOG.md) - Version history
- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guidelines
