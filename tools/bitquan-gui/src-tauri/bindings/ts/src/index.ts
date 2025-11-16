export * from './types';
export * from './client';
export * from './wallet';
export * from './transaction';
export * from './errors';

// Version
export const VERSION = '0.1.0';

// Default RPC URL
export const DEFAULT_RPC_URL = 'https://api.bitquan.network/rpc';

// Re-export main classes for convenience
export { BitQuanClient } from './client';
export { Wallet, validateAddress, extractNetwork, convertNetwork } from './wallet';
export { TransactionBuilder, TransactionUtils, MerkleTree } from './transaction';
export { BitQuanError } from './errors';