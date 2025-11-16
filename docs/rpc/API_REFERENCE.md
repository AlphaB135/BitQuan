# RPC API Reference

**Last Updated: 2025-01-07**

Complete JSON-RPC 2.0 API reference for BitQuan nodes.

## Overview

BitQuan provides a JSON-RPC 2.0 API over HTTPS with JWT authentication. All endpoints require TLS and valid JWT tokens.

## Authentication

See [JWT Quick Start](../guides/JWT_QUICK_START.md) for authentication setup.

```bash
# Get JWT token
curl -X POST https://node:8332/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"secret"}'

# Use token
curl -X POST https://node:8332/rpc \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}'
```

## Blockchain RPCs

### getblockcount

Get current block height.

**Request**:
```json
{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}
```

**Response**:
```json
{"jsonrpc":"2.0","result":12345,"id":1}
```

### getblockhash

Get block hash at height.

**Request**:
```json
{"jsonrpc":"2.0","method":"getblockhash","params":[12345],"id":1}
```

**Response**:
```json
{
  "jsonrpc":"2.0",
  "result":"0x1234567890abcdef...",
  "id":1
}
```

### getblock

Get block data by hash.

**Request**:
```json
{
  "jsonrpc":"2.0",
  "method":"getblock",
  "params":["0x1234...", true],
  "id":1
}
```

**Response**: Full block object with transactions.

### getblockheader

Get block header only.

## Transaction RPCs

### getrawtransaction

Get raw transaction by txid.

### sendrawtransaction

Broadcast signed transaction.

### gettxout

Get unspent transaction output.

## Network RPCs

### getpeerinfo

Get connected peer information.

### getnetworkinfo

Get network status and configuration.

## Mining RPCs

### getblocktemplate

Get block template for mining.

### submitblock

Submit mined block.

## Wallet RPCs

### getbalance

Get wallet balance.

### sendtoaddress

Send to address (simplified).

### listunspent

List unspent outputs.

## Utility RPCs

### validateaddress

Validate and decode address.

### estimatefee

Estimate fee for transaction.

## Error Codes

Standard JSON-RPC 2.0 errors:

- `-32700` - Parse error
- `-32600` - Invalid request
- `-32601` - Method not found
- `-32602` - Invalid params
- `-32603` - Internal error

BitQuan-specific:

- `-1` - Miscellaneous error
- `-5` - Invalid address
- `-8` - Block not found
- `-25` - Transaction rejected
- `-26` - Transaction already in blockchain

## Rate Limiting

- Default: 100 requests/minute per IP
- Authenticated: 1000 requests/minute
- Headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`

## See Also

- [CLI Reference](../cli/bitquan-node.md) - Command-line tools
- [JWT Setup](../guides/JWT_QUICK_START.md) - Authentication
- [Operations](../ops/) - Production deployment

---

*Updated on: 2025-01-07*

**Note**: This is a stub. Full API documentation is being developed. See source code for complete method list.
