# BitQuan Testnet Faucet 🚰

A standalone microservice for distributing testnet coins to developers.

## Architecture

```
┌─────────────┐     HTTP     ┌──────────────┐     RPC      ┌──────────────┐
│  Web Browser│ ───────────▶ │  Faucet Service│ ──────────▶ │  BitQuan Node │
└─────────────┘              └──────────────┘              └──────────────┘
     (Port 3000)                   (warp)                   (Port 8332)
```

**Separation of Concerns**: The faucet is a completely separate service from the node. It talks to the node via JSON-RPC over HTTP.

## Features

- ✅ **Web Interface**: Clean HTML form at `http://localhost:3000`
- ✅ **Rate Limiting**: 1 request per minute per IP
- ✅ **Address Validation**: Only accepts valid `bq1...` addresses
- ✅ **RPC Integration**: Uses `sendtoaddress` JSON-RPC method
- ✅ **CORS Enabled**: Cross-origin requests allowed

## Quick Start

### 1. Start BitQuan Node

First, start your BitQuan node with RPC enabled:

```bash
# Terminal 1: Start node
./target/release/bitquan-node --rpc --rpc-user=myuser --rpc-password=mypass
```

### 2. Start Faucet Service

In a new terminal, start the faucet:

```bash
# Terminal 2: Start faucet
export BITQUAN_RPC_URL=http://127.0.0.1:8332
export BITQUAN_RPC_USER=myuser
export BITQUAN_RPC_PASS=mypass

cargo run -p faucet
```

The faucet will start on `http://localhost:3000`

### 3. Get Testnet Coins

Open your browser: `http://localhost:3000`

Enter your BitQuan address (`bq1...`) and click "💧 Get Testnet Coins"

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `BITQUAN_RPC_URL` | `http://127.0.0.1:8332` | Node RPC endpoint |
| `BITQUAN_RPC_USER` | `user` | RPC username |
| `BITQUAN_RPC_PASS` | `pass` | RPC password |
| `FAUCET_PORT` | `3000` | Faucet web server port |
| `FAUCET_DRIP_AMOUNT` | `10.0` | Coins to send per request |

## API Usage

### Manual API Call

```bash
curl -X POST http://localhost:3000/api/drip \
  -H "Content-Type: application/json" \
  -d '{"address":"bq1...youraddress..."}'

# Response:
# {"txid":"<transaction_hash>"}
```

### Error Responses

```json
{"error":"Rate limit exceeded. Please wait 1 minute between requests."}
{"error":"Invalid BitQuan address format. Must start with 'bq1'"}
{"error":"Failed to send coins: RPC error: ..."}
```

## Building

```bash
# Development build
cargo build -p faucet

# Release build
cargo build -p faucet --release

# Run release binary
./target/release/faucet
```

## Rate Limiting

The faucet uses in-memory rate limiting:
- **1 request per minute** per IP address
- Old entries are automatically cleaned up
- Uses `DashMap` for thread-safe concurrent access

## Security Notes

⚠️ **Testnet Only**: This faucet is for testnet use only. Never expose this to mainnet!

**Production Considerations**:
- Use a separate RPC user with limited permissions
- Set `FAUCET_DRIP_AMOUNT` to a reasonable testnet value
- Consider adding CAPTCHA to prevent automated abuse
- Add database-backed rate limiting for persistence across restarts

## Troubleshooting

### "Failed to send coins: RPC error"

**Check**:
- Node is running: `ps aux | grep bitquan-node`
- RPC is accessible: `curl http://127.0.0.1:8332`
- RPC credentials are correct

### "Rate limit exceeded"

Wait 60 seconds between requests from the same IP.

## Architecture Rationale

**Why a separate service?**

1. **Separation of Concerns**: Node handles blockchain, faucet handles distribution
2. **Isolation**: Faucet issues don't affect node stability
3. **Flexibility**: Easy to deploy separately, scale independently
4. **Security**: Limited RPC permissions for faucet-specific user

## License

Apache License 2.0 - See main project LICENSE
