#!/usr/bin/env python3
import sys
import os
sys.path.insert(0, '/Users/alphab/BitQuan')

# Try to read the RocksDB to get tip info
try:
    from rocksdb import DB
    
    db_path = os.path.expanduser("~/.bitquan/mainnet")
    db = DB(db_path, read_only=True)
    
    # Try to get the tip
    tip = db.get(b'tip')
    if tip:
        print(f"Tip data: {tip.hex()}")
        
        # Try to decode as block header
        if len(tip) >= 80:  # Block header size
            # Extract bits from header (bytes 72-75)
            bits = int.from_bytes(tip[72:76], 'little')
            print(f"Current tip bits: 0x{bits:08x}")
            
            # Compare with difficulties
            mainnet_bits = 0x1c00ffff
            devnet_max_bits = 0x207fffff
            
            if bits >= devnet_max_bits:
                print("❌ EASY DIFFICULTY - This explains nonce 0 mining!")
            elif bits >= mainnet_bits:
                print("⚠️  Medium difficulty")
            else:
                print("✅ Hard mainnet difficulty")
                
            print(f"Mainnet target: 0x{mainnet_bits:08x}")
            print(f"Current bits:   0x{bits:08x}")
            print(f"Devnet max:     0x{devnet_max_bits:08x}")
    else:
        print("No tip found")
        
    db.close()
    
except ImportError:
    print("rocksdb Python module not available")
except Exception as e:
    print(f"Error: {e}")