# BitQuan TypeScript SDK (@bitquan/sdk)

Official TypeScript/JavaScript SDK for interacting with BitQuan blockchain.

## Features

- 🔐 Post-quantum cryptography (Dilithium signatures)
- 🌐 JSON-RPC 2.0 client  
- 📦 Address encoding/decoding (Bech32m)
- 🏗️ Transaction builder with PSBT support
- ✅ Full TypeScript support

## Status

🚧 **Work in Progress** - API is subject to change

### Implemented
- ✅ Address encode/decode (Bech32m with `q1` prefix)
- ✅ Basic RPC client structure
- ⏳ Transaction serialization (in progress)

### Planned
- [ ] Dilithium signature verification (browser-compatible)
- [ ] HD wallet support
- [ ] PSBT (Post-Quantum Signed Bitcoin Transaction) format
- [ ] Hardware wallet integration

## Installation

```bash
npm install @bitquan/sdk
# or
yarn add @bitquan/sdk
```

## Quick Start

```typescript
import { BitQuanClient, Address } from '@bitquan/sdk';

// Connect to node
const client = new BitQuanClient('http://localhost:8332');

// Get blockchain info
const info = await client.getBlockchainInfo();
console.log(`Height: ${info.blocks}`);

// Address utilities
const address = Address.encode(publicKeyHash);
const isValid = Address.validate('q1...');
```

## API Preview

### RPC Client

```typescript
const client = new BitQuanClient(url, options);

// Blockchain queries
await client.getBlockCount();
await client.getBlockchainInfo();
await client.getBlock(hash);

// Transactions
await client.getTransaction(txid);
await client.sendRawTransaction(hex);

// Mining
await client.getMiningInfo();
await client.getBlockTemplate();
```

### Address Utilities

```typescript
import { Address } from '@bitquan/sdk';

// Encode address (Bech32m)
const address = Address.encode(publicKeyHash);  // Returns: q1...

// Decode address
const pkh = Address.decode('q1...');

// Validate
const valid = Address.validate('q1...');
```

### Transaction Builder (Planned)

```typescript
import { TransactionBuilder } from '@bitquan/sdk';

const tx = new TransactionBuilder()
  .addInput({ txid, vout, value })
  .addOutput({ address, value })
  .build();
```

## Development

```bash
# Clone and setup
git clone https://github.com/bitquan/BitQuan
cd BitQuan/bindings/ts

# Install dependencies
npm install

# Build
npm run build

# Test
npm test

# Watch mode
npm run dev
```

## Project Structure

```
bindings/ts/
├── src/
│   ├── client/       # RPC client
│   ├── address/      # Address encoding
│   ├── tx/           # Transaction building
│   ├── crypto/       # PQC utilities
│   └── index.ts      # Main exports
├── tests/
│   └── *.test.ts     # Test files
└── package.json
```

## Building from Source

```bash
# In BitQuan root
cd bindings/ts

# Install and build
npm install
npm run build

# Run tests
npm test

# Link locally for testing
npm link
```

## Examples

See `examples/` directory:
- `basic-wallet.ts` - Create and use wallet
- `rpc-client.ts` - Query blockchain
- `tx-builder.ts` - Build transactions

## Compatibility

- Node.js 18+
- Modern browsers (ES2020+)
- TypeScript 5.0+

## Testing

```bash
# Unit tests
npm test

# E2E tests (requires running node)
npm run test:e2e

# Coverage
npm run test:coverage
```

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

Key areas needing help:
- [ ] Browser-compatible Dilithium verification
- [ ] PSBT format implementation
- [ ] Hardware wallet support
- [ ] Documentation and examples

## Security

⚠️ **Pre-alpha software** - Do not use for production!

- Cryptography review pending
- API may change significantly
- Test with testnet only

## Resources

- Main docs: [docs.bitquan.org](https://docs.bitquan.org)
- Rust implementation: `../../crates/`
- Specifications: `../../docs/spec/`
- Community: [Discord](https://discord.gg/bitquan)

## License

MIT - See [LICENSE](../../LICENSE)

---

**Roadmap**

- **Q1 2025**: Basic RPC client + address utilities ✅
- **Q2 2025**: Transaction building + PSBT
- **Q3 2025**: Hardware wallet support
- **Q4 2025**: Production-ready v1.0

Built with ❤️ by the BitQuan community
