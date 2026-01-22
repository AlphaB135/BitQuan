# JSON-RPC API Calls

This example shows you how to use BitQuan's JSON-RPC API to interact with the node programmatically.

## Prerequisites

- BitQuan node running with RPC enabled
- curl or similar HTTP client
- 15 minutes

**Start node with RPC:**
```bash
./target/release/bitquan-node --network devnet
```

RPC will be available at `http://127.0.0.1:18443`

## Example 1: Basic RPC Call

### Call info Method

```bash
curl -X POST http://127.0.0.1:18443 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "info",
    "params": [],
    "id": 1
  }'
```

### Expected Output

```json
{
  "jsonrpc": "2.0",
  "result": {
    "version": "v1.0-audit-20251122",
    "network": "devnet",
    "chain_height": 0,
    "best_block": null,
    "peers": 0,
    "syncing": false
  },
  "id": 1
}
```

## Example 2: Get Block Count

### Request

```bash
curl -X POST http://127.0.0.1:18443 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "getblockcount",
    "params": [],
    "id": 1
  }'
```

### Expected Output

```json
{
  "jsonrpc": "2.0",
  "result": 116,
  "id": 1
}
```

## Example 3: Get Block Hash

### Request Block Hash at Height

```bash
curl -X POST http://127.0.0.1:18443 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "getblockhash",
    "params": [100],
    "id": 1
  }'
```

### Expected Output

```json
{
  "jsonrpc": "2.0",
  "result": "0000...abcd1234",
  "id": 1
}
```

## Example 4: Get Block Details

### Request Block by Hash

```bash
curl -X POST http://127.0.0.1:18443 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "getblock",
    "params": ["0000...abcd1234"],
    "id": 1
  }'
```

### Expected Output

```json
{
  "jsonrpc": "2.0",
  "result": {
    "header": {
      "version": 1,
      "prev_block": "0000...abcd1233",
      "merkle_root": "0000...5678abcd",
      "pqc_agg_hint": "0000...0000",
      "time": 1705459200,
      "bits": 545259519,
      "nonce": 0,
      "algo_id": 0
    },
    "transactions": [
      {
        "inputs": [
          {
            "prev_txid": "0000...0000",
            "vout": 0,
            "script_sig": ""
          }
        ],
        "outputs": [
          {
            "value": "50000000000000000000",
            "script_pubkey": "a820..."
          }
        ]
      }
    ]
  },
  "id": 1
}
```

## Example 5: Get Balance

### Request Balance for Address

```bash
curl -X POST http://127.0.0.1:18443 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "balance",
    "params": ["bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q"],
    "id": 1
  }'
```

### Expected Output

```json
{
  "jsonrpc": "2.0",
  "result": {
    "address": "bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q",
    "utxo_count": 100,
    "balance_qbits": "500000000000000000000",
    "balance_bq": "50.000000000000000000"
  },
  "id": 1
}
```

## Example 6: Submit Transaction

### Submit Signed Transaction

```bash
curl -X POST http://127.0.0.1:18443 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "submittransaction",
    "params": ["<hex-encoded-transaction>"],
    "id": 1
  }'
```

### Expected Output

```json
{
  "jsonrpc": "2.0",
  "result": {
    "txid": "b6a327f6490e48eaff9ec30bb6c3876244ce44704a1e9345f45da040189f1b5c",
    "accepted": true
  },
  "id": 1
}
```

## Example 7: Batch Requests

### Multiple Calls in One Request

```bash
curl -X POST http://127.0.0.1:18443 \
  -H "Content-Type: application/json" \
  -d '[
    {
      "jsonrpc": "2.0",
      "method": "getblockcount",
      "params": [],
      "id": 1
    },
    {
      "jsonrpc": "2.0",
      "method": "getbestblock",
      "params": [],
      "id": 2
    },
    {
      "jsonrpc": "2.0",
      "method": "info",
      "params": [],
      "id": 3
    }
  ]'
```

### Expected Output

```json
[
  {
    "jsonrpc": "2.0",
    "result": 116,
    "id": 1
  },
  {
    "jsonrpc": "2.0",
    "result": {
      "hash": "0000...abcd1234",
      "height": 116
    },
    "id": 2
  },
  {
    "jsonrpc": "2.0",
    "result": {
      "version": "v1.0-audit-20251122",
      "network": "devnet",
      "chain_height": 116
    },
    "id": 3
  }
]
```

## Example 8: Python Integration

### Simple Python Client

```python
#!/usr/bin/env python3
import requests
import json

class BitQuanRPC:
    def __init__(self, url="http://127.0.0.1:18443"):
        self.url = url
        self.headers = {"Content-Type": "application/json"}
        self.request_id = 0

    def call(self, method, params=[]):
        self.request_id += 1
        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": self.request_id
        }
        response = requests.post(
            self.url,
            headers=self.headers,
            data=json.dumps(payload)
        )
        return response.json()["result"]

# Usage
rpc = BitQuanRPC()

# Get block count
count = rpc.call("getblockcount")
print(f"Block count: {count}")

# Get info
info = rpc.call("info")
print(f"Network: {info['network']}")
print(f"Height: {info['chain_height']}")

# Get balance
balance = rpc.call("balance", ["bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q"])
print(f"Balance: {balance['balance_bq']} BQ")
```

### Expected Output

```
Block count: 116
Network: devnet
Height: 116
Balance: 50.000000000000000000 BQ
```

## Example 9: WebSocket Connection

### Subscribe to Block Notifications

```javascript
// Node.js example
const WebSocket = require('ws');

const ws = new WebSocket('ws://127.0.0.1:18443');

ws.on('open', () => {
  console.log('Connected to RPC WebSocket');

  // Subscribe to new blocks
  ws.send(JSON.stringify({
    jsonrpc: "2.0",
    method: "subscribe",
    params: ["blocks"],
    id: 1
  }));
});

ws.on('message', (data) => {
  const msg = JSON.parse(data);
  console.log('New block:', msg.result);
});

ws.on('error', (error) => {
  console.error('WebSocket error:', error);
});
```

## Available RPC Methods

| Method | Parameters | Description |
|--------|------------|-------------|
| `info` | None | Node information |
| `getblockcount` | None | Current block height |
| `getblockhash` | height | Block hash at height |
| `getblock` | hash | Block details |
| `getbestblock` | None | Best block hash/height |
| `balance` | address | Address balance |
| `submittransaction` | tx_hex | Submit transaction |
| `mempool` | None | Pending transactions |
| `peers` | None | Connected peers |

## Common Errors

### Error: Method Not Found

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32601,
    "message": "Method not found"
  },
  "id": 1
}
```

**Solution:** Check method name and spelling.

### Error: Invalid Params

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Invalid params"
  },
  "id": 1
}
```

**Solution:** Verify parameter types and count.

### Error: Parse Error

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32700,
    "message": "Parse error"
  },
  "id": null
}
```

**Solution:** Check JSON syntax.

## RPC Authentication

### JWT Authentication (Production)

For production/mainnet, RPC requires JWT authentication:

```bash
# 1. Get JWT token
TOKEN=$(cat /path/to/jwt.hex)

# 2. Include in request header
curl -X POST http://127.0.0.1:18443 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "info",
    "params": [],
    "id": 1
  }'
```

See [JWT Quick Start](../guides/JWT_QUICK_START.md) for setup details.

## Advanced Usage

### Error Handling

```python
def call_with_retry(self, method, params=[], max_retries=3):
    for attempt in range(max_retries):
        try:
            return self.call(method, params)
        except requests.exceptions.ConnectionError:
            if attempt < max_retries - 1:
                time.sleep(2 ** attempt)  # Exponential backoff
                continue
            raise
```

### Async Requests

```python
import asyncio
import aiohttp

async def async_call(method, params=[]):
    async with aiohttp.ClientSession() as session:
        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        }
        async with session.post(
            "http://127.0.0.1:18443",
            json=payload
        ) as response:
            return await response.json()

# Usage
async def main():
    result = await async_call("getblockcount")
    print(result)

asyncio.run(main())
```

## Testing RPC

### Test All Methods

```bash
#!/bin/bash
# test-rpc.sh - Test all RPC methods

METHODS=(
  "info"
  "getblockcount"
  "getbestblock"
  "mempool"
  "peers"
)

for method in "${METHODS[@]}"; do
  echo "Testing: $method"
  curl -s -X POST http://127.0.0.1:18443 \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":[],\"id\":1}" \
    | jq '.'
  echo ""
done
```

## What's Next?

- [API Reference](../api/rpc/API_REFERENCE.md) - Full API documentation
- [JWT Setup](../guides/JWT_QUICK_START.md) - Authentication guide
- [Operations](../operations/README.md) - Node operations

## Related Documentation

- [RPC API Reference](../api/rpc/API_REFERENCE.md) - Complete API docs
- [RPC Testing](../api/rpc/testing.md) - Testing guide
- [JWT Authentication](../guides/JWT_QUICK_START.md) - Auth setup
