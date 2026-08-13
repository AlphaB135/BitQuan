//! BitQuan reference node entrypoint.
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(missing_docs)]

// Library modules - public APIs may not all be used in main binary
// Note: Some modules have their own #![allow(dead_code)] at module level
mod address;
#[allow(dead_code)]
mod block_submit;
#[allow(dead_code)]
mod chainstate;
#[allow(dead_code)]
mod keystore;
#[allow(dead_code)]
mod metrics;
#[allow(dead_code)]
mod miner;
mod mnemonic;
#[allow(dead_code)]
mod pool_template;
#[allow(dead_code)]
mod reward_engine;
#[cfg(feature = "rocksdb-backend")]
mod rpc;
mod stratum_server;
#[allow(dead_code)]
mod sync_task;
mod tx_builder;
mod utxo;
#[allow(dead_code)]
mod vardiff;
#[allow(dead_code)]
mod wallet;
#[allow(dead_code)]
mod worker;
mod ws_dashboard;

// Command modules (must be declared before cli which uses it)
pub mod commands;

// CLI utilities
mod cli;
mod cli_commands;

// Import CLI helper functions
use cli::{
    ensure_pow_allowed, extract_config_array, extract_config_value, load_network_from_config,
    parse_network_id,
};
use cli_commands::Commands;

// Import moved command functions
// Note: MiningOptions and PowMode are defined locally in main.rs
use commands::mining::{
    check_block, mine_genesis, mine_once, parse_hybrid_weights, rng_demo, run_stratum_server,
};
use commands::node::{
    build_tx, check_balance, genesis_verify, script_from_address, verify_database,
};
use commands::p2p::RpcServerOptions;
use commands::rpc::{
    generate_self_signed_cert_cli, hash_password_cli, jwt_user_add, jwt_user_list, jwt_user_remove,
};
use commands::wallet::{
    multisig_info, tx_combine_signatures, tx_sign_partial, wallet_address, wallet_backup,
    wallet_from_mnemonic, wallet_gen, wallet_gen_mnemonic, wallet_gen_multisig, wallet_restore,
    wallet_send, wallet_sign, wallet_verify,
};
// Import for address validation (moved to commands/node)
use bitquan_types::error::{Error, Result};
use bitquan_types::NetworkId;
use clap::Parser;
use commands::node::address_validate;
use log::error;

/// Proof-of-Work algorithm mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowMode {
    /// Standard SHA-256d hashcash (Bitcoin-style)
    Hashcash,
    /// Mock mode for testing (debug builds only)
    Mock,
    /// RandomX algorithm (memory-hard)
    #[cfg(feature = "randomx")]
    RandomX,
    /// Hybrid mode combining multiple algorithms
    Hybrid,
    /// Ethash algorithm (Ethereum-style)
    Ethash,
}

impl PowMode {
    fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "hashcash" | "sha256d" | "real" => Ok(PowMode::Hashcash),
            "mock" | "dev-fast-pow" => {
                #[cfg(feature = "testing")]
                return Ok(PowMode::Mock);

                #[cfg(not(feature = "testing"))]
                return crate::cli::invalid(
                    "Mock PoW is only available with '--features testing'. \
                     Use 'hashcash' for real proof-of-work mining.",
                );
            }
            #[cfg(feature = "randomx")]
            "randomx" => Ok(PowMode::RandomX),
            #[cfg(feature = "randomx")]
            "hybrid" => Ok(PowMode::Hybrid),
            #[cfg(not(feature = "randomx"))]
            "hybrid" => Ok(PowMode::Hybrid),
            "ethash" => Ok(PowMode::Ethash),
            other => crate::cli::invalid(format!("unknown pow engine '{}'", other)),
        }
    }
}

#[derive(Parser)]
#[command(
    name = "bitquan-node",
    version,
    about = "BitQuan reference node (prototype)",
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Custom parser for u128 values in CLI arguments
/// Clap doesn't have built-in u128 support, so we use string parsing
///
/// Install panic hook for better crash reporting
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        error!("\n=== PANIC ===");
        if let Some(location) = panic_info.location() {
            error!(
                "Location: {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            error!("Message: {}", s);
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            error!("Message: {}", s);
        }
        error!("==============\n");
    }));
}

#[allow(clippy::too_many_arguments)]
#[tokio::main]
async fn main() -> Result<()> {
    // Install panic hook for better crash reporting
    install_panic_hook();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            config,
            rpc_bind,
            p2p_bind,
        } => {
            let network = load_network_from_config(&config)?;
            run_node(&config, rpc_bind.as_deref(), p2p_bind.as_deref(), network).await
        }
        Commands::MineGenesis { max_tries, output } => mine_genesis(max_tries, &output),
        Commands::CheckBlock { path } => check_block(&path),
        Commands::Rng { label, length } => rng_demo(&label, length),
        Commands::MineOnce {
            max_tries,
            payout_script_hex,
            bits,
            network,
            pow,
        } => {
            let network_id = parse_network_id(&network)?;
            let pow_mode = PowMode::parse(&pow)?;
            ensure_pow_allowed(pow_mode, network_id)?;
            mine_once(max_tries, &payout_script_hex, bits, network_id, pow_mode)
        }
        Commands::Mine {
            datadir,
            payout_script_hex,
            bits,
            max_nonce,
            network,
            pow,
            threads,
            limit_blocks,
            hybrid_weights,
            #[cfg(feature = "randomx")]
                randomx_mode: _randomx_mode,
            #[cfg(feature = "randomx")]
                randomx_seed: _randomx_seed,
            peers,
        } => {
            let network_id = parse_network_id(&network)?;
            let pow_mode = PowMode::parse(&pow)?;
            ensure_pow_allowed(pow_mode, network_id)?;

            let weights = if matches!(pow_mode, PowMode::Hybrid) {
                Some(parse_hybrid_weights(
                    hybrid_weights.as_deref().unwrap_or("sha256d:1,ethash:2"),
                )?)
            } else {
                None
            };

            let mining_handle = tokio::task::spawn_blocking(move || {
                commands::mining::mine_continuous(commands::mining::MiningOptions {
                    datadir,
                    payout_script_hex,
                    bits_override: bits,
                    max_nonce,
                    threads,
                    limit_blocks,
                    network: network_id,
                    pow_mode,
                    hybrid_weights: weights,
                    peers,
                })
            });
            mining_handle
                .await
                .map_err(|e| Error::Invalid(format!("mining task failed: {e}")))?
        }
        Commands::WalletGen {
            algo,
            network,
            output,
            password,
        } => wallet_gen(&algo, &network, output.as_deref(), password.as_deref()),
        Commands::WalletGenMnemonic {
            words,
            output,
            password,
            show_mnemonic,
        } => wallet_gen_mnemonic(words, output.as_deref(), password.as_deref(), show_mnemonic),
        Commands::WalletFromMnemonic {
            mnemonic,
            passphrase,
            output,
            password,
        } => wallet_from_mnemonic(
            mnemonic.as_deref(),
            passphrase.as_deref(),
            output.as_deref(),
            password.as_deref(),
        ),
        Commands::WalletBackup {
            keystore,
            output,
            backup_password,
            network,
            label,
        } => wallet_backup(
            &keystore,
            &output,
            backup_password.as_deref(),
            &network,
            label.clone(),
        ),
        Commands::WalletRestore {
            backup,
            output,
            backup_password,
        } => wallet_restore(&backup, &output, backup_password.as_deref()),
        Commands::WalletGenMultisig {
            threshold,
            keystores,
            labels,
            output,
        } => wallet_gen_multisig(threshold, &keystores, &labels, &output),
        Commands::MultisigInfo { config } => multisig_info(&config),
        Commands::TxSignPartial {
            tx,
            keystore,
            multisig_config,
            output,
            password,
        } => tx_sign_partial(
            &tx,
            &keystore,
            &multisig_config,
            &output,
            password.as_deref(),
        ),
        Commands::TxCombineSignatures {
            tx,
            signatures,
            multisig_config,
            output,
        } => tx_combine_signatures(&tx, &signatures, &multisig_config, &output),
        Commands::WalletAddress { keystore, password } => {
            wallet_address(&keystore, password.as_deref())
        }
        Commands::ScriptFromAddress { address } => script_from_address(&address),
        Commands::ValidateAddress { address } => address_validate(&address),
        Commands::WalletSign {
            keystore,
            message,
            password,
        } => wallet_sign(&keystore, &message, password.as_deref()),
        Commands::WalletVerify {
            pubkey,
            message,
            signature,
        } => wallet_verify(&pubkey, &message, &signature),
        Commands::WalletSend {
            keystore,
            to,
            amount,
            fee_rate,
            password,
            datadir,
        } => {
            wallet_send(
                &keystore,
                &to,
                amount,
                fee_rate,
                password.as_deref(),
                &datadir,
            )
            .await
        }
        Commands::BuildTx {
            prev_txid,
            prev_vout,
            value,
            to_script_hex,
        } => build_tx(&prev_txid, prev_vout, value, &to_script_hex),
        Commands::P2PDemo { addr } => commands::p2p::p2p_demo(&addr),
        Commands::P2PServer {
            listen,
            max_peers,
            datadir,
            network,
            #[cfg(feature = "rocksdb-backend")]
            rpc_listen,
            #[cfg(feature = "rocksdb-backend")]
            rpc_username,
            #[cfg(feature = "rocksdb-backend")]
            rpc_password,
            #[cfg(feature = "rocksdb-backend")]
            rpc_max_body,
            #[cfg(feature = "rocksdb-backend")]
            rpc_rl_burst,
            #[cfg(feature = "rocksdb-backend")]
            rpc_rl_refill_per_sec,
            #[cfg(feature = "rocksdb-backend")]
            rpc_conn_cooldown_ms,
            #[cfg(feature = "rocksdb-backend")]
            rpc_max_header,
            #[cfg(feature = "rocksdb-backend")]
            rpc_header_timeout_ms,
            #[cfg(feature = "rocksdb-backend")]
            rpc_trust_proxy,
            #[cfg(feature = "rocksdb-backend")]
            rpc_trusted_cidr,
            #[cfg(feature = "rocksdb-backend")]
            rpc_tls_cert,
            #[cfg(feature = "rocksdb-backend")]
            rpc_tls_key,
            #[cfg(feature = "rocksdb-backend")]
            rpc_allow_insecure,
            #[cfg(feature = "rocksdb-backend")]
            jwt_config,
            #[cfg(feature = "rocksdb-backend")]
            jwt_secret,
            connect,
        } => {
            #[cfg(feature = "rocksdb-backend")]
            {
                let network_id = parse_network_id(&network)?;
                commands::p2p::p2p_server(
                    &listen,
                    max_peers,
                    &datadir,
                    RpcServerOptions {
                        listen: rpc_listen.as_deref(),
                        username: rpc_username.as_deref(),
                        password: rpc_password.as_deref(),
                        max_body_bytes: rpc_max_body,
                        rl_burst: rpc_rl_burst,
                        rl_refill_per_sec: rpc_rl_refill_per_sec,
                        conn_cooldown_ms: rpc_conn_cooldown_ms,
                        max_header_bytes: rpc_max_header,
                        header_timeout_ms: rpc_header_timeout_ms,
                        trust_proxy: rpc_trust_proxy,
                        trusted_cidr: rpc_trusted_cidr,
                        tls_cert: rpc_tls_cert.as_deref(),
                        tls_key: rpc_tls_key.as_deref(),
                        allow_insecure: rpc_allow_insecure,
                        jwt_config_path: jwt_config.as_deref(),
                        jwt_secret: jwt_secret.as_deref(),
                    },
                    network_id,
                    Some(connect), // bootstrap_peers: connect to specified peers
                )
                .await
            }
            #[cfg(not(feature = "rocksdb-backend"))]
            {
                let network_id = parse_network_id(&network)?;
                let _ = (&listen, max_peers, &datadir);
                commands::p2p::p2p_server(
                    &listen,
                    max_peers,
                    &datadir,
                    RpcServerOptions::default(),
                    network_id,
                    Some(connect), // bootstrap_peers: connect to specified peers
                )
                .await
            }
        }
        Commands::P2PConnect {
            peer,
            height,
            network,
        } => {
            let network_id = parse_network_id(&network)?;
            commands::p2p::p2p_connect(&peer, height, network_id).await
        }
        Commands::StratumServer {
            stratum_bind,
            stratum_allow,
            stratum_diff,
            network,
        } => {
            let network_id = parse_network_id(&network)?;
            let allow_list = stratum_allow
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            run_stratum_server(stratum_bind, allow_list, stratum_diff, network_id)
        }
        Commands::Balance {
            datadir,
            script_hex,
            address,
        } => check_balance(&datadir, script_hex.as_deref(), address.as_deref()),
        #[cfg(feature = "rocksdb-backend")]
        Commands::GenerateCert { output } => generate_self_signed_cert_cli(&output),
        Commands::HashPassword { password } => hash_password_cli(password.as_deref()),
        Commands::JwtUserAdd {
            config,
            username,
            role,
            password,
        } => jwt_user_add(&config, &username, &role, password.as_deref()),
        Commands::JwtUserRemove { config, username } => jwt_user_remove(&config, &username),
        Commands::JwtUserList { config } => jwt_user_list(&config),
        #[cfg(feature = "rocksdb-backend")]
        Commands::VerifyDb {
            path,
            backup,
            backup_path,
            rebuild,
        } => verify_database(&path, backup, backup_path.as_deref(), rebuild),
        Commands::GenesisVerify {
            genesis_file,
            network,
        } => genesis_verify(&genesis_file, &network),
    }
}

async fn run_node(
    config_path: &str,
    rpc_bind: Option<&str>,
    p2p_bind: Option<&str>,
    network: NetworkId,
) -> Result<()> {
    // Parse config file for settings
    let config_content = std::fs::read_to_string(config_path).unwrap_or_default();

    // Extract db_path from config (default to ./data/chainstate)
    let datadir = extract_config_value(&config_content, "db_path")
        .unwrap_or_else(|| "./data/chainstate".to_string());

    // Extract p2p_port from config for deriving metrics port
    let config_p2p_port: u16 = extract_config_value(&config_content, "p2p_port")
        .and_then(|s| s.parse().ok())
        .unwrap_or(18444);

    // Use CLI override or config value for P2P address
    let p2p_addr = p2p_bind
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("0.0.0.0:{}", config_p2p_port));

    // Extract rpc_bind from config file as fallback if not provided via CLI
    let rpc_addr = rpc_bind
        .map(|s| s.to_string())
        .or_else(|| extract_config_value(&config_content, "rpc_bind"));

    log::info!(
    "Starting BitQuan node with configuration: {config_path}\nP2P listening on {p2p_addr}\nData directory: {datadir}"
  );

    // Create data directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&datadir) {
        error!(
            "Warning: Failed to create data directory {}: {}",
            datadir, e
        );
    }

    // Metrics server will be started by commands::p2p::p2p_server() with proper port derivation

    // Extract bootstrap_nodes from config
    let bootstrap_peers = extract_config_array(&config_content, "bootstrap_nodes");
    let bootstrap_peers_opt = if bootstrap_peers.is_empty() {
        None
    } else {
        Some(bootstrap_peers)
    };

    let rpc_user = extract_config_value(&config_content, "rpc_user")
        .or_else(|| extract_config_value(&config_content, "rpc_username"));
    let rpc_pass = extract_config_value(&config_content, "rpc_password");
    let allow_insecure = extract_config_value(&config_content, "allow_insecure")
        .map(|s| s.to_lowercase() == "true")
        .unwrap_or(false);

    commands::p2p::p2p_server(
        &p2p_addr,
        50, // max_peers
        &datadir,
        RpcServerOptions {
            listen: rpc_addr.as_deref(),
            username: rpc_user.as_deref(),
            password: rpc_pass.as_deref(),
            #[cfg(feature = "rocksdb-backend")]
            jwt_config_path: None,
            #[cfg(feature = "rocksdb-backend")]
            jwt_secret: None,
            #[cfg(feature = "rocksdb-backend")]
            max_body_bytes: 1_000_000,
            #[cfg(feature = "rocksdb-backend")]
            rl_burst: 10,
            #[cfg(feature = "rocksdb-backend")]
            rl_refill_per_sec: 1,
            #[cfg(feature = "rocksdb-backend")]
            conn_cooldown_ms: 1000,
            #[cfg(feature = "rocksdb-backend")]
            max_header_bytes: 8192,
            #[cfg(feature = "rocksdb-backend")]
            header_timeout_ms: 5000,
            #[cfg(feature = "rocksdb-backend")]
            trust_proxy: false,
            #[cfg(feature = "rocksdb-backend")]
            trusted_cidr: vec![],
            #[cfg(feature = "rocksdb-backend")]
            tls_cert: None,
            #[cfg(feature = "rocksdb-backend")]
            tls_key: None,
            #[cfg(feature = "rocksdb-backend")]
            allow_insecure,
        },
        network,
        bootstrap_peers_opt,
    )
    .await
}
