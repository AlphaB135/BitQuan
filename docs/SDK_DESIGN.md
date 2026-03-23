# BitQuan SDK & Tooling Documentation

**Version:** 1.0
**Last Updated:** 2026-03-17

---

## Table of Contents

1. [Rust SDK (bq-sdk)](#1-rust-sdk-bq-sdk)
2. [TypeScript Bindings](#2-typescript-bindings)
3. [CLI Tools](#3-cli-tools)
4. [Integration Examples](#4-integration-examples)

---

## 1. Rust SDK (bq-sdk)

### 1.1 Overview

The `bq-sdk` crate provides a comprehensive Rust SDK for BitQuan blockchain development with post-quantum security.

```toml
# Cargo.toml
[dependencies]
bq-sdk = "0.1.0"
```

### 1.2 Key Generation (Dilithium5)

```rust
use bq_sdk::crypto::DilithiumKeyPair;
use bq_sdk::{Wallet, WalletConfig, Network};

// Generate a new Dilithium5 keypair
let keypair = DilithiumKeyPair::generate()?;

// Access public key (1,952 bytes)
let public_key = keypair.public_key();
println!("Public key: {} bytes", public_key.len()); // 1952

// Sign a message
let message = b"Hello, post-quantum world!";
let signature = keypair.sign(message)?;
println!("Signature: {} bytes", signature.len()); // 4595

// Verify signature
let valid = keypair.verify(message, &signature)?;
assert!(valid);
```

#### HD Wallet Key Generation

```rust
use bq_sdk::wallet::{SimpleWallet, WalletConfig, Mnemonic, DerivationPath};
use bq_sdk::Network;

// Generate new wallet with quantum-enhanced entropy
let config = WalletConfig::new(Network::Mainnet);
let wallet = SimpleWallet::generate(&config)?;

// Get the mnemonic (24 words)
let mnemonic = wallet.get_mnemonic().unwrap();
println!("Mnemonic: {}", mnemonic.as_string());

// Derive address at specific path
let path = DerivationPath::bq_standard(0, 0, 0);
let address = wallet.get_address(&path)?;
println!("Address: {}", address);

// Get public key for external use
let public_key = wallet.get_public_key(&path)?;
```

#### Restoring from Mnemonic

```rust
use bq_sdk::wallet::{SimpleWallet, WalletConfig, Mnemonic};

// Parse existing mnemonic
let mnemonic = Mnemonic::from_str(
    "abandon abandon abandon abandon abandon abandon abandon \
     abandon abandon abandon abandon abandon abandon abandon \
     abandon abandon abandon abandon abandon abandon abandon \
     abandon abandon abandon art",
    false // quantum_enhanced
)?;

// Restore wallet
let config = WalletConfig::desktop();
let wallet = SimpleWallet::from_mnemonic(&mnemonic, &config)?;

// Derive same addresses
let address = wallet.get_address(&DerivationPath::default())?;
```

### 1.3 Transaction Building

```rust
use bq_sdk::psbt::{PQPSBT, PSBTInput, PSBTOutput};
use bq_sdk::address::Address;
use bq_sdk::Network;

// Create PSBT builder
let mut builder = PQPSBT::builder()
    .network(Network::Mainnet)
    .version(1)
    .locktime(0);

// Add input (UTXO to spend)
let txid_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
let txid = hex::decode(txid_hex)?;
builder = builder.add_input(PSBTInput {
    txid: txid.try_into()?,
    vout: 0,
    sequence: 0xFFFFFFFF,
    amount: 10_000_000, // 0.1 BQ
})?;

// Add output (recipient)
let recipient = Address::parse("bq1qyqsq9q5z5khxv8y2w3...")?;
builder = builder.add_output(PSBTOutput {
    address: recipient.to_string(),
    amount: 9_900_000, // 0.099 BQ (0.001 BQ fee)
})?;

// Build unsigned PSBT
let mut psbt = builder.build()?;
println!("Unsigned PSBT created");
```

### 1.4 Signing Flow

```rust
use bq_sdk::wallet::{SimpleWallet, WalletConfig};
use bq_sdk::psbt::PQPSBT;

// Create or load wallet
let config = WalletConfig::desktop();
let mut wallet = SimpleWallet::generate(&config)?;

// Sign the PSBT
wallet.sign_psbt(&mut psbt)?;

// Check signatures
for (i, input) in psbt.inputs.iter().enumerate() {
    if let Some(sig) = input.get_dilithium_signature() {
        println!("Input {}: Signed ({} bytes)", i, sig.len());
    }
}

// Finalize and extract transaction
let tx = psbt.finalize()?;
println!("Transaction ready to broadcast");
```

#### Multi-Signature Signing

```rust
use bq_sdk::wallet::{MultisigWallet, MultisigConfig, PartialSignature};

// Create 2-of-3 multisig wallet
let config = MultisigConfig {
    required_sigs: 2,
    total_signers: 3,
    public_keys: vec![
        alice_pubkey.to_hex(),
        bob_pubkey.to_hex(),
        charlie_pubkey.to_hex(),
    ],
    label: Some("Treasury".to_string()),
    created_at: current_timestamp(),
};

let multisig = MultisigWallet::new(config)?;

// Create pending transaction
let pending = multisig.create_pending_tx(inputs, outputs)?;

// First signer (Alice)
let alice_sig = alice_wallet.sign_partial(&pending)?;
pending.add_signature(alice_sig)?;

// Second signer (Bob)
let bob_sig = bob_wallet.sign_partial(&pending)?;
pending.add_signature(bob_sig)?;

// Finalize with 2 signatures
let tx = multisig.finalize(pending)?;
```

### 1.5 Broadcasting to Network

```rust
use bq_sdk::rpc::RpcClient;
use bq_sdk::Network;

// Connect to node
let client = RpcClient::new("http://localhost:8332")?
    .with_auth("username", "password");

// Broadcast transaction
let txid = client.broadcast_transaction(&tx)?;
println!("Broadcast TXID: {}", txid);

// Check transaction status
let status = client.get_transaction_status(&txid)?;
println!("Confirmations: {}", status.confirmations);
```

### 1.6 Complete Example: Send Transaction

```rust
use bq_sdk::{
    Wallet, WalletConfig, Network,
    psbt::{PQPSBT, PSBTInput, PSBTOutput},
    rpc::RpcClient,
};
use bq_sdk::wallet::SimpleWallet;

fn send_transaction(
    recipient: &str,
    amount: u64,
    fee: u64,
) -> Result<String, Box<dyn std::error::Error>> {
    // 1. Setup wallet
    let config = WalletConfig::desktop();
    let mut wallet = SimpleWallet::generate(&config)?;

    // 2. Connect to node
    let client = RpcClient::new("http://localhost:8332")?
        .with_auth("user", "pass");

    // 3. Get UTXOs
    let utxos = client.list_unspent(&wallet.get_address(&DerivationPath::default())?)?;

    // 4. Select UTXO
    let utxo = utxos.iter()
        .find(|u| u.amount >= amount + fee)
        .ok_or("Insufficient funds")?;

    // 5. Build transaction
    let mut psbt = PQPSBT::builder()
        .network(Network::Mainnet)
        .add_input(PSBTInput {
            txid: utxo.txid,
            vout: utxo.vout,
            sequence: 0xFFFFFFFF,
            amount: utxo.amount,
        })?
        .add_output(PSBTOutput {
            address: recipient.to_string(),
            amount,
        })?
        // Change output
        .add_output(PSBTOutput {
            address: wallet.get_address(&DerivationPath::default())?.to_string(),
            amount: utxo.amount - amount - fee,
        })?
        .build()?;

    // 6. Sign
    wallet.sign_psbt(&mut psbt)?;

    // 7. Finalize
    let tx = psbt.finalize()?;

    // 8. Broadcast
    let txid = client.broadcast_transaction(&tx)?;

    Ok(txid)
}

fn main() {
    match send_transaction("bq1qyqsq9q5z5...", 1_000_000, 10_000) {
        Ok(txid) => println!("Sent! TXID: {}", txid),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

---

## 2. TypeScript Bindings

### 2.1 Setup

```bash
# Install package
npm install @bitquan/sdk

# Or with yarn
yarn add @bitquan/sdk
```

### 2.2 wasm-bindgen Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│                    TypeScript SDK Architecture                      │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│   TypeScript Application                                           │
│   ─────────────────────                                            │
│         │                                                          │
│         ▼                                                          │
│   @bitquan/sdk (TypeScript wrapper)                               │
│   ─────────────────────────────────                                │
│         │                                                          │
│         ▼                                                          │
│   wasm-bindgen (auto-generated)                                   │
│   ─────────────────────────                                        │
│         │                                                          │
│         ▼                                                          │
│   bq-sdk-wasm (WebAssembly)                                       │
│   ────────────────────────                                         │
│         │                                                          │
│         ▼                                                          │
│   Rust bq-sdk crate                                               │
│   ──────────────────                                               │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

### 2.3 API Surface Design

```typescript
// @bitquan/sdk/src/index.ts

export { Wallet } from './wallet';
export { Address } from './address';
export { Transaction, PSBT } from './transaction';
export { RpcClient } from './rpc';

export { Network, SignatureAlgorithm } from './types';

// Re-export errors
export { BQError, WalletError, PSBTErrors } from './errors';
```

```typescript
// @bitquan/sdk/src/wallet.ts

import { WasmWallet } from './wasm';

export interface WalletConfig {
  network: Network;
  signatureAlgorithm?: SignatureAlgorithm;
  quantumEntropy?: boolean;
}

export class Wallet {
  private inner: WasmWallet;

  private constructor(inner: WasmWallet) {
    this.inner = inner;
  }

  /**
   * Generate a new wallet with random mnemonic
   */
  static async generate(config: WalletConfig): Promise<Wallet> {
    const wasm = await loadWasm();
    const inner = wasm.Wallet.generate(config);
    return new Wallet(inner);
  }

  /**
   * Restore wallet from mnemonic phrase
   */
  static async fromMnemonic(
    mnemonic: string,
    config: WalletConfig
  ): Promise<Wallet> {
    const wasm = await loadWasm();
    const inner = wasm.Wallet.fromMnemonic(mnemonic, config);
    return new Wallet(inner);
  }

  /**
   * Get address at derivation path
   */
  async getAddress(path?: string): Promise<Address> {
    return this.inner.get_address(path || 'm');
  }

  /**
   * Get the mnemonic phrase (only available after generation)
   */
  getMnemonic(): string | null {
    return this.inner.get_mnemonic();
  }

  /**
   * Sign a PSBT
   */
  async signPsbt(psbt: PSBT): Promise<PSBT> {
    return this.inner.sign_psbt(psbt);
  }

  /**
   * Lock wallet (clear sensitive data from memory)
   */
  lock(): void {
    this.inner.lock();
  }

  /**
   * Check if wallet is locked
   */
  isLocked(): boolean {
    return this.inner.is_locked();
  }
}
```

### 2.4 Promise-Based Interface

```typescript
// @bitquan/sdk/src/transaction.ts

export class PSBT {
  private inner: WasmPSBT;

  private constructor(inner: WasmPSBT) {
    this.inner = inner;
  }

  /**
   * Create a new PSBT builder
   */
  static builder(): PSBTBuilder {
    return new PSBTBuilder();
  }

  /**
   * Parse PSBT from base64
   */
  static async fromBase64(data: string): Promise<PSBT> {
    const wasm = await loadWasm();
    return new PSBT(wasm.PSBT.from_base64(data));
  }

  /**
   * Serialize to base64
   */
  toBase64(): string {
    return this.inner.to_base64();
  }

  /**
   * Get transaction ID (after finalization)
   */
  getTxid(): string {
    return this.inner.txid();
  }

  /**
   * Calculate fee
   */
  getFee(): bigint {
    return this.inner.fee();
  }

  /**
   * Finalize and extract transaction
   */
  async finalize(): Promise<Transaction> {
    const tx = await this.inner.finalize();
    return new Transaction(tx);
  }
}

export class PSBTBuilder {
  private inputs: PSBTInput[] = [];
  private outputs: PSBTOutput[] = [];
  private network: Network = Network.Mainnet;

  setNetwork(network: Network): this {
    this.network = network;
    return this;
  }

  addInput(input: PSBTInput): this {
    this.inputs.push(input);
    return this;
  }

  addOutput(output: PSBTOutput): this {
    this.outputs.push(output);
    return this;
  }

  async build(): Promise<PSBT> {
    const wasm = await loadWasm();
    const builder = wasm.PSBT.builder();

    for (const input of this.inputs) {
      builder.add_input(input);
    }

    for (const output of this.outputs) {
      builder.add_output(output);
    }

    return new PSBT(builder.build());
  }
}

export interface PSBTInput {
  txid: string;
  vout: number;
  sequence?: number;
  amount: bigint;
}

export interface PSBTOutput {
  address: string;
  amount: bigint;
}
```

### 2.5 Example: Browser Wallet

```html
<!DOCTYPE html>
<html>
<head>
  <title>BitQuan Browser Wallet</title>
</head>
<body>
  <script type="module">
    import { Wallet, PSBT, RpcClient, Network } from '@bitquan/sdk';

    class BrowserWallet {
      constructor() {
        this.wallet = null;
        this.client = null;
      }

      async init() {
        // Connect to node
        this.client = new RpcClient('http://localhost:8332', {
          username: 'user',
          password: 'pass'
        });
      }

      async createWallet() {
        this.wallet = await Wallet.generate({
          network: Network.Mainnet,
          quantumEntropy: true
        });

        const mnemonic = this.wallet.getMnemonic();
        console.log('Save this mnemonic:', mnemonic);

        const address = await this.wallet.getAddress();
        console.log('Your address:', address.toString());

        return { mnemonic, address };
      }

      async restoreWallet(mnemonic) {
        this.wallet = await Wallet.fromMnemonic(mnemonic, {
          network: Network.Mainnet
        });

        return await this.wallet.getAddress();
      }

      async getBalance() {
        const address = await this.wallet.getAddress();
        return await this.client.getBalance(address.toString());
      }

      async send(recipient, amountBQ) {
        const amount = BigInt(amountBQ) * 1000000000000000000n; // BQ to qbits
        const fee = 1000000000000000n; // 0.001 BQ

        // Get UTXOs
        const address = await this.wallet.getAddress();
        const utxos = await this.client.listUnspent(address.toString());

        // Build transaction
        const psbt = await PSBT.builder()
          .setNetwork(Network.Mainnet)
          .addInput({
            txid: utxos[0].txid,
            vout: utxos[0].vout,
            amount: BigInt(utxos[0].amount)
          })
          .addOutput({
            address: recipient,
            amount: amount
          })
          .addOutput({
            address: address.toString(),
            amount: BigInt(utxos[0].amount) - amount - fee
          })
          .build();

        // Sign
        const signed = await this.wallet.signPsbt(psbt);

        // Finalize
        const tx = await signed.finalize();

        // Broadcast
        const txid = await this.client.broadcast(tx);
        return txid;
      }
    }

    // Usage
    const wallet = new BrowserWallet();
    await wallet.init();

    document.getElementById('create-btn').onclick = async () => {
      const result = await wallet.createWallet();
      document.getElementById('mnemonic').textContent = result.mnemonic;
      document.getElementById('address').textContent = result.address.toString();
    };
  </script>

  <button id="create-btn">Create Wallet</button>
  <p>Mnemonic: <span id="mnemonic"></span></p>
  <p>Address: <span id="address"></span></p>
</body>
</html>
```

### 2.6 React Integration

```tsx
// hooks/useWallet.ts
import { useState, useEffect } from 'react';
import { Wallet, Network } from '@bitquan/sdk';

export function useWallet() {
  const [wallet, setWallet] = useState<Wallet | null>(null);
  const [address, setAddress] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const generate = async () => {
    setLoading(true);
    setError(null);
    try {
      const w = await Wallet.generate({
        network: Network.Mainnet,
        quantumEntropy: true
      });
      setWallet(w);
      const addr = await w.getAddress();
      setAddress(addr.toString());
    } catch (e) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  const restore = async (mnemonic: string) => {
    setLoading(true);
    setError(null);
    try {
      const w = await Wallet.fromMnemonic(mnemonic, {
        network: Network.Mainnet
      });
      setWallet(w);
      const addr = await w.getAddress();
      setAddress(addr.toString());
    } catch (e) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  return { wallet, address, loading, error, generate, restore };
}

// components/WalletComponent.tsx
import { useWallet } from '../hooks/useWallet';

export function WalletComponent() {
  const { address, loading, error, generate, restore } = useWallet();
  const [mnemonic, setMnemonic] = useState('');

  return (
    <div>
      <h2>BitQuan Wallet</h2>

      {error && <div className="error">{error}</div>}

      <p>Address: {address || 'No wallet'}</p>

      <button onClick={generate} disabled={loading}>
        {loading ? 'Creating...' : 'Create New Wallet'}
      </button>

      <div>
        <input
          placeholder="Enter mnemonic"
          value={mnemonic}
          onChange={(e) => setMnemonic(e.target.value)}
        />
        <button onClick={() => restore(mnemonic)} disabled={loading}>
          Restore
        </button>
      </div>
    </div>
  );
}
```

---

## 3. CLI Tools

### 3.1 bitquan-cli Commands

#### Installation

```bash
# Build from source
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan
cargo build --release

# Binary location
./target/release/bitquan-node --help
```

#### Wallet Commands

```bash
# Generate new wallet
bitquan-node wallet-gen \
  --network mainnet \
  --output wallet.keystore

# Display wallet info
bitquan-node wallet-info \
  --keystore wallet.keystore

# List addresses
bitquan-node wallet-addresses \
  --keystore wallet.keystore \
  --count 10

# Get balance
bitquan-node wallet-balance \
  --keystore wallet.keystore \
  --node http://localhost:8332

# Backup wallet
bitquan-node wallet-backup \
  --keystore wallet.keystore \
  --output wallet_backup.json
```

#### Transaction Commands

```bash
# Create transaction
bitquan-node tx-create \
  --from bq1qyqsq9q5z5khxv8y2w3... \
  --to bq1qxyzabc123... \
  --amount 1.0 \
  --fee 0.001 \
  --output tx.psbt

# Sign transaction
bitquan-node tx-sign \
  --input tx.psbt \
  --keystore wallet.keystore \
  --output tx_signed.psbt

# Broadcast transaction
bitquan-node tx-send \
  --input tx_signed.psbt \
  --node http://localhost:8332

# Decode transaction
bitquan-node tx-decode \
  --input tx.hex

# Inspect PSBT
bitquan-node psbt-inspect \
  --input tx.psbt
```

#### Network Diagnostics

```bash
# Get blockchain info
bitquan-node getblockchaininfo \
  --node http://localhost:8332

# Get network info
bitquan-node getnetworkinfo \
  --node http://localhost:8332

# Get peer info
bitquan-node getpeerinfo \
  --node http://localhost:8332

# Get mempool info
bitquan-node getmempoolinfo \
  --node http://localhost:8332

# Ping node
bitquan-node ping \
  --node http://localhost:8332

# Get block by hash/height
bitquan-node getblock \
  --node http://localhost:8332 \
  --hash 00000000000000000001...

# Get transaction
bitquan-node gettx \
  --node http://localhost:8332 \
  --txid abc123...
```

#### Mining Commands

```bash
# Mine genesis block
bitquan-node mine-genesis \
  --output genesis.json \
  --max-tries 100000000

# Mine single block
bitquan-node mine-once \
  --network mainnet \
  --pow hashcash \
  --payout-script 76a914... \
  --bits 0x1c00ffff

# Continuous mining
bitquan-node mine \
  --datadir ./data/chainstate \
  --network mainnet \
  --pow hashcash \
  --threads 4 \
  --payout-script 76a914...
```

### 3.2 Command Reference

| Command | Description |
|---------|-------------|
| `run` | Start node |
| `wallet-gen` | Generate new wallet |
| `wallet-info` | Display wallet info |
| `wallet-addresses` | List addresses |
| `wallet-balance` | Get balance |
| `wallet-backup` | Backup wallet |
| `wallet-restore` | Restore from mnemonic |
| `tx-create` | Create transaction |
| `tx-sign` | Sign transaction |
| `tx-send` | Broadcast transaction |
| `tx-decode` | Decode transaction |
| `psbt-inspect` | Inspect PSBT |
| `getblockchaininfo` | Blockchain status |
| `getnetworkinfo` | Network status |
| `getpeerinfo` | Connected peers |
| `getmempoolinfo` | Mempool status |
| `getblock` | Get block |
| `gettx` | Get transaction |
| `mine-genesis` | Mine genesis |
| `mine-once` | Mine single block |
| `mine` | Continuous mining |

### 3.3 Configuration File

```toml
# config/bitquan.toml

[network]
id = "mainnet"
p2p_port = 8333
max_peers = 125

[rpc]
enabled = true
bind = "127.0.0.1:8332"
username = "admin"
password = "secure_password_here"
require_auth = true

[storage]
data_dir = "./data/chainstate"
cache_size_mb = 256

[mining]
enabled = false
threads = 4

[logging]
level = "info"
file = "./logs/bitquan.log"
```

---

## 4. Integration Examples

### 4.1 Exchange Integration

```rust
// exchange/integration.rs

use bq_sdk::{
    Wallet, WalletConfig, Network,
    rpc::RpcClient,
    psbt::{PQPSBT, PSBTInput, PSBTOutput},
};
use std::collections::HashMap;

pub struct BitQuanIntegration {
    client: RpcClient,
    hot_wallet: SimpleWallet,
    deposit_addresses: HashMap<String, String>,
}

impl BitQuanIntegration {
    pub async fn new(node_url: &str, keystore_path: &str) -> Result<Self, Error> {
        let client = RpcClient::new(node_url)?
            .with_auth("exchange", "password");

        let keystore = std::fs::read(keystore_path)?;
        let hot_wallet = SimpleWallet::from_keystore(&keystore)?;

        Ok(Self {
            client,
            hot_wallet,
            deposit_addresses: HashMap::new(),
        })
    }

    /// Generate deposit address for user
    pub async fn generate_deposit_address(&mut self, user_id: &str) -> Result<String, Error> {
        let path = DerivationPath::bq_standard(0, 0, self.deposit_addresses.len() as u32);
        let address = self.hot_wallet.get_address(&path)?;

        self.deposit_addresses.insert(user_id.to_string(), address.to_string());

        Ok(address.to_string())
    }

    /// Check deposits for all users
    pub async fn check_deposits(&self) -> Result<Vec<Deposit>, Error> {
        let mut deposits = Vec::new();

        for (user_id, address) in &self.deposit_addresses {
            let txs = self.client.list_transactions_by_address(address, 100)?;

            for tx in txs {
                if tx.confirmations >= 12 { // 12 confirmations required
                    deposits.push(Deposit {
                        user_id: user_id.clone(),
                        txid: tx.txid,
                        amount: tx.amount,
                        confirmations: tx.confirmations,
                    });
                }
            }
        }

        Ok(deposits)
    }

    /// Process withdrawal
    pub async fn withdraw(
        &mut self,
        to_address: &str,
        amount: u64,
        fee: u64,
    ) -> Result<String, Error> {
        // Get UTXOs
        let utxos = self.client.list_unspent(&self.hot_wallet.get_address(&DerivationPath::default())?)?;

        // Select UTXOs
        let selected = self.select_utxos(&utxos, amount + fee)?;

        // Build transaction
        let mut builder = PQPSBT::builder()
            .network(Network::Mainnet);

        let mut input_total = 0;
        for utxo in &selected {
            builder = builder.add_input(PSBTInput {
                txid: utxo.txid,
                vout: utxo.vout,
                sequence: 0xFFFFFFFF,
                amount: utxo.amount,
            })?;
            input_total += utxo.amount;
        }

        // Add withdrawal output
        builder = builder.add_output(PSBTOutput {
            address: to_address.to_string(),
            amount,
        })?;

        // Add change output
        let change_address = self.hot_wallet.get_address(&DerivationPath::default())?;
        builder = builder.add_output(PSBTOutput {
            address: change_address.to_string(),
            amount: input_total - amount - fee,
        })?;

        let mut psbt = builder.build()?;

        // Sign
        self.hot_wallet.sign_psbt(&mut psbt)?;

        // Finalize and broadcast
        let tx = psbt.finalize()?;
        let txid = self.client.broadcast_transaction(&tx)?;

        Ok(txid)
    }

    fn select_utxos(&self, utxos: &[UTXO], needed: u64) -> Result<Vec<UTXO>, Error> {
        let mut selected = Vec::new();
        let mut total = 0;

        for utxo in utxos {
            selected.push(utxo.clone());
            total += utxo.amount;

            if total >= needed {
                return Ok(selected);
            }
        }

        Err(Error::InsufficientFunds)
    }
}

pub struct Deposit {
    pub user_id: String,
    pub txid: String,
    pub amount: u64,
    pub confirmations: u32,
}
```

### 4.2 Merchant Payment Flow

```typescript
// merchant/payment.ts

import { Address, RpcClient, PSBT, Network } from '@bitquan/sdk';

export interface PaymentRequest {
  invoiceId: string;
  amount: bigint;
  address: string;
  expiresAt: Date;
  status: 'pending' | 'paid' | 'expired';
}

export class MerchantPayment {
  private client: RpcClient;
  private payments: Map<string, PaymentRequest> = new Map();

  constructor(nodeUrl: string) {
    this.client = new RpcClient(nodeUrl);
  }

  async createInvoice(amountBQ: number, expiresInMinutes: number = 30): Promise<PaymentRequest> {
    const invoiceId = crypto.randomUUID();
    const address = await this.generatePaymentAddress(invoiceId);
    const amount = BigInt(amountBQ) * 1000000000000000000n;

    const payment: PaymentRequest = {
      invoiceId,
      amount,
      address: address.toString(),
      expiresAt: new Date(Date.now() + expiresInMinutes * 60 * 1000),
      status: 'pending'
    };

    this.payments.set(invoiceId, payment);
    return payment;
  }

  async checkPayment(invoiceId: string): Promise<PaymentRequest> {
    const payment = this.payments.get(invoiceId);
    if (!payment) throw new Error('Invoice not found');

    if (new Date() > payment.expiresAt) {
      payment.status = 'expired';
      return payment;
    }

    // Check for incoming transactions
    const txs = await this.client.listTransactionsByAddress(payment.address, 10);

    for (const tx of txs) {
      if (tx.amount >= payment.amount && tx.confirmations >= 1) {
        payment.status = 'paid';
        return payment;
      }
    }

    return payment;
  }

  private async generatePaymentAddress(invoiceId: string): Promise<Address> {
    // In production, derive from HD wallet
    // For demo, use a new address each time
    return Address.generate(Network.Mainnet);
  }
}

// Express.js endpoint
import express from 'express';

const app = express();
const payments = new MerchantPayment('http://localhost:8332');

app.post('/api/invoice', async (req, res) => {
  const { amount } = req.body;
  const invoice = await payments.createInvoice(amount);
  res.json(invoice);
});

app.get('/api/invoice/:id', async (req, res) => {
  const invoice = await payments.checkPayment(req.params.id);
  res.json(invoice);
});

app.listen(3000);
```

### 4.3 Hardware Wallet Interface

```rust
// hardware/interface.rs

use bq_sdk::hardware::{HardwareWallet, DeviceCapabilities};
use bq_sdk::psbt::PQPSBT;

pub struct HardwareWalletManager {
    device: Option<Box<dyn HardwareWallet>>,
}

impl HardwareWalletManager {
    pub fn new() -> Self {
        Self { device: None }
    }

    /// Connect to hardware wallet
    pub async fn connect(&mut self) -> Result<DeviceInfo, HardwareError> {
        // Detect connected devices
        let devices = detect_devices()?;

        if devices.is_empty() {
            return Err(HardwareError::NoDeviceFound);
        }

        // Connect to first device
        let device = connect_device(&devices[0])?;
        self.device = Some(device);

        // Get device info
        let info = self.device.as_ref().unwrap().get_info().await?;

        Ok(info)
    }

    /// Get address from device (requires user confirmation)
    pub async fn get_address(&self, path: &DerivationPath) -> Result<Address, HardwareError> {
        let device = self.device.as_ref()
            .ok_or(HardwareError::NotConnected)?;

        // This triggers display on device
        let address = device.get_address(path).await?;

        Ok(address)
    }

    /// Sign transaction on device (requires user confirmation)
    pub async fn sign_transaction(
        &self,
        psbt: &mut PQPSBT,
    ) -> Result<Vec<PartialSignature>, HardwareError> {
        let device = self.device.as_ref()
            .ok_or(HardwareError::NotConnected)?;

        // Check device supports Dilithium
        let caps = device.get_capabilities().await?;
        if !caps.dilithium5 {
            return Err(HardwareError::UnsupportedAlgorithm);
        }

        // Display transaction on device for confirmation
        // User must press button to confirm
        let signatures = device.sign_transaction(psbt).await?;

        Ok(signatures)
    }
}

// Example usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = HardwareWalletManager::new();

    // Connect
    let info = manager.connect().await?;
    println!("Connected: {} ({})", info.model, info.firmware_version);
    println!("Dilithium5 support: {}", info.capabilities.dilithium5);

    // Get address
    let address = manager.get_address(&DerivationPath::default()).await?;
    println!("Address: {}", address);

    // Sign transaction
    let mut psbt = create_unsigned_psbt()?;
    let signatures = manager.sign_transaction(&mut psbt).await?;

    println!("Signed {} inputs", signatures.len());

    Ok(())
}
```

---

## Appendix A: SDK Module Reference

| Module | Description |
|--------|-------------|
| `bq_sdk::wallet` | Wallet management, HD derivation |
| `bq_sdk::address` | Address encoding/decoding |
| `bq_sdk::psbt` | PQ-PSBT transaction building |
| `bq_sdk::crypto` | Dilithium5 cryptographic operations |
| `bq_sdk::hardware` | Hardware wallet integration |
| `bq_sdk::rpc` | Node RPC client |

## Appendix B: Error Types

```rust
pub enum SDKError {
    Address(AddressError),
    PSBT(PSBTError),
    Wallet(WalletError),
    Hardware(HardwareError),
    Crypto(String),
    Serialization(String),
    IO(std::io::Error),
}
```

## Appendix C: Feature Flags

| Feature | Description |
|---------|-------------|
| `default` | Core SDK |
| `hardware` | Hardware wallet support |
| `rpc` | RPC client |
| `wasm` | WebAssembly bindings |

---

*Last Updated: 2026-03-17*
*Author: BitQuan Core Team*
