# JWT Manual Testing Guide

## Quick Test with curl

### Step 1: Start the node (when CLI is ready)
```bash
# With JWT enabled
cargo run --bin bitquan-node -- \
  --network devnet \
  --rpc-addr 127.0.0.1:18332 \
  --jwt-secret "my-super-secret-key-change-in-production"
```

### Step 2: Test Login
```bash
# Login to get JWT token
curl -X POST http://localhost:18332/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'

# Expected response:
# {
#   "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
#   "token_type": "Bearer",
#   "expires_in": 3600
# }
```

### Step 3: Save the token
```bash
# Extract token (on macOS/Linux)
TOKEN=$(curl -s -X POST http://localhost:18332/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}' \
  | jq -r '.access_token')

echo "Token: $TOKEN"
```

### Step 4: Use JWT token for RPC
```bash
# Call RPC method with Bearer token
curl -X POST http://localhost:18332 \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "getblockcount",
    "id": 1
  }'

# Expected response:
# {
#   "jsonrpc": "2.0",
#   "result": 123,
#   "id": 1
# }
```

### Step 5: Test invalid credentials
```bash
# Try wrong password
curl -X POST http://localhost:18332/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"wrongpassword"}'

# Expected response (401):
# {
#   "error": "invalid_credentials",
#   "message": "Invalid password"
# }
```

### Step 6: Test without token (should fail)
```bash
# Try RPC without Authorization header
curl -X POST http://localhost:18332 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "getblockcount",
    "id": 1
  }'

# Expected response (401 Unauthorized)
```

### Step 7: Test expired token (after 1 hour)
```bash
# Use old token after expiration
curl -X POST http://localhost:18332 \
  -H "Authorization: Bearer $EXPIRED_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "getblockcount",
    "id": 1
  }'

# Expected: 401 Unauthorized with "Token expired" message
```

## Test with Python

```python
import requests
import json

# 1. Login
login_url = "http://localhost:18332/auth/login"
login_data = {
    "username": "admin",
    "password": "admin123"
}

response = requests.post(login_url, json=login_data)
print(f"Login Status: {response.status_code}")

if response.status_code == 200:
    token_data = response.json()
    token = token_data['access_token']
    print(f"Token: {token[:50]}...")
    
    # 2. Use token for RPC
    rpc_url = "http://localhost:18332"
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json"
    }
    rpc_data = {
        "jsonrpc": "2.0",
        "method": "getblockcount",
        "id": 1
    }
    
    rpc_response = requests.post(rpc_url, headers=headers, json=rpc_data)
    print(f"RPC Status: {rpc_response.status_code}")
    print(f"RPC Response: {rpc_response.json()}")
```

## Test with JavaScript/Node.js

```javascript
const fetch = require('node-fetch');

async function test() {
    // 1. Login
    const loginResponse = await fetch('http://localhost:18332/auth/login', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({
            username: 'admin',
            password: 'admin123'
        })
    });
    
    const {access_token} = await loginResponse.json();
    console.log('Token:', access_token.substring(0, 50) + '...');
    
    // 2. Use token
    const rpcResponse = await fetch('http://localhost:18332', {
        method: 'POST',
        headers: {
            'Authorization': `Bearer ${access_token}`,
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            jsonrpc: '2.0',
            method: 'getblockcount',
            id: 1
        })
    });
    
    const result = await rpcResponse.json();
    console.log('Result:', result);
}

test().catch(console.error);
```

## Verify JWT Token at jwt.io

1. Copy your token
2. Go to https://jwt.io/
3. Paste token in "Encoded" section
4. You should see decoded claims:
```json
{
  "sub": "admin",
  "role": "admin",
  "exp": 1730567890,
  "iat": 1730564290
}
```

## Expected Endpoints

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/health` | GET | No | Health check |
| `/auth/login` | POST | No | Get JWT token |
| `/` | POST | JWT | JSON-RPC calls |

## Status Codes

| Code | Meaning | When |
|------|---------|------|
| 200 | OK | Successful login or RPC call |
| 400 | Bad Request | Invalid JSON |
| 401 | Unauthorized | Invalid credentials or expired token |
| 503 | Service Unavailable | JWT not configured |

---

## Current Status ✅

**What works**:
- ✅ Login endpoint `/auth/login`
- ✅ JWT token generation
- ✅ Bearer token authentication
- ✅ Error responses

**What's missing**:
- ⏳ CLI integration (--jwt-secret flag)
- ⏳ Config file support
- ⏳ Token refresh endpoint
- ⏳ Password hashing (using plaintext!)

**Security Warning** ⚠️:
- Passwords are stored in plaintext
- JWT secret is hardcoded
- FOR DEVELOPMENT ONLY!

---

## Next Steps

1. Add `--jwt-secret` CLI flag
2. Hash passwords with Argon2
3. Load users from config file
4. Add `/auth/refresh` endpoint
5. Add integration tests

**Estimated time**: 4-6 hours
