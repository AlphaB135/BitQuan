#!/usr/bin/env python3
"""
Load testing script for BitQuan P2P network.

This script simulates many concurrent connections to test:
1. Server scalability
2. Memory usage under load
3. Connection handling performance
"""

import socket
import time
import argparse
import threading
import sys
import psutil
import os

class LoadTestResult:
    def __init__(self):
        self.total_connections = 0
        self.successful_connections = 0
        self.failed_connections = 0
        self.start_time = 0
        self.end_time = 0
        self.peak_memory = 0

def monitor_memory(result, duration=1):
    """
    Monitor memory usage during the test.
    """
    process = psutil.Process(os.getpid())

    while True:
        try:
            memory_mb = process.memory_info().rss / 1024 / 1024
            result.peak_memory = max(result.peak_memory, memory_mb)
            time.sleep(duration)
        except:
            break

def create_connection(host, port, connection_id, duration=10):
    """
    Create a single connection and keep it alive.
    """
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(5)
        s.connect((host, port))

        # Send some data
        s.send(b'Hello from connection ' + str(connection_id).encode() + b'\n')

        # Keep connection alive
        start_time = time.time()
        while time.time() - start_time < duration:
            try:
                # Try to read any response
                data = s.recv(1024)
                if not data:
                    break
                time.sleep(1)
            except socket.timeout:
                # Timeout is expected, continue
                continue
            except:
                break

        s.close()
        return True

    except Exception as e:
        # print(f"Connection {connection_id} failed: {e}")
        return False

def concurrent_load_test(host, port, connections, duration, batch_size=50):
    """
    Perform concurrent load test with specified number of connections.
    """
    print(f"[*] Starting load test with {connections} connections for {duration}s each")
    print(f"[*] Batch size: {batch_size}")

    result = LoadTestResult()
    result.total_connections = connections
    result.start_time = time.time()

    # Start memory monitoring
    memory_thread = threading.Thread(
        target=monitor_memory,
        args=(result,),
        daemon=True
    )
    memory_thread.start()

    # Create connections in batches
    active_connections = 0
    connection_threads = []

    for i in range(connections):
        thread = threading.Thread(
            target=create_connection,
            args=(host, port, i + 1, duration),
            daemon=True
        )

        try:
            thread.start()
            connection_threads.append(thread)
            active_connections += 1
            result.successful_connections += 1

            # Add small delay between connections to avoid overwhelming the server
            time.sleep(0.01)

            # Print progress
            if (i + 1) % batch_size == 0:
                print(f"[*] Launched {i + 1}/{connections} connections")

        except Exception as e:
            result.failed_connections += 1
            print(f"[-] Failed to launch connection {i + 1}: {e}")

    print(f"[*] All {connections} connections launched. Waiting for completion...")

    # Wait for all connections to complete
    for thread in connection_threads:
        thread.join(timeout=duration + 10)

    result.end_time = time.time()

    return result

def print_results(result):
    """
    Print load test results.
    """
    duration = result.end_time - result.start_time
    success_rate = (result.successful_connections / result.total_connections) * 100

    print("\n" + "=" * 60)
    print("LOAD TEST RESULTS")
    print("=" * 60)
    print(f"Total connections attempted: {result.total_connections}")
    print(f"Successful connections: {result.successful_connections}")
    print(f"Failed connections: {result.failed_connections}")
    print(f"Success rate: {success_rate:.1f}%")
    print(f"Test duration: {duration:.1f} seconds")
    print(f"Peak memory usage: {result.peak_memory:.1f} MB")
    print(f"Average connection rate: {result.successful_connections/duration:.1f} conn/s")

    if success_rate >= 95:
        print("\n[+] EXCELLENT: Server handled the load successfully!")
    elif success_rate >= 80:
        print("\n[+] GOOD: Server handled most of the load")
    else:
        print("\n[-] POOR: Server struggled with the load")

    # Memory usage analysis
    memory_per_connection = result.peak_memory / max(result.successful_connections, 1)
    print(f"\nMemory usage: {memory_per_connection:.2f} MB per connection")

    if memory_per_connection < 1:
        print("[+] EXCELLENT: Very efficient memory usage (async-like)")
    elif memory_per_connection < 5:
        print("[+] GOOD: Reasonable memory usage")
    else:
        print("[-] HIGH: Memory usage could be improved")

def main():
    parser = argparse.ArgumentParser(description='Load test BitQuan P2P server')
    parser.add_argument('--host', default='127.0.0.1', help='Target host (default: 127.0.0.1)')
    parser.add_argument('--port', type=int, default=18444, help='Target port (default: 18444)')
    parser.add_argument('--connections', type=int, default=1000, help='Number of connections (default: 1000)')
    parser.add_argument('--duration', type=int, default=10, help='Duration per connection in seconds (default: 10)')
    parser.add_argument('--batch-size', type=int, default=50, help='Connections to launch per batch (default: 50)')

    args = parser.parse_args()

    print("=" * 60)
    print("BitQuan P2P Network Load Test")
    print("=" * 60)
    print(f"Target: {args.host}:{args.port}")
    print(f"Connections: {args.connections}")
    print(f"Duration: {args.duration}s per connection")

    # Check server responsiveness first
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(5)
        s.connect((args.host, args.port))
        s.close()
        print(f"[+] Server at {args.host}:{args.port} is responsive")
    except Exception as e:
        print(f"[-] Cannot connect to server: {e}")
        sys.exit(1)

    # Run load test
    print()
    result = concurrent_load_test(
        args.host,
        args.port,
        args.connections,
        args.duration,
        args.batch_size
    )

    # Print results
    print_results(result)

    # Exit based on success rate
    success_rate = (result.successful_connections / result.total_connections) * 100
    sys.exit(0 if success_rate >= 80 else 1)

if __name__ == '__main__':
    try:
        main()
    except KeyboardInterrupt:
        print("\n[!] Test interrupted by user")
        sys.exit(2)
    except Exception as e:
        print(f"\n[!] Unexpected error: {e}")
        sys.exit(3)
