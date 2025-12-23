//! Multi-signature wallet demo
//!
//! Run with: cargo run --example multisig_demo

use wallet::multisig::{MultisigConfig, MultisigWallet, MultisigWalletManager};

fn main() {
    println!("=== BitQuan Multi-Signature Wallet Demo ===\n");

    // Example 1: Create a 2-of-3 multisig wallet
    println!("1. Creating a 2-of-3 multisig wallet...");
    let public_keys = vec![
        "dilithium5_pubkey_alice_0x1234...".to_string(),
        "dilithium5_pubkey_bob_0x5678...".to_string(),
        "dilithium5_pubkey_charlie_0xabcd...".to_string(),
    ];

    #[allow(clippy::expect_used)]
    let config = MultisigConfig::new(2, public_keys.clone(), Some("Team Wallet".to_string()))
        .expect("Failed to create config");

    println!("   Config: {}", config.config_type());
    println!("   Address: {}", config.address());
    println!("   Signers: {}", config.total_signers);
    println!();

    // Example 2: Create wallet and pending transaction
    println!("2. Creating a pending transaction...");
    let wallet = MultisigWallet::new(config.clone());
    let tx_data = b"Transfer 100 BQ to bq1qxyz...";
    let mut pending = wallet.create_pending_tx(tx_data);

    println!("   Transaction ID: {}", pending.tx_id);
    println!("   Signatures needed: {}", pending.signatures_needed());
    println!("   Progress: {:.0}%", pending.progress_percentage());
    println!();

    // Example 3: First signature (Alice)
    println!("3. Alice signs the transaction...");
    let alice_signature = b"dilithium5_sig_alice_..."; // In real use, generate with keypair.sign()
    #[allow(clippy::expect_used)]
    wallet
        .add_signature(&mut pending, &public_keys[0], alice_signature)
        .expect("Failed to add Alice's signature");

    println!("   ✓ Alice signed");
    println!(
        "   Signatures: {}/{}",
        pending.signature_count(),
        config.required_sigs
    );
    println!("   Progress: {:.0}%", pending.progress_percentage());
    println!(
        "   Still need signatures from: {:?}",
        pending.pending_signers()
    );
    println!();

    // Example 4: Second signature (Bob)
    println!("4. Bob signs the transaction...");
    let bob_signature = b"dilithium5_sig_bob_...";
    #[allow(clippy::expect_used)]
    wallet
        .add_signature(&mut pending, &public_keys[1], bob_signature)
        .expect("Failed to add Bob's signature");

    println!("   ✓ Bob signed");
    println!(
        "   Signatures: {}/{}",
        pending.signature_count(),
        config.required_sigs
    );
    println!("   Progress: {:.0}%", pending.progress_percentage());
    println!("   Transaction complete: {}", pending.is_complete());
    println!();

    // Example 5: Finalize transaction
    println!("5. Finalizing transaction...");
    #[allow(clippy::expect_used)]
    let finalized = wallet
        .finalize_transaction(&pending)
        .expect("Failed to finalize");

    println!("   ✓ Transaction finalized!");
    println!("   Final signature count: {}", finalized.signatures.len());
    #[allow(clippy::expect_used)]
    finalized.verify().expect("Verification failed");
    println!("   ✓ Verification passed");
    println!();

    // Example 6: Using MultisigWalletManager
    println!("6. Using MultisigWalletManager...");
    let mut manager = MultisigWalletManager::new();

    // Add wallet
    let wallet2 = MultisigWallet::new(config.clone());
    manager.add_wallet(wallet2);

    // Create and track pending transaction
    let pending2 = wallet.create_pending_tx(b"Another transaction");
    manager.add_pending_tx(pending2);

    println!("   Wallets managed: {}", manager.list_addresses().len());
    println!(
        "   Pending transactions: {}",
        manager.list_pending_txs().len()
    );
    println!(
        "   Incomplete transactions: {}",
        manager.list_incomplete_txs().len()
    );
    println!();

    // Example 7: Different multisig configurations
    println!("7. Different multisig configurations:");

    let configs = vec![
        ("1-of-1 (single signature)", 1, 1),
        ("2-of-2 (both must sign)", 2, 2),
        ("2-of-3 (any 2 of 3)", 2, 3),
        ("3-of-5 (any 3 of 5)", 3, 5),
        ("4-of-7 (any 4 of 7)", 4, 7),
    ];

    for (name, required, total) in configs {
        let keys: Vec<String> = (0..total).map(|i| format!("pubkey_{}", i)).collect();

        let cfg = MultisigConfig::new(required, keys, None).expect("Failed to create config");

        println!("   {} -> Address: {}", name, cfg.address());
    }
    println!();

    // Example 8: Error handling
    println!("8. Error handling examples:");

    // Try to add duplicate signature
    let mut pending3 = wallet.create_pending_tx(b"test");
    wallet
        .add_signature(&mut pending3, &public_keys[0], b"sig1")
        .ok();
    match wallet.add_signature(&mut pending3, &public_keys[0], b"sig2") {
        Err(e) => println!("   ✓ Duplicate signature rejected: {}", e),
        _ => println!("   ✗ Should have rejected duplicate signature"),
    }

    // Try to add unknown signer
    match wallet.add_signature(&mut pending3, "unknown_key", b"sig") {
        Err(e) => println!("   ✓ Unknown signer rejected: {}", e),
        _ => println!("   ✗ Should have rejected unknown signer"),
    }

    // Try to finalize with insufficient signatures
    match wallet.finalize_transaction(&pending3) {
        Err(e) => println!("   ✓ Insufficient signatures: {}", e),
        _ => println!("   ✗ Should have rejected insufficient signatures"),
    }
    println!();

    println!("=== Demo Complete ===");
    println!("\nKey Features:");
    println!("  ✓ M-of-N signature schemes (flexible)");
    println!("  ✓ Duplicate signature prevention");
    println!("  ✓ Unknown signer rejection");
    println!("  ✓ Progress tracking");
    println!("  ✓ Transaction finalization");
    println!("  ✓ Comprehensive error handling");
    println!("\nReady for integration with Dilithium5 signatures!");
}
