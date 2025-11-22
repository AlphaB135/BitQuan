# BitQuan Post-Quantum Security Documentation

## Overview

BitQuan implements comprehensive post-quantum security features to protect against quantum computing threats. The system combines NIST-standardized post-quantum cryptography with advanced memory protection and constant-time operations.

## Core Post-Quantum Components

### 1. Dilithium3 Digital Signatures

**Location**: `crates/pqc-dilithium-seeded/`, `crates/crypto/src/lib.rs:102-139`

BitQuan uses **Dilithium3**, a NIST-standardized post-quantum digital signature algorithm based on module lattice problems.

#### Key Properties:
- **Security Level**: Dilithium3 provides ~128-bit quantum security
- **Signature Size**: 3,293 bytes
- **Public Key Size**: 1,952 bytes
- **Standardization**: NIST PQC FIPS 204 (draft)

#### Implementation Details:
```rust
// Signature verification with size validation
const DILITHIUM3_SIG_SIZE: usize = 3293;
const DILITHIUM3_PK_SIZE: usize = 1952;

// Constant-time verification using patched pqc_dilithium
dilithium::crypto_sign_verify(&sig_bytes, message, &pk_bytes)
```

#### Security Features:
- **Deterministic Signing**: Prevents timing side-channel attacks
- **Message Size Limits**: Prevents DoS attacks (>1MB messages rejected)
- **Input Validation**: Strict size checking for signatures and public keys

### 2. Secure Key Management

**Location**: `crates/crypto/src/wallet/secure_types.rs`

#### SecurePrivateKey Implementation:
- **Memory Locking**: Uses `mlock()` on Unix systems to prevent swapping
- **Zeroization**: Automatic memory clearing on drop using `ZeroizeOnDrop`
- **Constant-Time Operations**: All comparisons execute in constant time
- **Secure Allocation**: Custom `SecureAllocator` for sensitive data

```rust
// Memory locking on Unix systems
#[cfg(all(unix, feature = "memory-locking"))]
fn lock_memory(&mut self) -> Result<(), std::io::Error> {
    let result = unsafe { mlock(ptr, len) };
    // Prevents swapping to disk
}
```

#### Key Security Features:
- **Constant-Time Comparison**: `constant_time_eq()` prevents timing attacks
- **Secure Hash Verification**: SHA-256 based key verification
- **Memory Protection**: Locked memory prevents swap exposure
- **Automatic Cleanup**: Zeroization on drop and deallocation

### 3. Advanced Encryption

**Location**: `crates/crypto/src/wallet/encryption.rs`

#### AES-256-GCM Encryption:
- **Authenticated Encryption**: Provides confidentiality and integrity
- **Key Derivation**: Argon2id for password-based key derivation
- **Random Nonces**: 96-bit cryptographically secure nonces
- **Key Zeroization**: Immediate cleanup after use

```rust
// AES-256-GCM with Argon2id KDF
let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
let ciphertext = cipher.encrypt(nonce, plaintext)?;
key_bytes.zeroize(); // Immediate cleanup
```

#### Encryption Security Features:
- **Strong KDF**: Argon2id with configurable parameters (64 MiB memory, 3 iterations)
- **Perfect Forward Secrecy**: Unique salts for each encryption
- **Authentication**: GCM tag prevents tampering
- **Side-Channel Protection**: Constant-time operations

### 4. Constant-Time Operations

**Location**: `crates/crypto/src/constant_time.rs`

#### Comprehensive Constant-Time Library:
- **Memory Operations**: `constant_time_memcpy()`, `constant_time_zeroize()`
- **Comparisons**: `constant_time_eq()`, `constant_time_hash_eq()`
- **Selection**: `constant_time_select()` for conditional operations
- **Arithmetic**: `constant_time_min()`, `constant_time_max()`

```rust
// Prevents timing attacks in cryptographic operations
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into() // Uses subtle crate for constant-time guarantees
}
```

### 5. Post-Quantum Key Derivation

**Location**: `crates/crypto/src/wallet/kdf.rs`

#### Argon2id Configuration:
- **Memory Cost**: 65,536 KiB (64 MiB)
- **Time Cost**: 3 iterations
- **Parallelism**: 4 threads
- **Salt Length**: 32 bytes (cryptographically secure)

```rust
// Server-grade Argon2id parameters
let kdf = KeyDerivation {
    memory_cost_kib: 65_536, // 64 MiB
    time_cost: 3,
    parallelism: 4,
};
```

## Integration Testing

**Location**: `tests/pqc_integration_test.rs`

Comprehensive test suite validates:
- **End-to-End Flow**: Key generation → encryption → signing → verification
- **Multi-threaded Security**: Concurrent signing with cache testing
- **Cache Performance**: Memory caching with timeout management
- **Error Handling**: Wrong password and key rejection
- **Performance Benchmarks**: Timing measurements and cache speedup

### Test Results:
- **Signature Generation**: ~2-5ms per signature
- **Verification**: ~1-3ms per verification
- **Cache Speedup**: 10-100x faster repeated operations
- **Memory Usage**: Configurable cache with automatic cleanup

## Security Architecture

### Defense in Depth:

1. **Post-Quantum Primitives**: Dilithium3 signatures resistant to quantum attacks
2. **Memory Protection**: Locked memory prevents swap exposure
3. **Side-Channel Resistance**: Constant-time operations prevent timing attacks
4. **Strong Encryption**: AES-256-GCM with Argon2id KDF
5. **Secure Key Management**: Zeroization and secure allocation
6. **Input Validation**: Size limits and format checking

### Threat Mitigation:

| Threat | Mitigation |
|--------|------------|
| Quantum Computing | Dilithium3 lattice-based signatures |
| Timing Attacks | Constant-time operations |
| Memory Dump Attacks | Memory locking and zeroization |
| Side-Channel Attacks | Deterministic signing, constant-time ops |
| Brute Force | Argon2id with high memory cost |
| Tampering | AES-GCM authentication tags |

## Configuration Options

### Security Levels:
- **Server Mode**: Maximum security, no caching
- **Performance Mode**: Caching enabled for speed
- **Custom**: Configurable cache timeout and memory limits

### Feature Flags:
- `memory-locking`: Enable mlock() for memory protection
- `memory-security`: Full memory security suite
- `deterministic_tests`: Reproducible test results

## Compliance and Standards

- **NIST PQC**: Dilithium3 standardized in FIPS 204 (draft)
- **FIPS 140-2**: Uses validated cryptographic primitives
- **Common Criteria**: Lattice-based cryptography evaluation
- **ISO/IEC 14888**: Digital signature mechanisms

## Performance Characteristics

### Benchmarks:
- **Key Generation**: ~10-20ms
- **Signing**: ~2-5ms
- **Verification**: ~1-3ms
- **Encryption**: ~1-2ms for 1KB data
- **Decryption**: ~1-2ms for 1KB data

### Memory Usage:
- **Private Key**: 4KB (including overhead)
- **Public Key**: 2KB
- **Signature**: 3.3KB
- **Cache**: Configurable, typically 10-100MB

## Future Enhancements

### Roadmap:
1. **Hybrid Signatures**: Combine Dilithium with ECDSA for backward compatibility
2. **Additional PQC Algorithms**: Support for Falcon, SPHINCS+
3. **Hardware Acceleration**: Intel IPP and ARM Crypto extensions
4. **Formal Verification**: Mathematical proofs of security properties
5. **Audit Trail**: Comprehensive logging for compliance

## Security Recommendations

### For Production Deployment:
1. **Enable Memory Locking**: Use `memory-locking` feature flag
2. **Server Configuration**: Use `WalletConfig::server()` for maximum security
3. **Regular Updates**: Keep PQC libraries updated
4. **Monitoring**: Monitor cache usage and performance metrics
5. **Backup Security**: Secure backup of encrypted keystores

### For Development:
1. **Test Coverage**: Run full PQC integration test suite
2. **Security Review**: Regular code audits and penetration testing
3. **Performance Testing**: Benchmark under realistic workloads
4. **Memory Analysis**: Verify no memory leaks or timing variations

## Conclusion

BitQuan's post-quantum security implementation provides comprehensive protection against current and future threats. The combination of NIST-standardized Dilithium3 signatures, advanced memory protection, and constant-time operations creates a robust security foundation for quantum-resistant blockchain operations.

The modular design allows for future enhancements while maintaining backward compatibility, ensuring BitQuan remains secure as quantum computing capabilities evolve.
