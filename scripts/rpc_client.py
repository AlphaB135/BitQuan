#!/usr/bin/env python3
"""
BitQuan RPC Client - Python SDK Example
Demonstrates how to interact with BitQuan node via JSON-RPC
"""

import requests
from typing import Any, Dict

class BitQuanRPC:
    """Simple JSON-RPC 2.0 client for BitQuan node"""

    def __init__(self, url: str = "http://127.0.0.1:8332", timeout: int = 30):
        self.url = url
        self.timeout = timeout
        self.id_counter = 0

    def _call(self, method: str, params: list = None) -> Any:
        """Make a JSON-RPC call"""
        self.id_counter += 1

        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params or [],
            "id": self.id_counter
        }

        try:
            response = requests.post(
                self.url,
                json=payload,
                timeout=self.timeout,
                headers={"Content-Type": "application/json"}
            )
            response.raise_for_status()

            data = response.json()

            if "error" in data and data["error"] is not None:
                raise Exception(f"RPC Error: {data['error']}")

            return data.get("result")

        except requests.exceptions.RequestException as e:
            raise Exception(f"Connection error: {e}")

    # Blockchain methods
    def getblockcount(self) -> int:
        """Get current block height"""
        return self._call("getblockcount")

    def getblockchaininfo(self) -> Dict:
        """Get blockchain information"""
        return self._call("getblockchaininfo")

    def getbestblockhash(self) -> str:
        """Get hash of best (tip) block"""
        return self._call("getbestblockhash")

    def getblockhash(self, height: int) -> str:
        """Get block hash at specific height"""
        return self._call("getblockhash", [height])

    # Mining methods
    def getmininginfo(self) -> Dict:
        """Get mining information"""
        return self._call("getmininginfo")

    def getblocktemplate(self) -> Dict:
        """Get block template for mining"""
        return self._call("getblocktemplate")

    def submitblock(self, block_hex: str) -> bool:
        """Submit a mined block"""
        return self._call("submitblock", [block_hex])

    # Transaction methods
    def gettransaction(self, txid: str) -> Dict:
        """Get transaction by ID"""
        return self._call("gettransaction", [txid])


def main():
    """Example usage"""
    print("BitQuan RPC Client - Python SDK")
    print("=" * 50)
    print()

    # Connect to local node
    client = BitQuanRPC("http://127.0.0.1:8332")

    try:
        # Get blockchain info
        print("📊 Blockchain Info:")
        info = client.getblockchaininfo()
        print(f"   Chain: {info.get('chain', 'unknown')}")
        print(f"   Blocks: {info.get('blocks', 0)}")
        print(f"   Best hash: {info.get('bestblockhash', 'N/A')[:16]}...")
        print(f"   Difficulty: {info.get('difficulty', 0)}")
        print()

        # Get mining info
        print("⛏️  Mining Info:")
        mining = client.getmininginfo()
        print(f"   Blocks: {mining.get('blocks', 0)}")
        print(f"   Difficulty: {mining.get('difficulty', 0)}")
        print(f"   Network hashrate: {mining.get('networkhashps', 0):.2f} H/s")
        print()

        # Get block count
        height = client.getblockcount()
        print(f"📏 Current Height: {height}")
        print()

        if height > 0:
            # Get tip block hash
            tip_hash = client.getbestblockhash()
            print(f"🔝 Tip Block Hash: {tip_hash}")

            # Get genesis block hash
            genesis_hash = client.getblockhash(0)
            print(f"🌱 Genesis Block Hash: {genesis_hash}")

        print()
        print("✅ RPC connection successful!")

    except Exception as e:
        print(f"❌ Error: {e}")
        print()
        print("Make sure BitQuan node is running:")
        print("  cargo run --release --features rocksdb-backend -- run")


if __name__ == "__main__":
    main()
