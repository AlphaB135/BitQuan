import urllib.request
import json
import base64
import time
import os
import sys

# --- CONFIG ---
RPC_URL = "http://127.0.0.1:8332"
RPC_USER = "admin"
RPC_PASS = "mainnet_secure_2025"
MINING_ADDRESS = "bq1qy7as4ahnjngn79vslhm35vuzwkktdzqe7qz7rwug0ee3mwchjjccdl8ff6"
TARGET_HEIGHT = 101 # Maturity + 1 for confirmation
# ----------------

def get_auth_header():
    s = f"{RPC_USER}:{RPC_PASS}"
    enc = base64.b64encode(s.encode()).decode()
    return f"Basic {enc}"

def rpc_call(method, params):
    payload = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    }
    
    req = urllib.request.Request(RPC_URL)
    req.add_header('Content-Type', 'application/json')
    req.add_header('Authorization', get_auth_header())
    
    data = json.dumps(payload).encode()
    
    try:
        with urllib.request.urlopen(req, data=data, timeout=30) as f:
            resp = json.loads(f.read().decode())
            return resp
    except urllib.error.HTTPError as e:
        print(f"❌ HTTP Error: {e.code} {e.reason}")
        print(e.read().decode())
        return None
    except Exception as e:
        print(f"❌ Error: {e}")
        return None

print(f"🚀 Connecting to {RPC_URL} with Basic Auth...")

# 1. Check current height
print("📊 Checking blockchain status...")
height_resp = rpc_call("getblockcount", [])
if not height_resp or 'result' not in height_resp:
    print("❌ Failed to get block count. Exiting.")
    sys.exit(1)

current_height = height_resp['result']
print(f"   Current height: {current_height}")

# 2. Mine to maturity if needed
# FORCE mining 101 blocks because previous blocks were OP_1 (unspendable)
force_mine = 101
print(f"⚠️  Mining {force_mine} blocks to GENERATE SPENDABLE COINS (generatetoaddress)...")

for i in range(force_mine):
    print(f"🔨 Mining block {current_height + i + 1} (Spendable)...", end='\r')
    # Use generatetoaddress(nblocks, address)
    result = rpc_call("generatetoaddress", [1, MINING_ADDRESS])
    if not result or not result.get('result'):
        print(f"\n❌ Mining failed at block {current_height + i + 1}")
        sys.exit(1)

print("\n✅ Blocks mined! Maturity counting starts now.")

print("🎉 Done!")
