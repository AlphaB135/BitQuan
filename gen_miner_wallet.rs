use bitquan_node::wallet::WalletKeypair;
use std::path::Path;

fn main() {
    let wallet = WalletKeypair::new_for_network(bitquan_types::NetworkId::Regtest)
        .expect("Failed to generate wallet");
    
    wallet.save_to_file(Path::new("miner_wallet.json"))
        .expect("Failed to save wallet");
    
    println!("Miner wallet saved to miner_wallet.json");
    println!("Address: {}", wallet.address());
}
