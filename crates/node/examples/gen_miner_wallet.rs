//! Generate unencrypted miner wallet for development/testing

use bitquan_node::wallet::address;
use bitquan_node::wallet::WalletKeypair;
use std::path::Path;

fn main() {
    let wallet = WalletKeypair::generate_dilithium5().expect("Failed to generate wallet");

    wallet
        .save_to_file(Path::new("miner_wallet.json"))
        .expect("Failed to save wallet");

    let pubkey_hash = wallet.public_key_hash();
    let addr = address::encode(&pubkey_hash);

    println!("✅ Miner wallet saved to miner_wallet.json");
    println!("📍 Address: {}", addr);
}
