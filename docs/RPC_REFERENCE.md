# BitQuan RPC Reference

**Last Updated**: 2026-03-27
**Default Port**: 18443 (mainnet), 19443 (testnet), 18332 (devnet)

## Authentication

All RPC endpoints require JWT authentication.

```bash
# Login
curl -X POST http://localhost:18332/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"your_password"}'

# Response
{"token":"eyJhbGciOiJIUzI1NiJ9...","refresh_token":"..."}
```

Include the token in all subsequent requests:
```
Authorization: Bearer <token>
```

---

## Blockchain

### getblockcount

Get current block height.

```bash
curl -X POST http://localhost:18332 \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}'
```

```json
{"jsonrpc":"2.0","result":1234,"id":1}
```

### getblockchaininfo

Get blockchain status and network info.

```json
{"jsonrpc":"2.0","method":"getblockchaininfo","params":[],"id":1}
```

```json
{
  "jsonrpc": "2.0",
  "result": {
    "chain": "devnet",
    "blocks": 1234,
    "headers": 1234,
    "difficulty": 1.0,
    "best_block_hash": "0000...",
    "verification_progress": 1.0
  },
  "id": 1
}
```

### getbestblockhash

Get hash of the tip block.

```json
{"jsonrpc":"2.0","method":"getbestblockhash","params":[],"id":1}
```

### getblockhash

Get block hash by height.

```json
{"jsonrpc":"2.0","method":"getblockhash","params":[100],"id":1}
```

---

## Mining

### getmininginfo

Get mining status and difficulty.

```json
{"jsonrpc":"2.0","method":"getmininginfo","params":[],"id":1}
```

```json
{
  "result": {
    "blocks": 1234,
    "difficulty": 1.0,
    "network_hashrate": 0,
    "pooled_tx": 5
  }
}
```

### getblocktemplate

Get block template for mining.

```json
{"jsonrpc":"2.0","method":"getblocktemplate","params":[],"id":1}
```

### getwork

Get mining work data.

```json
{"jsonrpc":"2.0","method":"getwork","params":[],"id":1}
```

### submitblock

Submit a mined block.

```json
{"jsonrpc":"2.0","method":"submitblock","params":["<block_hex>"],"id":1}
```

### submitwork

Submit mining solution.

```json
{"jsonrpc":"2.0","method":"submitwork","params":["<work_data>"],"id":1}
```

---

## Transactions

### gettransaction

Get transaction details by TXID.

```json
{"jsonrpc":"2.0","method":"gettransaction","params":["<txid>"],"id":1}
```

```json
{
  "result": {
    "txid": "abc123...",
    "block_hash": "0000...",
    "block_height": 100,
    "inputs": [...],
    "outputs": [...],
    "amount": 5000000000,
    "confirmations": 1134
  }
}
```

### submittransaction

Broadcast a signed transaction.

```json
{"jsonrpc":"2.0","method":"submittransaction","params":["<tx_hex>"],"id":1}
```

### sendtoaddress

Send funds to an address (convenience method).

```json
{"jsonrpc":"2.0","method":"sendtoaddress","params":["<address>", 1000],"id":1}
```

### generate

Mine blocks immediately (devnet/regtest only).

```json
{"jsonrpc":"2.0","method":"generate","params":[10],"id":1}
```

### generatetoaddress

Mine blocks to a specific address.

```json
{"jsonrpc":"2.0","method":"generatetoaddress","params":[10, "<address>"],"id":1}
```

---

## Network

### getnetworkstatus

Get P2P network status.

```json
{"jsonrpc":"2.0","method":"getnetworkstatus","params":[],"id":1}
```

```json
{
  "result": {
    "connected_peers": 3,
    "known_peers": 10,
    "network_id": "devnet",
    "p2p_port": 18444
  }
}
```

### sync

Trigger or check sync status.

```json
{"jsonrpc":"2.0","method":"sync","params":[],"id":1}
```

---

## Mining Pool

### getpoolstats

Get mining pool statistics.

```json
{"jsonrpc":"2.0","method":"getpoolstats","params":[],"id":1}
```

### getminerstats

Get stats for a specific miner.

```json
{"jsonrpc":"2.0","method":"getminerstats","params":["<miner_id>"],"id":1}
```

### createpayout

Create a pool payout transaction.

```json
{"jsonrpc":"2.0","method":"createpayout","params":[{"address":"<addr>","amount":1000}],"id":1}
```

---

## Error Codes

| Code | Meaning |
|------|---------|
| -32600 | Invalid Request (malformed JSON) |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32603 | Internal error |
| -32700 | Parse error |
| 401 | Unauthorized (missing/invalid token) |
| 403 | Forbidden (insufficient permissions) |
