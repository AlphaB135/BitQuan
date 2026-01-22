#!/usr/bin/env python3
"""
BitQuan Testnet Faucet
Distributes testnet coins for development and testing
"""

import os
from datetime import datetime, timedelta
from flask import Flask, request, jsonify, render_template_string
from typing import Dict, Optional
import requests

app = Flask(__name__)

# Configuration
FAUCET_WALLET = "tools/faucet-wallet.keystore"
FAUCET_PASSWORD = os.getenv("FAUCET_PASSWORD", "faucet123")
RPC_URL = "http://127.0.0.1:19443"
DISTRIBUTION_AMOUNT = 100000000  # 1 BQ in qbits
COOLDOWN_HOURS = 1
MAX_DAILY_PER_IP = 5

# In-memory storage (use Redis in production)
faucet_db: Dict[str, Dict] = {}

def get_client_ip():
    """Get client IP address"""
    if request.headers.getlist("X-Forwarded-For"):
        return request.headers.getlist("X-Forwarded-For")[0]
    return request.remote_addr

def check_rate_limit(ip: str) -> Optional[str]:
    """Check if IP is rate limited"""
    now = datetime.now()

    if ip not in faucet_db:
        faucet_db[ip] = {
            "last_request": now,
            "daily_count": 1,
            "daily_reset": now.replace(hour=0, minute=0, second=0, microsecond=0)
        }
        return None

    user_data = faucet_db[ip]

    # Reset daily counter if needed
    if now.date() > user_data["daily_reset"].date():
        user_data["daily_count"] = 0
        user_data["daily_reset"] = now.replace(hour=0, minute=0, second=0, microsecond=0)

    # Check cooldown
    if now - user_data["last_request"] < timedelta(hours=COOLDOWN_HOURS):
        return f"Please wait {COOLDOWN_HOURS} hours between requests"

    # Check daily limit
    if user_data["daily_count"] >= MAX_DAILY_PER_IP:
        return f"Maximum {MAX_DAILY_PER_IP} requests per day per IP"

    return None

def send_transaction(address: str) -> Optional[str]:
    """Send transaction via RPC"""
    try:
        payload = {
            "jsonrpc": "2.0",
            "method": "wallet-send",
            "params": {
                "keystore": FAUCET_WALLET,
                "to": address,
                "amount": DISTRIBUTION_AMOUNT,
                "fee_rate": 1,
                "password": FAUCET_PASSWORD
            },
            "id": 1
        }

        response = requests.post(RPC_URL, json=payload, timeout=30)
        if response.status_code == 200:
            result = response.json()
            if "result" in result:
                return result["result"].get("txid")

        return None
    except Exception as e:
        print(f"Transaction error: {e}")
        return None

# HTML Template
FAUCET_HTML = """
<!DOCTYPE html>
<html>
<head>
    <title>BitQuan Testnet Faucet</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        .header { text-align: center; color: #4CAF50; margin-bottom: 30px; }
        .form { background: #f5f5f5; padding: 20px; border-radius: 8px; }
        .input { width: 100%; padding: 10px; margin: 10px 0; border: 1px solid #ddd; border-radius: 4px; }
        .button { background: #4CAF50; color: white; padding: 12px 24px; border: none; border-radius: 4px; cursor: pointer; width: 100%; }
        .button:hover { background: #45a049; }
        .info { background: #e7f3fe; padding: 15px; border-radius: 4px; margin: 20px 0; }
        .error { background: #ffebee; color: #c62828; padding: 15px; border-radius: 4px; margin: 20px 0; }
        .success { background: #e8f5e8; color: #2e7d32; padding: 15px; border-radius: 4px; margin: 20px 0; }
        .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin: 20px 0; }
        .stat-card { background: white; padding: 15px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
    </style>
</head>
<body>
    <div class="header">
        <h1>🚰 BitQuan Testnet Faucet</h1>
        <p>Get free testnet coins for development and testing</p>
    </div>

    <div class="stats">
        <div class="stat-card">
            <h3>💰 Amount</h3>
            <p>1.0 BQ per request</p>
        </div>
        <div class="stat-card">
            <h3>⏰ Cooldown</h3>
            <p>{{ cooldown_hours }} hours</p>
        </div>
        <div class="stat-card">
            <h3>📊 Daily Limit</h3>
            <p>{{ max_daily }} requests per IP</p>
        </div>
    </div>

    <div class="info">
        <h3>📋 How to use:</h3>
        <ol>
            <li>Get your BitQuan testnet address using: <code>./bitquan-node wallet-address --keystore your-wallet.keystore</code></li>
            <li>Enter your address below and click "Get Testnet Coins"</li>
            <li>Wait for transaction to confirm (check block explorer)</li>
        </ol>
    </div>

    {% if message %}
    <div class="{% if error %}error{% else %}success{% endif %}">
        {{ message }}
    </div>
    {% endif %}

    <div class="form">
        <form method="POST">
            <label for="address">🏠 Your BitQuan Testnet Address:</label>
            <input type="text" id="address" name="address" class="input"
                   placeholder="bq1q..." required>
            <button type="submit" class="button">🚰 Get Testnet Coins</button>
        </form>
    </div>

    <div class="info">
        <h3>🔗 Useful Links:</h3>
        <ul>
            <li><a href="https://github.com/AlphaB135/BitQuan" target="_blank">BitQuan GitHub</a></li>
            <li><a href="https://explorer.testnet.bitquan.org" target="_blank">Testnet Explorer</a></li>
            <li><a href="https://docs.bitquan.org" target="_blank">Documentation</a></li>
        </ul>
    </div>
</body>
</html>
"""

@app.route("/", methods=["GET", "POST"])
def faucet():
    if request.method == "GET":
        return render_template_string(FAUCET_HTML,
                                 cooldown_hours=COOLDOWN_HOURS,
                                 max_daily=MAX_DAILY_PER_IP)

    # POST request
    address = request.form.get("address", "").strip()

    if not address:
        return render_template_string(FAUCET_HTML,
                                 message="❌ Please enter a valid address",
                                 error=True,
                                 cooldown_hours=COOLDOWN_HOURS,
                                 max_daily=MAX_DAILY_PER_IP)

    # Validate address format (basic check)
    if not address.startswith("bq1q") or len(address) < 20:
        return render_template_string(FAUCET_HTML,
                                 message="❌ Invalid BitQuan address format",
                                 error=True,
                                 cooldown_hours=COOLDOWN_HOURS,
                                 max_daily=MAX_DAILY_PER_IP)

    # Check rate limits
    ip = get_client_ip()
    rate_error = check_rate_limit(ip)
    if rate_error:
        return render_template_string(FAUCET_HTML,
                                 message=f"❌ {rate_error}",
                                 error=True,
                                 cooldown_hours=COOLDOWN_HOURS,
                                 max_daily=MAX_DAILY_PER_IP)

    # Send transaction
    txid = send_transaction(address)
    if txid:
        # Update rate limit data
        faucet_db[ip]["last_request"] = datetime.now()
        faucet_db[ip]["daily_count"] += 1

        return render_template_string(FAUCET_HTML,
                                 message=f"✅ Success! Transaction sent: {txid}",
                                 error=False,
                                 cooldown_hours=COOLDOWN_HOURS,
                                 max_daily=MAX_DAILY_PER_IP)
    else:
        return render_template_string(FAUCET_HTML,
                                 message="❌ Failed to send transaction. Please try again later.",
                                 error=True,
                                 cooldown_hours=COOLDOWN_HOURS,
                                 max_daily=MAX_DAILY_PER_IP)

@app.route("/health")
def health():
    return jsonify({"status": "ok", "timestamp": datetime.now().isoformat()})

@app.route("/stats")
def stats():
    total_distributed = len(faucet_db) * DISTRIBUTION_AMOUNT
    return jsonify({
        "total_distributed": total_distributed,
        "active_users": len(faucet_db),
        "cooldown_hours": COOLDOWN_HOURS,
        "max_daily": MAX_DAILY_PER_IP
    })

if __name__ == "__main__":
    print("🚰 BitQuan Testnet Faucet starting...")
    print(f"📍 RPC URL: {RPC_URL}")
    print(f"💰 Distribution amount: {DISTRIBUTION_AMOUNT} qbits ({DISTRIBUTION_AMOUNT/100000000} BQ)")
    print(f"⏰ Cooldown: {COOLDOWN_HOURS} hours")
    print(f"📊 Daily limit: {MAX_DAILY_PER_IP} per IP")

    app.run(host="0.0.0.0", port=8080, debug=False)
