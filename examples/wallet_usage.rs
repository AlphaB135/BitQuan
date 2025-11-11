//! # BitQuan Wallet Examples
//!
//! This file demonstrates common usage patterns for the BitQuan wallet library.

use std::time::Duration;
use wallet::keystore::{
    cleanup_expired_cache, decrypt_keystore, decrypt_keystore_with_config,
    encrypt_keystore_adaptive, encrypt_keystore_with_config, get_cache_memory_usage,
    get_cache_stats, WalletConfig,
};

/// Example 1: Basic usage (recommended for most users)
fn basic_usage_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Basic Usage Example ===");

    // Encrypt some sensitive data (e.g., private key)
    let private_key = b"my_super_secret_private_key_12345";
    let password = "my-very-strong-password-ABC!123";

    let keystore = encrypt_keystore_adaptive(private_key, password, None);
    println!("✅ Data encrypted successfully");

    // Decrypt it back
    let start = std::time::Instant::now();
    let decrypted = decrypt_keystore(&keystore, password)?;
    let first_time = start.elapsed();

    println!("✅ First decryption: {:?} (cold cache)", first_time);
    assert_eq!(decrypted, private_key);

    // Decrypt again (should be much faster)
    let start = std::time::Instant::now();
    let decrypted2 = decrypt_keystore(&keystore, password)?;
    let second_time = start.elapsed();

    println!("✅ Second decryption: {:?} (hot cache)", second_time);
    assert_eq!(decrypted2, private_key);

    // Show performance improvement
    let speedup = first_time.as_nanos() as f64 / second_time.as_nanos() as f64;
    println!("🚀 Speedup: {:.0}x faster", speedup);

    Ok(())
}

/// Example 2: Server configuration (maximum security)
fn server_security_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Server Security Example ===");

    let sensitive_data = b"server_master_seed_phrase";
    let server_password = "server-master-password-2024!";

    // Use server configuration: maximum security, no caching
    let server_config = WalletConfig::server();
    let keystore =
        encrypt_keystore_with_config(sensitive_data, server_password, None, &server_config);

    println!("✅ Data encrypted with server-grade security");

    // Decrypt (always performs full KDF, no caching)
    let start = std::time::Instant::now();
    let decrypted = decrypt_keystore_with_config(&keystore, server_password, &server_config)?;
    let duration = start.elapsed();

    println!("✅ Decrypted in {:?} (no cache for security)", duration);
    assert_eq!(decrypted, sensitive_data);

    Ok(())
}

/// Example 3: Mobile configuration (battery optimized)
fn mobile_optimization_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Mobile Optimization Example ===");

    let mobile_data = b"user_wallet_private_key";
    let user_password = "user_secure_password_123";

    // Mobile configuration: balanced for battery life
    let mobile_config = WalletConfig::mobile().with_cache_timeout(Duration::from_secs(60)); // Short cache

    let keystore = encrypt_keystore_with_config(mobile_data, user_password, None, &mobile_config);

    println!("✅ Data encrypted with mobile-optimized settings");

    // Decrypt with mobile config
    let decrypted = decrypt_keystore_with_config(&keystore, user_password, &mobile_config)?;
    assert_eq!(decrypted, mobile_data);

    println!("✅ Mobile decryption successful");

    Ok(())
}

/// Example 4: Cache monitoring for production
fn cache_monitoring_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Cache Monitoring Example ===");

    // Create multiple keystores to populate cache
    let passwords = ["password1", "password2", "password3"];
    let mut keystores = Vec::new();

    for pwd in &passwords {
        let ks = encrypt_keystore_adaptive(b"test_data", pwd, None);
        keystores.push(ks);
    }

    println!("✅ Created {} keystores", keystores.len());

    // Decrypt all to populate cache
    for (i, ks) in keystores.iter().enumerate() {
        decrypt_keystore(ks, passwords[i])?;
    }

    // Monitor cache statistics
    let stats = get_cache_stats();
    let memory_bytes = get_cache_memory_usage();

    println!("📊 Cache Statistics:");
    println!("   Total entries: {}", stats.total_entries);
    println!("   Active entries: {}", stats.active_entries);
    println!("   Expired entries: {}", stats.expired_entries);
    println!(
        "   Memory usage: {} bytes ({} KB)",
        memory_bytes,
        memory_bytes / 1024
    );

    // Cleanup expired entries (if any)
    cleanup_expired_cache();
    println!("🧹 Cache cleanup completed");

    Ok(())
}

/// Example 5: Error handling
fn error_handling_example() {
    println!("\n=== Error Handling Example ===");

    let keystore = encrypt_keystore_adaptive(b"secret", "correct_password", None);

    // Try wrong password
    match decrypt_keystore(&keystore, "wrong_password") {
        Ok(_) => println!("❌ This should not happen!"),
        Err(e) => println!("✅ Wrong password correctly rejected: {}", e),
    }

    // Correct password
    match decrypt_keystore(&keystore, "correct_password") {
        Ok(data) => println!("✅ Correct password accepted: {} bytes", data.len()),
        Err(e) => println!("❌ Unexpected error: {}", e),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 BitQuan Wallet Library Examples\n");

    basic_usage_example()?;
    server_security_example()?;
    mobile_optimization_example()?;
    cache_monitoring_example()?;
    error_handling_example();

    println!("\n✨ All examples completed successfully!");
    println!("\n📚 Key Takeaways:");
    println!("   • Use encrypt_keystore_adaptive() for most applications");
    println!("   • Use WalletConfig::server() for maximum security");
    println!("   • Use WalletConfig::mobile() for battery-optimized apps");
    println!("   • Monitor cache usage in production with get_cache_stats()");
    println!("   • Cache provides ~5,000x speedup for repeated operations");

    Ok(())
}
