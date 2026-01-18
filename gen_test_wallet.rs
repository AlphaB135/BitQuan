use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This script is for testing only
    // In production, wallets should be created using the proper wallet commands

    println!("Creating test wallet at miner_wallet.json");

    // For regtest testing, we can use the simple wallet format
    // Load from my_wallet.json and copy it

    let source = Path::new("./crates/wallet/my_wallet.json");
    let dest = Path::new("./miner_wallet.json");

    if !source.exists() {
        println!("Source wallet not found at {:?}", source);
        return Ok(());
    }

    std::fs::copy(source, dest)?;
    println!("Copied wallet from {:?} to {:?}", source, dest);

    Ok(())
}
