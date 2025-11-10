# BitQuan Difficulty Configuration Fix

## Summary
Fixed the mining difficulty configuration to properly read from network-specific config files instead of using hardcoded values.

## Problem
- Mainnet was using testnet difficulty (0x1d00ffff instead of 0x1c00ffff)
- Mining function had hardcoded difficulty values
- Config file `difficulty_bits` values were not being read
- Each network did not have properly differentiated difficulty levels

## Solution

### 1. Added Config File Reader
Created `load_difficulty_from_config()` function in `crates/node/src/main.rs`:
- Reads `difficulty_bits` from network-specific config files
- Supports all networks: mainnet, testnet, devnet, regtest
- Provides fallback defaults if config file is missing

### 2. Updated Mining Logic
Modified `mine_continuous()` function:
- When `--bits=0` (default), loads difficulty from config file
- When `--bits` is specified, uses the override value
- Displays which difficulty value is being used at startup

### 3. Fixed Config Files
Updated `/config/devnet.toml`:
- Changed from `0x1d00ffff` to `0x207fffff` for easier development testing

## Difficulty Levels (Hardest to Easiest)

| Network  | Difficulty   | Description                    |
|----------|--------------|--------------------------------|
| Mainnet  | `0x1c00ffff` | Production (hardest)           |
| Testnet  | `0x1d00ffff` | Public testing (medium)        |
| Devnet   | `0x207fffff` | Development (easiest)          |
| Regtest  | `0x207fffff` | Regression testing (easiest)   |

## Usage

### Default behavior (reads from config):
```bash
# Mainnet - uses 0x1c00ffff from config/mainnet.toml
./bitquan-node mine --network mainnet --datadir ./data/mainnet

# Testnet - uses 0x1d00ffff from config/testnet.toml
./bitquan-node mine --network testnet --datadir ./data/testnet

# Devnet - uses 0x207fffff from config/devnet.toml
./bitquan-node mine --network devnet --datadir ./data/devnet
```

### Override with custom difficulty:
```bash
# Override with custom difficulty
./bitquan-node mine --network mainnet --bits 0x1d00ffff --datadir ./data/mainnet
```

## Files Modified

1. **crates/node/src/main.rs**
   - Added `load_difficulty_from_config()` function (lines 122-156)
   - Updated `mine_continuous()` to use config loader (lines 1432-1438)
   - Removed hardcoded difficulty values

2. **config/devnet.toml**
   - Updated `difficulty_bits` from `"0x1d00ffff"` to `"0x207fffff"`

3. **config/mainnet.toml**
   - Verified correct value: `"0x1c00ffff"` (no change needed)

4. **config/testnet.toml**
   - Verified correct value: `"0x1d00ffff"` (no change needed)

## Testing

Build and test:
```bash
# Build the node
cargo build --release --bin bitquan-node

# Verify config files
grep difficulty_bits config/*.toml

# Test mining (will show loaded difficulty)
./bitquan-node mine --network mainnet --datadir /tmp/test-mainnet --limit-blocks 1
```

## Benefits

1. **Network Isolation**: Each network has its proper difficulty level
2. **Configuration Flexibility**: Can adjust difficulty via config files without code changes
3. **Transparency**: Mining startup shows which difficulty is being used
4. **Maintainability**: No hardcoded values in mining logic
5. **Override Support**: CLI `--bits` parameter still works for testing

## Technical Details

### Config File Format
```toml
[network]
difficulty_bits = "0x1c00ffff"
```

### Fallback Behavior
If config file is missing or malformed, uses these defaults:
- Mainnet: `0x1c00ffff`
- Testnet: `0x1d00ffff`
- Devnet: `0x207fffff`
- Regtest: `0x207fffff`

### Difficulty Calculation
- Lower hex value = harder difficulty (more leading zeros required)
- `0x1c00ffff` requires more work than `0x1d00ffff`
- `0x207fffff` is the easiest for development/testing

## Verification

All configuration files verified:
```
✓ Mainnet:  0x1c00ffff (hardest - production ready)
✓ Testnet:  0x1d00ffff (medium - public testing)
✓ Devnet:   0x207fffff (easiest - development)
```

Build successful:
```
✓ Compiled bitquan-node v0.1.0
✓ No compilation errors
✓ All tests pass
```
