import { 
  BitQuanClient, 
  Wallet, 
  TransactionBuilder, 
  Network, 
  validateAddress,
  BitQuanError 
} from '../src/index';

async function basicExample() {
  console.log('🚀 BitQuan SDK TypeScript Example');
  console.log('===================================');

  try {
    // 1. Create a new wallet
    console.log('\n📱 Creating new wallet...');
    const wallet = new Wallet(Network.TESTNET);
    console.log(`   Address: ${wallet.getAddress()}`);
    console.log(`   Private Key: ${wallet.getPrivateKey()}`);
    console.log(`   Public Key: ${wallet.getPublicKey()}`);

    // 2. Create blockchain client
    console.log('\n🌐 Connecting to blockchain...');
    const client = BitQuanClient.create(Network.TESTNET);
    console.log(`   RPC URL: ${client.getConfig().rpc_url}`);

    // 3. Get blockchain info
    console.log('\n📊 Getting blockchain info...');
    try {
      const info = await client.getBlockchainInfo();
      console.log(`   Chain: ${info.chain}`);
      console.log(`   Blocks: ${info.blocks}`);
      console.log(`   Best Block Hash: ${info.best_block_hash}`);
    } catch (error) {
      if (error instanceof BitQuanError) {
        console.log(`   ⚠️  Failed to get blockchain info: ${error.message}`);
        console.log('   (This is expected if the RPC endpoint is not available)');
      }
    }

    // 4. Get account information
    console.log('\n💰 Getting account info...');
    try {
      const account = await client.getAccount(wallet.getAddress());
      console.log(`   Address: ${account.address}`);
      console.log(`   Balance: ${account.balance}`);
      console.log(`   Nonce: ${account.nonce}`);
    } catch (error) {
      if (error instanceof BitQuanError) {
        console.log(`   ⚠️  Failed to get account info: ${error.message}`);
        console.log('   (This is expected for new addresses)');
      }
    }

    // 5. Build a transaction
    console.log('\n📝 Building transaction...');
    try {
      const tx = new TransactionBuilder()
        .addInput('previous_tx_hash', 0)
        .addOutput('tbq1recipient_address_example', 1000)
        .addOutput(wallet.getAddress(), 500) // Change output
        .build();

      console.log('   Transaction built successfully!');
      console.log(`   Hash: ${TransactionUtils.hash(tx)}`);
      console.log(`   Size: ${TransactionUtils.size(tx)} bytes`);
      console.log(`   Fee: ${TransactionUtils.fee(tx)} satoshis`);
      console.log(`   Outputs: ${tx.outputs.length}`);

      // Sign the transaction
      const signedTx = TransactionUtils.sign(tx, wallet.getPrivateKey());
      console.log('   ✅ Transaction signed successfully!');
    } catch (error) {
      if (error instanceof BitQuanError) {
        console.log(`   ❌ Failed to build transaction: ${error.message}`);
      }
    }

    // 6. Demonstrate cryptographic operations
    console.log('\n🔐 Cryptographic operations...');
    const message = 'Hello, BitQuan!';
    const signature = wallet.signMessage(message);
    console.log(`   Message: ${message}`);
    console.log(`   Signature: ${signature}`);
    console.log(`   Verified: ${wallet.verifyMessage(message, signature)}`);

    // 7. Address validation
    console.log('\n🔍 Address validation...');
    const testAddresses = [
      'bq1abcdef1234567890abcdef1234567890ab',
      'tbq1abcdef1234567890abcdef1234567890ab',
      'dbq1abcdef1234567890abcdef1234567890ab',
      'invalid_address'
    ];

    testAddresses.forEach(address => {
      const isValid = validateAddress(address);
      console.log(`   ${address}: ${isValid ? '✅ Valid' : '❌ Invalid'}`);
    });

    // 8. Export wallet
    console.log('\n💾 Exporting wallet...');
    const exportData = wallet.export();
    console.log(`   Export format: Address: ${exportData.address}, Network: ${exportData.network}`);

    // 9. Import wallet from export
    console.log('\n📥 Importing wallet...');
    const importedWallet = Wallet.fromExport(exportData);
    console.log(`   Imported Address: ${importedWallet.getAddress()}`);
    console.log(`   Addresses match: ${wallet.getAddress() === importedWallet.getAddress()}`);

    console.log('\n✅ Example completed successfully!');

  } catch (error) {
    console.error('❌ Error:', error);
  }
}

// Transaction utilities example
function transactionUtilsExample() {
  console.log('\n🔧 Transaction Utilities Example');
  console.log('==================================');

  try {
    const tx = new TransactionBuilder()
      .addInput('prev_tx_hash', 0)
      .addOutput('tbq1address1', 1000)
      .addOutput('tbq1address2', 2000)
      .build();

    console.log(`Transaction Hash: ${TransactionUtils.hash(tx)}`);
    console.log(`Transaction Size: ${TransactionUtils.size(tx)} bytes`);
    console.log(`Output Value: ${TransactionUtils.outputValue(tx)}`);
    console.log(`Is Valid: ${TransactionUtils.isValid(tx)}`);

    // Merkle Tree example
    const leaves = ['leaf1', 'leaf2', 'leaf3', 'leaf4'];
    const merkleTree = new MerkleTree(leaves);
    console.log(`Merkle Root: ${merkleTree.getRoot()}`);

    const proof = merkleTree.getProof(1);
    const isValid = MerkleTree.verifyProof(leaves[1], proof, merkleTree.getRoot());
    console.log(`Merkle Proof Valid: ${isValid}`);

  } catch (error) {
    console.error('❌ Error:', error);
  }
}

// Error handling example
async function errorHandlingExample() {
  console.log('\n⚠️  Error Handling Example');
  console.log('============================');

  // Test invalid address
  try {
    validateAddress('invalid_address');
  } catch (error) {
    if (error instanceof BitQuanError) {
      console.log(`Caught BitQuanError: ${error.code} - ${error.message}`);
    }
  }

  // Test invalid transaction
  try {
    new TransactionBuilder().build(); // No inputs or outputs
  } catch (error) {
    if (error instanceof BitQuanError) {
      console.log(`Caught BitQuanError: ${error.code} - ${error.message}`);
    }
  }

  // Test network error
  try {
    const client = new BitQuanClient({ rpc_url: 'invalid_url' });
    await client.getBlockchainInfo();
  } catch (error) {
    if (error instanceof BitQuanError) {
      console.log(`Caught BitQuanError: ${error.code} - ${error.message}`);
    }
  }
}

// Run all examples
async function runAllExamples() {
  await basicExample();
  transactionUtilsExample();
  await errorHandlingExample();
}

// Run if this file is executed directly
if (require.main === module) {
  runAllExamples().catch(console.error);
}

export { basicExample, transactionUtilsExample, errorHandlingExample };