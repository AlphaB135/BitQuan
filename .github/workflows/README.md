# BitQuan CI/CD Workflows

This directory contains enhanced CI/CD workflows for BitQuan. The workflows provide comprehensive automation for testing, security, performance, and documentation.

## Overview

The enhanced CI/CD pipeline includes:

### 1. Security Scanning (`security-scan.yml`)
- Comprehensive security audit with `cargo-audit`
- Dependency checking with `cargo-deny`
- Secret scanning with TruffleHog
- Malware detection with ClamAV and YARA
- OSSF Security Scorecard analysis
- Fuzz testing and property-based testing

### 2. Benchmark Testing (`benchmark.yml`)
- Regression detection with `cargo-benchcmp`
- Performance benchmarking with `cargo-criterion`
- Integration benchmarks
- Performance testing with flamegraphs
- Comparison with historical benchmarks

### 3. Enhanced CI Pipeline (`enhanced-ci.yml`)
- Quick checks for PRs (formatting, linting, unit tests)
- Full test suite on multiple platforms (Linux, macOS, Windows)
- Cross-platform build matrix
- Docker build with multi-arch support
- Code coverage reporting
- Documentation building
- Quality gate checks

### 4. Release Notes Generation (`release-notes.yml`)
- Automated changelog generation
- Commit analysis
- PR-based release notes
- Support for tag-based releases
- Dry run capability

### 5. Multi-Platform Docker Build (`docker-multiplatform.yml`)
- Build for multiple architectures (AMD64, ARM64, ARMv7)
- Security scanning with Trivy and Clair
- Documentation generation
- Integration testing with Docker Compose
- Automated deployment to GitHub Container Registry

### 6. Integration Tests (`integration-tests.yml`)
- Basic integration tests
- Network integration tests
- Multi-node testing
- Database integration (PostgreSQL, Redis, SQLite)
- Wallet integration tests
- Performance integration tests
- Stress testing
- Security integration tests

### 7. Performance Benchmarks (`performance-benchmarks.yml`)
- Consensus benchmarks
- Crypto benchmarks
- Mempool benchmarks
- Memory profiling with Valgrind
- Flamegraph generation
- Load testing with Locust
- Database performance testing
- Regression detection

### 8. Documentation Build (`docs.yml`)
- Rust documentation generation
- mdBook documentation
- API documentation generation
- Documentation linting
- Link checking
- GitHub Pages deployment
- Preview generation for PRs

## Workflow Triggers

### Automatic Triggers
- **Push**: All workflows run on pushes to main, develop, and feature branches
- **Pull Requests**: Most workflows run on PRs targeting main/develop
- **Schedule**: Daily/nightly runs for heavy tasks (security, benchmarks)
- **Release**: Special handling on tag pushes

### Manual Triggers
- **Workflow Dispatch**: All workflows can be triggered manually
  - Platform selection for Docker builds
  - Version specification for release notes
  - Dry run options

## Quality Gates

The pipeline implements strict quality gates:

### Required Checks
- ✅ Format check (`cargo fmt`)
- ✅ Linting (`cargo clippy`)
- ✅ Unit tests
- ✅ Integration tests
- ✅ Security scan
- ✅ Build on all target platforms

### Optional Checks
- 📊 Code coverage (reported but not blocking)
- 🔍 Documentation (deployed but not blocking)
- 🚀 Performance benchmarks (monitored but not blocking)

## Performance

### Optimization Strategies
- **Caching**: Cargo registry and target directory caching
- **Parallel Execution**: Matrix builds run in parallel
- **Selective Execution**: Only run necessary jobs based on changes
- **Timeout Management**: Appropriate timeouts for each job type

### Resource Usage
- **Standard jobs**: 30-60 minutes
- **Heavy jobs**: 90-180 minutes
- **Memory**: Up to 4GB for some jobs
- **Storage**: Optimized with artifact retention policies

## Security

### Security Features
- **Secret Scanning**: Automatic detection of sensitive data
- **Dependency Scanning**: Vulnerability detection in dependencies
- **Code Analysis**: Security-focused linting rules
- **Container Scanning**: Trivy and Clair for Docker images
- **SBOM Generation**: Software Bill of Materials

### Security Best Practices
- Non-root user in Docker containers
- Regular security scans
- Dependency pinning and updating
- Access control and permissions

## Monitoring and Alerts

### Notification System
- **Slack Integration**: Security alerts and failures
- **GitHub Issues**: Comment on PRs with test results
- **Email Notifications**: On critical failures
- **Dashboard**: Real-time status display

### Reporting
- **Artifact Storage**: All artifacts stored for 30-365 days
- **Test Reports**: Detailed test results and coverage
- **Performance Reports**: Benchmark results and regressions
- **Security Reports**: Vulnerability findings and fixes

## Configuration

### Environment Variables
```yaml
env:
  CARGO_TERM_COLOR: always
  SOURCE_DATE_EPOCH: 1700000000
  RUSTFLAGS: -D warnings
  CARGO_BUILD_INCREMENTAL: false
```

### Secrets
- `GITHUB_TOKEN`: For GitHub API access
- `SLACK_WEBHOOK`: For notifications
- Registry credentials for Docker deployment

## Future Enhancements

### Planned Features
1. **Machine Learning**: Performance anomaly detection
2. **Canary Deployments**: Gradual rollouts with automated testing
3. **Interactive Testing**: UI-based integration tests
4. **Automated Updates**: Dependency update automation
5. **Branch Protection**: Enhanced branch rules

### Optimization Opportunities
- **Caching Strategy**: More granular cache keys
- **Parallelism**: Improved matrix job scheduling
- **Resource Management**: Better resource allocation
- **Cost Reduction**: Optimized runner usage

## Maintenance

### Updating Dependencies
- Update Rust toolchain versions quarterly
- Update GitHub Actions monthly
- Update Docker images regularly
- Update security tools as new versions are released

### Cleanup
- Old artifacts are automatically cleaned up
- Workflow runs are pruned after 90 days
- Documentation deployments are updated on each release

## Contributing

When contributing to BitQuan, ensure:

1. **CI/CD**: All workflows must pass
2. **Documentation**: Update relevant docs
3. **Tests**: Add tests for new features
4. **Security**: Follow security guidelines
5. **Performance**: Consider performance impact

## Troubleshooting

### Common Issues
1. **Timeouts**: Increase timeout for heavy jobs
2. **Cache Misses**: Update cache keys
3. **Resource Limits**: Request more resources if needed
4. **Network Issues**: Retry failed downloads

### Debug Commands
```bash
# Check workflow status
gh run list --limit 5

# View logs
gh run view <run-id> --log

# Download artifacts
gh run download <run-id>

# Manual trigger
gh workflow run <workflow-file.yml>
```

For more information, see the [BitQuan Documentation](https://docs.bitquan.com).