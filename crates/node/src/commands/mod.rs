//! Command modules for BitQuan CLI
//!
//! This module organizes all CLI commands into logical categories:
//! - wallet: Wallet operations (create, restore, send, balance, etc.)
//! - mining: Mining operations (mine genesis, continuous, stratum server)
//! - p2p: P2P network operations (server, connect, demo)
//! - rpc: RPC/JWT operations (server, user management, certificates)
//! - node: Node utilities (check balance, verify database, etc.)
//! - pruning: Blockchain data pruning (reduce disk usage)

pub mod mining;
pub mod node;
pub mod p2p;
pub mod pruning;
pub mod rpc;
pub mod wallet;
