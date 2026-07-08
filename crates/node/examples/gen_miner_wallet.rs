//! Generate miner wallet for development/testing
//! Note: Secret key IS encrypted with password, but file is not encrypted at file level

use bitquan_node::wallet::address;
use bitquan_node::wallet::WalletKeypair;
use std::path::Path;

fn main() {
    let wallet = WalletKeypair::generate_dilithium5().expect("Failed to generate wallet");

    let password =
        std::env::var("MINER_PASSWORD").expect("MINER_PASSWORD environment variable must be set");

    wallet
        .save_to_file(Path::new("miner_wallet.json"), &password)
        .expect("Failed to save wallet");

    let pubkey_hash = wallet.public_key_hash();
    let addr = address::encode(&pubkey_hash);

    println!("✅ Miner wallet saved to miner_wallet.json");
    println!("📍 Address: {}", addr);
    println!("⚠️  Password read from MINER_PASSWORD env var");
}
