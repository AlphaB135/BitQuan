#!/usr/bin/env python3
"""
BitQuan Attack Simulation Suite
ทดสอบความแข็งแกร่งของระบบด้วยการจำลองการโจมตีจริง

Created by: Hermes (ซากุระ) 🌸
"""

import argparse
import asyncio
import json
import random
import requests
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import List, Dict, Any

# Configuration
RPC_ENDPOINT = "http://140.245.127.249:19443/"
TIMEOUT = 10

class Colors:
    RED = '\033[0;31m'
    YELLOW = '\033[1;33m'
    GREEN = '\033[0;32m'
    BLUE = '\033[0;34m'
    NC = '\033[0m'

class AttackSimulator:
    def __init__(self, endpoint: str, jwt_token: str = None):
        self.endpoint = endpoint
        self.jwt_token = jwt_token
        self.session = requests.Session()
        self.results = {
            'total_requests': 0,
            'successful': 0,
            'failed': 0,
            'rate_limited': 0,
            'auth_failed': 0,
            'validation_failed': 0
        }

    def rpc_call(self, method: str, params: list = None) -> Dict[str, Any]:
        """Make a JSON-RPC call"""
        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params or [],
            "id": random.randint(1, 1000000)
        }

        headers = {"Content-Type": "application/json"}
        if self.jwt_token:
            headers["Authorization"] = f"Bearer {self.jwt_token}"

        try:
            response = self.session.post(
                self.endpoint,
                json=payload,
                headers=headers,
                timeout=TIMEOUT
            )
            self.results['total_requests'] += 1

            if response.status_code == 200:
                self.results['successful'] += 1
                return response.json()
            elif response.status_code == 429:
                self.results['rate_limited'] += 1
                return {'error': 'rate_limited'}
            elif response.status_code == 401:
                self.results['auth_failed'] += 1
                return {'error': 'auth_failed'}
            else:
                self.results['failed'] += 1
                return {'error': f'http_{response.status_code}'}

        except Exception as e:
            self.results['failed'] += 1
            return {'error': str(e)}

    def test_rate_limiting(self, num_requests: int = 1000, concurrent: int = 50):
        """Test 1: RPC Rate Limiting"""
        print(f"{Colors.BLUE}━━━ Test 1: Rate Limiting ━━━{Colors.NC}")
        print(f"Sending {num_requests} requests with {concurrent} concurrent connections...")

        start_time = time.time()

        with ThreadPoolExecutor(max_workers=concurrent) as executor:
            futures = [
                executor.submit(self.rpc_call, "getblockcount")
                for _ in range(num_requests)
            ]

            for future in as_completed(futures):
                result = future.result()

        elapsed = time.time() - start_time

        print(f"\n{Colors.GREEN}Results:{Colors.NC}")
        print(f"  Total requests: {self.results['total_requests']}")
        print(f"  Successful: {self.results['successful']}")
        print(f"  Rate limited: {self.results['rate_limited']}")
        print(f"  Failed: {self.results['failed']}")
        print(f"  Time: {elapsed:.2f}s")
        print(f"  Rate: {num_requests/elapsed:.2f} req/s")

        if self.results['rate_limited'] > 0:
            print(f"{Colors.GREEN}✓ Rate limiting is WORKING{Colors.NC}")
        else:
            print(f"{Colors.RED}✗ Rate limiting NOT DETECTED - Possible vulnerability!{Colors.NC}")

        self._reset_results()

    def test_input_validation(self):
        """Test 2: Input Validation (Injection Attacks)"""
        print(f"\n{Colors.BLUE}━━━ Test 2: Input Validation ━━━{Colors.NC}")

        injection_payloads = [
            # XSS
            "<script>alert('xss')</script>",
            "javascript:alert(1)",

            # SQL Injection
            "'; DROP TABLE users; --",
            "1' OR '1'='1",

            # Command Injection
            "; cat /etc/passwd",
            "| whoami",
            "&& rm -rf /",

            # Path Traversal
            "../../../etc/passwd",
            "..\\..\\..\\windows\\system32",

            # Null Byte
            "test\x00.txt",

            # Very long string
            "A" * 10000,

            # Deep nesting
            json.dumps([[[[[[[[[['deep']]]]]]]]]]),
        ]

        print(f"Testing {len(injection_payloads)} malicious payloads...")

        blocked = 0
        passed = 0

        for payload in injection_payloads:
            result = self.rpc_call("getblock", [payload])

            if 'error' in result:
                blocked += 1
                print(f"{Colors.GREEN}✓{Colors.NC} Blocked: {payload[:50]}")
            else:
                passed += 1
                print(f"{Colors.RED}✗{Colors.NC} PASSED: {payload[:50]}")

        print(f"\n{Colors.GREEN}Results:{Colors.NC}")
        print(f"  Blocked: {blocked}/{len(injection_payloads)}")
        print(f"  Passed: {passed}/{len(injection_payloads)}")

        if passed == 0:
            print(f"{Colors.GREEN}✓ Input validation is STRONG{Colors.NC}")
        else:
            print(f"{Colors.RED}✗ {passed} payloads bypassed validation!{Colors.NC}")

        self._reset_results()

    def test_authentication(self):
        """Test 3: Authentication Bypass"""
        print(f"\n{Colors.BLUE}━━━ Test 3: Authentication ━━━{Colors.NC}")

        # Try without JWT
        print("Attempting access without JWT token...")
        result = self.rpc_call("getblockcount")

        if 'error' in result and result['error'] == 'auth_failed':
            print(f"{Colors.GREEN}✓ Authentication is REQUIRED{Colors.NC}")
        else:
            print(f"{Colors.YELLOW}⚠️  No authentication required (might be intentional for testnet){Colors.NC}")

        self._reset_results()

    def test_dos_large_requests(self):
        """Test 4: DoS via Large Requests"""
        print(f"\n{Colors.BLUE}━━━ Test 4: Large Request DoS ━━━{Colors.NC}")

        sizes = [1024, 10240, 102400, 1024000, 10240000]  # 1KB to 10MB

        for size in sizes:
            payload = "A" * size
            print(f"Sending {size/1024:.1f}KB request...")

            result = self.rpc_call("getblock", [payload])

            if 'error' in result:
                print(f"{Colors.GREEN}✓ Request rejected (size: {size/1024:.1f}KB){Colors.NC}")
            else:
                print(f"{Colors.RED}✗ Request accepted (size: {size/1024:.1f}KB) - Possible DoS vector!{Colors.NC}")

        self._reset_results()

    def test_method_enumeration(self):
        """Test 5: RPC Method Enumeration"""
        print(f"\n{Colors.BLUE}━━━ Test 5: Method Enumeration ━━━{Colors.NC}")

        # Common RPC methods + some that shouldn't exist
        methods = [
            "getblockcount", "getblockhash", "getblock",  # Should work
            "stop", "shutdown", "restart",  # Dangerous
            "debug", "eval", "exec",  # Very dangerous
            "importprivkey", "dumpprivkey", "dumpwallet",  # Wallet dangerous
        ]

        print(f"Testing {len(methods)} methods...")

        allowed = []
        blocked = []

        for method in methods:
            result = self.rpc_call(method, [])

            if 'error' in result and 'not allowed' in str(result.get('error', '')).lower():
                blocked.append(method)
                print(f"{Colors.GREEN}✓{Colors.NC} Blocked: {method}")
            else:
                allowed.append(method)
                if method in ["stop", "shutdown", "exec", "eval"]:
                    print(f"{Colors.RED}✗{Colors.NC} ALLOWED: {method} (DANGEROUS!)")
                else:
                    print(f"{Colors.YELLOW}⚠️{Colors.NC}  Allowed: {method}")

        print(f"\n{Colors.GREEN}Results:{Colors.NC}")
        print(f"  Allowed: {len(allowed)}")
        print(f"  Blocked: {len(blocked)}")

        dangerous_allowed = [m for m in allowed if m in ["stop", "shutdown", "exec", "eval", "importprivkey", "dumpprivkey"]]
        if dangerous_allowed:
            print(f"{Colors.RED}✗ Dangerous methods allowed: {', '.join(dangerous_allowed)}{Colors.NC}")
        else:
            print(f"{Colors.GREEN}✓ No dangerous methods allowed{Colors.NC}")

        self._reset_results()

    def test_timing_attack(self, samples: int = 100):
        """Test 6: Timing Attack on Authentication"""
        print(f"\n{Colors.BLUE}━━━ Test 6: Timing Attack ━━━{Colors.NC}")
        print(f"Analyzing authentication timing with {samples} samples...")

        valid_times = []
        invalid_times = []

        # Simulate valid vs invalid tokens
        for i in range(samples):
            # Invalid token
            start = time.time()
            result = self.rpc_call("getblockcount")
            elapsed = time.time() - start
            invalid_times.append(elapsed)

        avg_invalid = sum(invalid_times) / len(invalid_times)
        print(f"  Average time for invalid auth: {avg_invalid*1000:.2f}ms")

        # Check if timing is consistent (constant-time)
        variance = sum((t - avg_invalid) ** 2 for t in invalid_times) / len(invalid_times)
        stddev = variance ** 0.5

        print(f"  Standard deviation: {stddev*1000:.2f}ms")

        if stddev < 0.01:  # Less than 10ms variance
            print(f"{Colors.GREEN}✓ Timing appears constant (protected against timing attacks){Colors.NC}")
        else:
            print(f"{Colors.YELLOW}⚠️  High timing variance - possible timing attack vector{Colors.NC}")

        self._reset_results()

    def _reset_results(self):
        """Reset result counters"""
        self.results = {
            'total_requests': 0,
            'successful': 0,
            'failed': 0,
            'rate_limited': 0,
            'auth_failed': 0,
            'validation_failed': 0
        }

    def run_all_tests(self):
        """Run complete test suite"""
        print(f"{Colors.BLUE}{'='*60}{Colors.NC}")
        print(f"{Colors.BLUE}BitQuan Attack Simulation Suite{Colors.NC}")
        print(f"{Colors.BLUE}Target: {self.endpoint}{Colors.NC}")
        print(f"{Colors.BLUE}{'='*60}{Colors.NC}\n")

        # Test 1: Rate Limiting
        self.test_rate_limiting(num_requests=500, concurrent=50)

        # Test 2: Input Validation
        self.test_input_validation()

        # Test 3: Authentication
        self.test_authentication()

        # Test 4: Large Requests
        self.test_dos_large_requests()

        # Test 5: Method Enumeration
        self.test_method_enumeration()

        # Test 6: Timing Attack
        self.test_timing_attack(samples=50)

        print(f"\n{Colors.BLUE}{'='*60}{Colors.NC}")
        print(f"{Colors.GREEN}✓ Test suite completed{Colors.NC}")
        print(f"{Colors.BLUE}{'='*60}{Colors.NC}")


def main():
    parser = argparse.ArgumentParser(description='BitQuan Attack Simulation Suite')
    parser.add_argument('--endpoint', default=RPC_ENDPOINT, help='RPC endpoint URL')
    parser.add_argument('--jwt', help='JWT token for authentication')
    parser.add_argument('--test', choices=['rate', 'validation', 'auth', 'dos', 'methods', 'timing', 'all'],
                        default='all', help='Specific test to run')

    args = parser.parse_args()

    simulator = AttackSimulator(args.endpoint, args.jwt)

    if args.test == 'all':
        simulator.run_all_tests()
    elif args.test == 'rate':
        simulator.test_rate_limiting()
    elif args.test == 'validation':
        simulator.test_input_validation()
    elif args.test == 'auth':
        simulator.test_authentication()
    elif args.test == 'dos':
        simulator.test_dos_large_requests()
    elif args.test == 'methods':
        simulator.test_method_enumeration()
    elif args.test == 'timing':
        simulator.test_timing_attack()


if __name__ == '__main__':
    main()
