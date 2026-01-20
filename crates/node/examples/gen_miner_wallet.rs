//! Generate miner wallet for development/testing
//! Note: Secret key IS encrypted with password, but file is not encrypted at file level

use bitquan_node::wallet::address;
use bitquan_node::wallet::WalletKeypair;
use std::path::Path;

fn main() {
    let wallet = WalletKeypair::generate_dilithium5().expect("Failed to generate wallet");

    // Use default password for development (CHANGE THIS IN PRODUCTION!)
    let password = "miner_dev_password";

    wallet
        .save_to_file(Path::new("miner_wallet.json"), password)
        .expect("Failed to save wallet");

    let pubkey_hash = wallet.public_key_hash();
    let addr = address::encode(&pubkey_hash);

    println!("✅ Miner wallet saved to miner_wallet.json");
    println!("📍 Address: {}", addr);
    println!("⚠️  Password: {}", password);
}
