//! Basic BitQuan SDK example
//! 
//! This example demonstrates basic usage of the BitQuan SDK including:
//! - Creating a wallet
//! - Connecting to the blockchain
//! - Querying account information
//! - Building and sending transactions

use bitquan_sdk::{BitQuanClient, WalletManager, Network, TransactionBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 BitQuan SDK Example");
    println!("=====================");

    // 1. Create wallet manager
    println!("\n📱 Creating wallet manager...");
    let mut wallet_manager = WalletManager::new(Network::Testnet);
    
    // 2. Create a new wallet
    println!("🔑 Creating new wallet...");
    let address = wallet_manager.create_wallet("example-wallet")?;
    println!("   Address: {}", address);
    
    // 3. Get the wallet
    let wallet = wallet_manager.get_default_wallet().unwrap();
    println!("   Private Key: {}", wallet.private_key());
    println!("   Public Key: {}", wallet.public_key());

    // 4. Create blockchain client
    println!("\n🌐 Connecting to blockchain...");
    let client = BitQuanClient::new_with_url("https://testnet.bitquan.network/rpc");
    
    // 5. Get blockchain info
    println!("📊 Getting blockchain info...");
    match client.get_blockchain_info().await {
        Ok(info) => {
            println!("   Chain: {}", info.chain);
            println!("   Blocks: {}", info.blocks);
            println!("   Best Block Hash: {}", info.best_block_hash);
        }
        Err(e) => {
            println!("   ⚠️  Failed to get blockchain info: {}", e);
            println!("   (This is expected if the RPC endpoint is not available)");
        }
    }

    // 6. Get account information
    println!("\n💰 Getting account info...");
    match client.get_account(&address).await {
        Ok(account) => {
            println!("   Address: {}", account.address);
            println!("   Balance: {}", account.balance);
            println!("   Nonce: {}", account.nonce);
        }
        Err(e) => {
            println!("   ⚠️  Failed to get account info: {}", e);
            println!("   (This is expected for new addresses)");
        }
    }

    // 7. Build a transaction
    println!("\n📝 Building transaction...");
    let tx_result = TransactionBuilder::new()
        .add_input("previous_tx_hash", 0)
        .add_output("bq1recipient_address_example", 1000)
        .add_output(&address, 500) // Change output
        .build();
    
    match tx_result {
        Ok(tx) => {
            println!("   Transaction built successfully!");
            println!("   Hash: {}", tx.hash());
            println!("   Size: {} bytes", tx.size());
            println!("   Fee: {} satoshis", tx.fee());
            println!("   Outputs: {}", tx.outputs.len());
            
            // Sign the transaction
            let mut signed_tx = tx.clone();
            if let Err(e) = signed_tx.sign(wallet.private_key()) {
                println!("   ⚠️  Failed to sign transaction: {}", e);
            } else {
                println!("   ✅ Transaction signed successfully!");
            }
        }
        Err(e) => {
            println!("   ❌ Failed to build transaction: {}", e);
        }
    }

    // 8. Demonstrate cryptographic operations
    println!("\n🔐 Cryptographic operations...");
    let message = "Hello, BitQuan!";
    let signature = wallet.sign_message(message)?;
    println!("   Message: {}", message);
    println!("   Signature: {}", signature);
    println!("   Verified: {}", wallet.verify_message(message, &signature));

    // 9. Export wallet
    println!("\n💾 Exporting wallet...");
    let export = wallet.export()?;
    println!("   Export format: Address: {}, Network: {:?}", export.address, export.network);

    println!("\n✅ Example completed successfully!");
    Ok(())
}