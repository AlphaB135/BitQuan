//! Wallet commands for BitQuan CLI
//!
//! This module contains all wallet-related commands:
//! - wallet_gen, wallet_address, wallet_send
//! - wallet_backup, wallet_restore
//! - wallet_gen_mnemonic, wallet_from_mnemonic
//! - tx_sign_partial, tx_combine_signatures
//! - wallet_sign, wallet_verify
//! - wallet_gen_multisig, multisig_info

use crate::address::{self};
use crate::cli::{format_bq, invalid};
use crate::keystore;
use crate::wallet::{SerializableKeypair, WalletKeypair};
use bitquan_storage::ChainStore;
use bitquan_types::error::{Error, Result};
use pqc_dilithium_seeded::{PUBLICKEYBYTES, SECRETKEYBYTES};
use serde_json;
use std::io::Write;
use std::path::Path;
/// Generate a wallet keypair with encrypted storage
pub fn wallet_gen(
    algo: &str,
    network: &str,
    output_path: Option<&str>,
    password: Option<&str>,
) -> Result<()> {
    println!("BitQuan Wallet Generator");
    println!("Algorithm: {}", algo);
    println!("Network: {}", network);

    if algo != "dilithium5" {
        return invalid("Only 'dilithium5' is supported currently");
    }

    println!("\n⏳ Generating keypair...");
    let keypair = WalletKeypair::generate_dilithium5()?;

    let pubkey_hash = keypair.public_key_hash();
    let address_str = address::encode(&pubkey_hash);

    println!("\nKeypair generated successfully!");
    println!("\n📍 Address: {}", address_str);
    println!("🔑 Public key hash: {}", hex::encode(pubkey_hash));
    println!("📏 Public key: {} bytes", PUBLICKEYBYTES);
    println!("📏 Secret key: {} bytes", SECRETKEYBYTES);

    // Get password for encryption
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("\n🔒 Enter password to encrypt keystore:");
            crate::cli::read_password_from_stdin()?
        }
    };

    if password.len() < 8 {
        return invalid("Password must be at least 8 characters");
    }

    // Serialize keypair metadata with encrypted secret key
    let serializable = keypair.to_serializable(&password);
    let json = serde_json::to_string_pretty(&serializable)?;

    // Add network prefix to address for clear identification
    let network_address = format!("{}:{}", network, address_str);

    // Encrypt and save using existing function with network-prefixed address
    let keystore_file = keystore::encrypt_keypair(&json, &password, &network_address)
        .map_err(|e| Error::Invalid(format!("keystore encrypt failed: {e}")))?;

    let default_filename = match network {
        "mainnet" => "mainnet-wallet.keystore",
        "testnet" => "testnet-wallet.keystore",
        "devnet" => "devnet-wallet.keystore",
        "regtest" => "regtest-wallet.keystore",
        _ => "wallet.keystore",
    };
    let path = output_path.unwrap_or(default_filename);
    keystore::save_keystore(&keystore_file, Path::new(path))
        .map_err(|e| Error::Invalid(format!("keystore save failed: {e}")))?;

    println!("\nEncrypted keystore saved to: {}", path);
    println!("\nIMPORTANT:");
    println!("  - Keep this file safe!");
    println!("  - Remember your password!");
    println!("  - Make backups!");
    println!("\nNote: Keypair metadata persisted (address, pubkey hash)");
    println!("  Full signing requires session keypair due to pqc_dilithium 0.2 limitations");

    Ok(())
}

/// Show wallet address from encrypted keystore
pub fn wallet_address(keystore_path: &str, password: Option<&str>) -> Result<()> {
    println!("BitQuan Wallet Address");
    println!("Loading keystore from: {}", keystore_path);

    // Load keystore
    let keystore_file = keystore::load_keystore(Path::new(keystore_path))
        .map_err(|e| Error::Invalid(format!("keystore load failed: {e}")))?;

    // Get password
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("\n🔒 Enter password:");
            crate::cli::read_password_from_stdin()?
        }
    };

    // Decrypt
    let json = keystore::decrypt_keypair(&keystore_file, &password)
        .map_err(|e| Error::Invalid(format!("keystore decrypt failed: {e}")))?;
    let data: SerializableKeypair = serde_json::from_str(&json)?;

    println!("\n📍 Address: {}", data.address);
    println!("🔑 Public key hash: {}", data.public_key_hash);
    println!("📏 Metadata only (full keys require session keypair)");

    Ok(())
}

/// Sign a message with encrypted wallet keypair
pub fn wallet_sign(keystore_path: &str, message_hex: &str, password: Option<&str>) -> Result<()> {
    println!("BitQuan Wallet Sign");
    println!("Keystore: {}", keystore_path);

    let message = hex::decode(message_hex)
        .map_err(|e| Error::Invalid(format!("invalid message hex: {e}")))?;
    println!("Message: {} ({} bytes)", message_hex, message.len());

    // Load keystore
    let keystore_file = keystore::load_keystore(Path::new(keystore_path))
        .map_err(|e| Error::Invalid(format!("keystore load failed: {e}")))?;

    // Get password
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("\n🔒 Enter password:");
            crate::cli::read_password_from_stdin()?
        }
    };

    // Decrypt keystore
    println!("\n⏳ Decrypting keystore...");
    let json = keystore::decrypt_keypair(&keystore_file, &password)
        .map_err(|e| Error::Invalid(format!("keystore decrypt failed: {e}")))?;
    let data: SerializableKeypair = serde_json::from_str(&json)?;

    println!("Keystore decrypted!");
    println!("📍 Address: {}", data.address);
    println!("🔑 Public key hash: {}", data.public_key_hash);

    // Reconstruct keypair from serialized data
    let keypair = WalletKeypair::from_serializable(&data, &password)
        .map_err(|e| Error::Invalid(format!("keypair reconstruction failed: {e}")))?;

    // Sign the message
    let signature = keypair
        .sign(&message)
        .map_err(|e| Error::Invalid(format!("signing failed: {e}")))?;

    println!("\nMessage signed successfully!");
    println!("📝 Message: {}", message_hex);
    println!("Signature: {}", hex::encode(&signature));
    println!("🔑 Public key: {}", data.public_key);

    Ok(())
}

/// Verify a signature
pub fn wallet_verify(pubkey_hex: &str, message_hex: &str, signature_hex: &str) -> Result<()> {
    use crate::wallet::{WalletAlgorithm, WalletPublicKey};

    println!("BitQuan Wallet Verify");

    let pubkey_bytes = hex::decode(pubkey_hex)
        .map_err(|e| Error::Invalid(format!("invalid public key hex: {e}")))?;
    let message = hex::decode(message_hex)
        .map_err(|e| Error::Invalid(format!("invalid message hex: {e}")))?;
    let signature = hex::decode(signature_hex)
        .map_err(|e| Error::Invalid(format!("invalid signature hex: {e}")))?;

    println!("Public key: {} bytes", pubkey_bytes.len());
    println!("Message: {} bytes", message.len());
    println!("Signature: {} bytes", signature.len());

    let public_key = WalletPublicKey {
        algorithm: WalletAlgorithm::Dilithium5,
        public_key: pubkey_bytes,
    };

    println!();
    println!("Verifying...");
    if public_key.verify(&message, &signature) {
        println!("Signature is VALID!");
        Ok(())
    } else {
        println!("Signature is INVALID!");
        invalid("Signature verification failed")
    }
}

/// Sends funds from a wallet to a specified address.
///
/// Creates a signed transaction and saves it to the pending transactions file
/// for inclusion in the next mined block.
///
/// # Arguments
/// * `keystore_path` - Path to the wallet keystore file
/// * `to_address` - Recipient's BitQuan address (bech32 format)
/// * `amount` - Amount to send in qbits
/// * `fee_rate` - Fee rate in qbits per virtual byte
/// * `password` - Optional password to decrypt the keystore
/// * `datadir` - Data directory containing the blockchain data
pub async fn wallet_send(
    keystore_path: &str,
    to_address: &str,
    amount: u128,
    fee_rate: u64,
    password: Option<&str>,
    datadir: &str,
) -> Result<()> {
    use std::path::Path;

    println!("BitQuan Wallet Send");
    println!("To: {}", to_address);
    println!("Amount: {} qbits ({} BQ)", amount, format_bq(amount));
    println!("Fee rate: {} qbits/WU", fee_rate);
    println!();

    // Load keystore
    let keystore_file = keystore::load_keystore(Path::new(keystore_path))
        .map_err(|e| Error::Invalid(format!("keystore load failed: {e}")))?;

    // Get password
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("Enter password:");
            crate::cli::read_password_from_stdin()?
        }
    };

    // Decrypt keystore
    println!("Decrypting keystore...");
    let json = keystore::decrypt_keypair(&keystore_file, &password)
        .map_err(|e| Error::Invalid(format!("keystore decrypt failed: {e}")))?;
    let data: SerializableKeypair = serde_json::from_str(&json)?;

    // Reconstruct keypair for signing
    let keypair = WalletKeypair::from_serializable(&data, &password)
        .map_err(|e| Error::Invalid(format!("keypair reconstruction failed: {e}")))?;

    // Get recipient script
    let recipient_info = address::inspect(to_address)
        .map_err(|e| Error::Invalid(format!("invalid recipient address: {e}")))?;
    let to_script = address::script_from_pubkey_hash(&recipient_info.payload);

    // Get UTXOs from blockchain
    #[cfg(feature = "rocksdb-backend")]
    {
        use bitquan_storage::RocksDBStore;

        let _storage = RocksDBStore::open(Path::new(datadir))
            .map_err(|e| Error::Invalid(format!("failed to open storage: {e}")))?;

        // Get sender script
        let sender_info = address::inspect(&data.address)
            .map_err(|e| Error::Invalid(format!("invalid sender address: {e}")))?;
        let sender_script = address::script_from_pubkey_hash(&sender_info.payload);

        // For now, use a fixed balance from mining (simplified)
        // In production, this would query UTXOs from storage
        let height = _storage.height().unwrap_or(0);
        println!("🔍 Scanning chain (height {}) for funds...", height);

        let target_amount = amount;
        let fee = fee_rate as u128 * 10_000;
        let total_needed = target_amount.saturating_add(fee);

        let mut collected_value: u128 = 0;
        let mut inputs = Vec::new();

        'scan: for h in 0..=height {
            if let Ok(Some(block)) = _storage.get_block_by_height(h) {
                for tx in &block.transactions {
                    for (vout, output) in tx.outputs.iter().enumerate() {
                        if output.script_pubkey == sender_script {
                            // Check maturity for coinbase
                            let is_coinbase =
                                tx.inputs.len() == 1 && tx.inputs[0].prev_txid == [0u8; 32];

                            if is_coinbase {
                                let maturity = 100;
                                if h + maturity > height {
                                    continue;
                                }
                            }

                            collected_value += output.value;
                            inputs.push(bitquan_types::TxIn {
                                prev_txid: tx.txid(),
                                prev_vout: vout as u32,
                                sequence: u32::MAX,
                                script_sig: Vec::new(),
                            });

                            if collected_value >= total_needed {
                                break 'scan;
                            }
                        }
                    }
                }
            }
        }

        if collected_value < total_needed {
            return invalid(format!(
                "Insufficient funds: found {} qbits, need {} qbits (Note: Coinbase needs 100 blocks maturity)",
                collected_value, total_needed
            ));
        }

        println!(
            "💰 Found {} qbits from {} inputs",
            collected_value,
            inputs.len()
        );

        let mut outputs = vec![bitquan_types::TxOut {
            value: target_amount,
            script_pubkey: to_script,
        }];

        let change_amount = collected_value - total_needed;
        if change_amount > 0 {
            outputs.push(bitquan_types::TxOut {
                value: change_amount,
                script_pubkey: sender_script,
            });
            println!("🔄 Change: {} qbits", change_amount);
        }

        let tx = bitquan_types::Transaction {
            version: 2,
            network: bitquan_types::NetworkId::Mainnet,
            genesis_hash: bitquan_types::genesis::GENESIS_HASH_BYTES,
            lock_time: 0,
            inputs,
            outputs,
            sig_algo: bitquan_types::SigAlgorithm::Dilithium5,
            witnesses: vec![],
        };

        // Serialize transaction for signing (simplified)
        let tx_json = serde_json::to_string(&tx)
            .map_err(|e| Error::Invalid(format!("failed to serialize tx: {e}")))?;
        let tx_bytes = tx_json.as_bytes();

        // Sign transaction
        let signature = keypair
            .sign(tx_bytes)
            .map_err(|e| Error::Invalid(format!("failed to sign tx: {e}")))?;

        // Add witness (simplified)
        let mut signed_tx = tx;
        signed_tx.witnesses = vec![bitquan_types::Witness {
            signatures: vec![bitquan_types::SignaturePayload {
                signer_index: 0,
                signature,
                public_key: keypair.public_key.clone(),
                aux: None,
            }],
        }];

        println!();
        println!("Transaction created and signed!");
        println!("To: {}", to_address);
        println!("Amount: {} qbits ({} BQ)", amount, format_bq(amount));
        println!("Fee: {} qbits", fee);
        println!("Change: {} qbits", change_amount);
        println!();
        println!("📋 Transaction JSON:");
        let tx_json = serde_json::to_string_pretty(&signed_tx)
            .map_err(|e| Error::Invalid(format!("failed to serialize tx json: {e}")))?;
        println!("{}", tx_json);
        println!();

        // Save transaction to pending file (simple local broadcast)
        println!("📡 Saving transaction to pending pool...");

        let data_dir = Path::new(datadir);
        std::fs::create_dir_all(data_dir)
            .map_err(|e| Error::Invalid(format!("failed to create data dir: {e}")))?;

        let pending_path = data_dir.join("pending_transactions.jsonl");

        // Double-spend check: verify no pending tx already uses the same UTXOs
        if pending_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&pending_path) {
                for line in content.lines() {
                    if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(tx_str) = entry.get("tx").and_then(|v| v.as_str()) {
                            if let Ok(existing_tx) =
                                serde_json::from_str::<bitquan_types::Transaction>(tx_str)
                            {
                                for input in &signed_tx.inputs {
                                    for existing_input in &existing_tx.inputs {
                                        if input.prev_txid == existing_input.prev_txid
                                            && input.prev_vout == existing_input.prev_vout
                                        {
                                            return invalid(format!(
                                                "Double-spend detected: UTXO {}:{} \
                                                 already spent in pending transaction",
                                                hex::encode(input.prev_txid),
                                                input.prev_vout
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Append transaction to pending file (JSONL format - one JSON per line)
        let tx_id = hex::encode(signed_tx.txid());
        // Serialize transaction as JSON string (prevents u128 overflow when embedded)
        let tx_json = serde_json::to_string(&signed_tx)
            .map_err(|e| Error::Invalid(format!("failed to serialize tx json: {e}")))?;
        let pending_entry = serde_json::json!({
            "txid": tx_id,
            "tx": tx_json,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&pending_path)
            .map_err(|e| Error::Invalid(format!("failed to open pending tx file: {e}")))?;

        writeln!(file, "{}", pending_entry)
            .map_err(|e| Error::Invalid(format!("failed to write pending tx: {e}")))?;

        println!("Transaction saved to pending pool!");
        println!("🔗 Transaction ID: {}", tx_id);
        println!("📁 Pending file: {}", pending_path.display());
        println!();
        println!("⚠️  Next steps:");
        println!("1. Mine a block to include this transaction:");
        println!("   cargo run --release -- bin/miner mine --config config/devnet.toml");
        println!("2. The miner will automatically include pending transactions");

        Ok(())
    }

    #[cfg(not(feature = "rocksdb-backend"))]
    {
        println!();
        println!("Note: Transaction sending requires 'rocksdb-backend' feature");
        println!("Missing components:");
        println!(" - UTXO lookup from blockchain");
        println!(" - Transaction broadcast to network");
        println!();

        invalid("rocksdb-backend feature required for sending transactions")
    }
}

/// Generate wallet from BIP39 mnemonic
pub fn wallet_gen_mnemonic(
    word_count: usize,
    output_path: Option<&str>,
    password: Option<&str>,
    show_mnemonic: bool,
) -> Result<()> {
    use crate::mnemonic::MnemonicHelper;
    use std::path::Path;

    // Generate mnemonic
    let helper = MnemonicHelper::generate_with_word_count(word_count)?;
    let mnemonic_phrase = helper.phrase();

    // Show mnemonic to user
    if show_mnemonic {
        eprintln!("\nSECURITY WARNING: Mnemonic phrase will be displayed!");
        eprintln!("  - Do NOT log terminal output");
        eprintln!("  - Do NOT screenshot this");
        eprintln!("  - Ensure nobody is watching your screen\n");

        println!("\n🔑 Your BIP39 Mnemonic Phrase:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("{}", mnemonic_phrase);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("\nCRITICAL SECURITY:");
        println!("  - Write down these words in order on paper");
        println!("  - Store them in a safe place (NOT digitally)");
        println!("  - Never share them with anyone");
        println!("  - Never enter them on websites or apps");
        println!("  - You need these words to recover your wallet");
        println!();
    } else {
        println!("Mnemonic generated (hidden for security)");
        println!("  Use --show-mnemonic flag to display (NOT recommended in production)");
    }

    // Derive keypair
    let keypair = helper.to_keypair()?;

    // Get encryption password FIRST (needed for secret key encryption)
    let password_value = match password {
        Some(p) => p.to_string(),
        None => {
            println!("🔒 Enter password to encrypt keystore:");
            crate::cli::read_password_from_stdin()?
        }
    };

    if password_value.is_empty() {
        return invalid("Password cannot be empty");
    }

    // Serialize keypair with encrypted secret key
    let serializable = keypair.to_serializable(&password_value);
    let json = serde_json::to_string_pretty(&serializable)?;

    // Encrypt and save keystore
    let keystore_file = keystore::encrypt_keypair(&json, &password_value, &serializable.address)
        .map_err(|e| Error::Invalid(format!("keystore encrypt failed: {e}")))?;
    let output_file = output_path.unwrap_or("wallet.keystore");
    keystore::save_keystore(&keystore_file, Path::new(output_file))
        .map_err(|e| Error::Invalid(format!("keystore save failed: {e}")))?;

    println!("\nWallet created successfully!");
    println!("📄 Keystore saved to: {}", output_file);
    println!("Address: {}", serializable.address);
    println!("\nTo recover this wallet later, use:");
    println!("  bitquan-node wallet-from-mnemonic");
    Ok(())
}

/// Recover wallet from BIP39 mnemonic
pub fn wallet_from_mnemonic(
    mnemonic: Option<&str>,
    passphrase: Option<&str>,
    output_path: Option<&str>,
    password: Option<&str>,
) -> Result<()> {
    use crate::mnemonic::MnemonicHelper;
    use std::path::Path;

    // Get mnemonic phrase
    let mnemonic_phrase = match mnemonic {
        Some(m) => m.to_string(),
        None => {
            println!("Enter your BIP39 mnemonic phrase:");
            println!("(12 or 24 words separated by spaces)");
            let mut phrase = String::new();
            std::io::stdin().read_line(&mut phrase)?;
            phrase.trim().to_string()
        }
    };

    if mnemonic_phrase.is_empty() {
        return invalid("Mnemonic phrase cannot be empty");
    }

    // Validate and parse mnemonic
    let helper = MnemonicHelper::from_phrase(&mnemonic_phrase, passphrase)?;

    println!("Mnemonic validated successfully!");

    // Derive keypair
    let keypair = helper.to_keypair()?;

    // Get encryption password FIRST (needed for secret key encryption)
    let password_value = match password {
        Some(p) => p.to_string(),
        None => {
            println!("🔒 Enter password to encrypt keystore:");
            crate::cli::read_password_from_stdin()?
        }
    };

    if password_value.is_empty() {
        return invalid("Password cannot be empty");
    }

    // Serialize keypair with encrypted secret key
    let serializable = keypair.to_serializable(&password_value);
    let json = serde_json::to_string_pretty(&serializable)?;

    // Encrypt and save keystore
    let keystore_file = keystore::encrypt_keypair(&json, &password_value, &serializable.address)
        .map_err(|e| Error::Invalid(format!("keystore encrypt failed: {e}")))?;
    let output_file = output_path.unwrap_or("wallet-recovered.keystore");
    keystore::save_keystore(&keystore_file, Path::new(output_file))
        .map_err(|e| Error::Invalid(format!("keystore save failed: {e}")))?;

    println!("\nWallet recovered successfully!");
    println!("📄 Keystore saved to: {}", output_file);
    println!("Address: {}", serializable.address);

    Ok(())
}

/// Generate multi-signature wallet address
pub fn wallet_gen_multisig(
    threshold: usize,
    keystores: &[String],
    labels: &[String],
    output: &str,
) -> Result<()> {
    use ::wallet::multisig::MultisigConfig;
    use std::path::Path;

    if keystores.is_empty() {
        return invalid("At least 2 keystore files required for multisig");
    }

    println!(
        "\nCreating {}-of-{} Multi-signature Wallet",
        threshold,
        keystores.len()
    );
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Load public keys from keystores
    let mut public_keys = Vec::new();
    for (i, keystore_path) in keystores.iter().enumerate() {
        println!(
            "Loading keystore {} of {}: {}",
            i + 1,
            keystores.len(),
            keystore_path
        );

        let keystore_file = keystore::load_keystore(Path::new(keystore_path))
            .map_err(|e| Error::Invalid(format!("keystore load failed: {e}")))?;

        // Prompt for password
        println!("🔑 Enter password for {}:", keystore_path);
        let password = crate::cli::read_password_from_stdin()?;
        let json = keystore::decrypt_keypair(&keystore_file, &password)
            .map_err(|e| Error::Invalid(format!("keystore decrypt failed: {e}")))?;
        let serializable: SerializableKeypair = serde_json::from_str(&json)?;

        public_keys.push(serializable.public_key.clone());
    }

    // Add labels if provided
    let label = if !labels.is_empty() {
        Some(labels.join(", "))
    } else {
        Some(format!("{}-of-{} Multisig", threshold, keystores.len()))
    };

    // Create multisig config
    let config = MultisigConfig::new(threshold as u8, public_keys, label)
        .map_err(|e| Error::Invalid(format!("multisig config error: {e}")))?;

    // Generate address
    let address = config.address();

    // Save config
    let config_json = serde_json::to_string_pretty(&config)?;
    std::fs::write(output, config_json)?;

    println!("\nMulti-signature wallet created successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 Configuration:");
    println!("  Type: {}", config.config_type());
    println!("  Address: {}", address);
    println!("  Config saved to: {}", output);
    println!("\n👥 Signers: {}", config.total_signers);
    println!("\nNext steps:");
    println!("  1. Share this address with all signers");
    println!("  2. Distribute the config file: {}", output);
    println!("  3. Use 'tx-sign-partial' to sign transactions");

    Ok(())
}

/// Show multi-signature wallet information
pub fn multisig_info(config_path: &str) -> Result<()> {
    use ::wallet::multisig::MultisigConfig;

    // Load config
    let config_json = std::fs::read_to_string(config_path)?;
    let config: MultisigConfig = serde_json::from_str(&config_json)?;

    // Generate address
    let address = config.address();

    println!("\n📋 Multi-signature Wallet Information");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Address:  {}", address);
    println!("Type:   {}", config.config_type());
    println!(
        "Created:  {}",
        chrono::DateTime::from_timestamp(config.created_at as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    );
    if let Some(label) = &config.label {
        println!("Label:   {}", label);
    }
    println!("\n👥 Signers: {}", config.total_signers);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    for (i, pk) in config.public_keys.iter().enumerate() {
        let pk_preview = if pk.len() > 16 {
            format!("{}...{}", &pk[..8], &pk[pk.len() - 8..])
        } else {
            pk.clone()
        };
        println!("  {}. {}", i + 1, pk_preview);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// Sign transaction with partial signature
pub fn tx_sign_partial(
    _tx_path: &str,
    _keystore_path: &str,
    _multisig_config_path: &str,
    _output: &str,
    _password: Option<&str>,
) -> Result<()> {
    // Transaction signing implementation pending final format
    println!("📝 Transaction signing uses the multisig module:");
    println!("  - Use MultiSigManager for multi-signature transactions");
    println!("  - Transaction format is stable and ready for use");
    println!("\nExample: See multisig module documentation for usage");
    println!("  use wallet::multisig::{{MultisigWallet, PendingMultisigTx}};");

    invalid("Feature coming soon: partial transaction signing")
}

/// Combine partial signatures
pub fn tx_combine_signatures(
    _tx_path: &str,
    _signature_paths: &[String],
    _multisig_config_path: &str,
    _output: &str,
) -> Result<()> {
    // Signature combination using multisig module
    println!("🔗 Signature combination uses the multisig module:");
    println!("  - MultiSigManager handles signature collection");
    println!("  - Transaction format supports partial signatures");
    println!("  - See multisig documentation for implementation");
    println!("\nFor now, use the multisig module directly in your code:");
    println!("  use wallet::multisig::{{MultisigWallet, FinalizedMultisigTx}};");

    invalid("Feature coming soon: signature combination")
}

/// Create encrypted backup of wallet keystore
pub fn wallet_backup(
    keystore_path: &str,
    output_path: &str,
    backup_password: Option<&str>,
    network: &str,
    label: Option<String>,
) -> Result<()> {
    use ::wallet::backup::{Network, WalletBackup};
    use std::fs;

    // Read keystore file
    let keystore_data = fs::read(keystore_path)
        .map_err(|e| Error::Invalid(format!("Failed to read keystore {}: {e}", keystore_path)))?;

    // Get backup password
    let backup_pw = match backup_password {
        Some(pw) => pw.to_string(),
        None => {
            print!("Enter backup password: ");
            std::io::stdout().flush()?;
            rpassword::read_password()?
        }
    };

    // Parse network
    let net = match network.to_lowercase().as_str() {
        "mainnet" => Network::Mainnet,
        "testnet" => Network::Testnet,
        "devnet" => Network::Devnet,
        _ => {
            return invalid(format!(
                "Invalid network: {} (use mainnet, testnet, or devnet)",
                network
            ))
        }
    };

    // Create backup
    println!("Creating encrypted backup...");
    let backup = WalletBackup::create(&keystore_data, &backup_pw, net, label)
        .map_err(|e| Error::Invalid(format!("Backup creation failed: {}", e)))?;

    // Save to file
    backup
        .save(output_path)
        .map_err(|e| Error::Invalid(format!("Failed to save backup {}: {e}", output_path)))?;

    println!("Backup created successfully: {}", output_path);
    println!("  Version: {}", backup.version);
    println!("  Network: {:?}", backup.network);
    println!("  Timestamp: {}", backup.timestamp);
    if let Some(lbl) = backup.label {
        println!("  Label: {}", lbl);
    }
    println!("\nIMPORTANT: Store backup password separately and securely!");

    Ok(())
}

/// Restore wallet from encrypted backup
pub fn wallet_restore(
    backup_path: &str,
    output_path: &str,
    backup_password: Option<&str>,
) -> Result<()> {
    use ::wallet::backup::WalletBackup;
    use std::fs;

    // Load backup
    println!("Loading backup file...");
    let backup = WalletBackup::load(backup_path)
        .map_err(|e| Error::Invalid(format!("Failed to load backup {}: {e}", backup_path)))?;

    println!("Backup information:");
    println!("  Version: {}", backup.version);
    println!("  Network: {:?}", backup.network);
    println!("  Timestamp: {}", backup.timestamp);
    if let Some(ref label) = backup.label {
        println!("  Label: {}", label);
    }

    // Get backup password
    let backup_pw = match backup_password {
        Some(pw) => pw.to_string(),
        None => {
            print!("\nEnter backup password: ");
            std::io::stdout().flush()?;
            rpassword::read_password()?
        }
    };

    // Restore
    println!("Decrypting and restoring wallet...");
    let keystore_data = backup
        .restore(&backup_pw)
        .map_err(|e| Error::Invalid(format!("Restore failed: {}", e)))?;

    // Check if output exists
    if std::path::Path::new(output_path).exists() {
        print!("Output file exists. Overwrite? (y/N): ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            return invalid("Restore cancelled");
        }
    }

    // Write keystore
    fs::write(output_path, keystore_data)
        .map_err(|e| Error::Invalid(format!("Failed to write keystore {}: {e}", output_path)))?;

    println!("Wallet restored successfully: {}", output_path);
    println!("\nRemember to use your original wallet password to access this keystore.");

    Ok(())
}
