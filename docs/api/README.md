# API Documentation

BitQuan provides comprehensive APIs for interacting with the blockchain.

## 🔌 Available APIs

### JSON-RPC API
- [RPC Overview](rpc/README.md) - Introduction to the JSON-RPC API
- [Authentication](rpc/authentication.md) - JWT-based authentication
- [Blockchain Methods](rpc/blockchain.md) - Block and transaction queries
- [Wallet Methods](rpc/wallet.md) - Wallet management operations
- [Mining Methods](rpc/mining.md) - Mining-related operations

### SDK Documentation
- [Rust SDK](sdk/rust.md) - Official Rust SDK
- [TypeScript SDK](sdk/typescript.md) - Official TypeScript SDK

### CLI Reference
- [CLI Commands](cli/README.md) - Command-line interface reference

## 🚀 Quick Start

### Using the JSON-RPC API

```bash
# Get block count
curl -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}' \
  http://localhost:8332

# Get block hash
curl -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockhash","params":[1000],"id":1}' \
  http://localhost:8332
```

### Using the Rust SDK

```rust
use bitquan_sdk::{Client, RpcClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RpcClient::new("http://localhost:8332")?;
    let block_count = client.get_block_count().await?;
    println!("Block count: {}", block_count);
    Ok(())
}
```

## 📚 Learn More

- [Architecture Overview](../architecture/overview.md) - Understand the system design
- [Security Best Practices](../security/best-practices.md) - Secure your API usage
- [Development Guide](../development/setup.md) - Set up development environment

## 🔐 Authentication

All API endpoints require authentication using JWT tokens. See the [Authentication Guide](rpc/authentication.md) for details.

## 📖 API Reference

For complete API documentation, see the specific sections:

- [Blockchain API](rpc/blockchain.md) - Block and transaction operations
- [Wallet API](rpc/wallet.md) - Wallet management
- [Mining API](rpc/mining.md) - Mining operations
- [Network API](rpc/network.md) - Network information

---

*Last updated: 2025-11-21*