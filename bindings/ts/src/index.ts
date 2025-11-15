/**
 * BitQuan SDK - Main exports
 * 
 * Comprehensive TypeScript SDK for BitQuan blockchain with post-quantum security
 */

// Address utilities
export { Address, AddressType, Network, ValidationResult } from './address';
export type { AddressError } from './address';

// PSBT (Post-Quantum Partially Signed Bitcoin Transactions)
export { PQPSBT, PQPSBTBuilder, PSBTInput, PSBTOutput, SignatureFlags } from './psbt';
export type { PSBTError, InputKey, OutputKey, GlobalKey } from './psbt';

// Wallet functionality
export { Wallet, HDWallet, Mnemonic, DerivationPath, WalletConfig } from './wallet';
export type { WalletError, SignatureAlgorithm, DerivationConfig, SecurityConfig, PerformanceConfig } from './wallet';

// Cryptographic utilities
export { Dilithium, QuantumEntropy, ConstantTime } from './crypto';
export type { CryptoError, DilithiumKeyPair } from './crypto';

// Hardware wallet support
export { HardwareWallet, HardwareWalletManager, DeviceCapabilities } from './hardware';
export type { HardwareError, DeviceInfo, Command, ResponseStatus } from './hardware';

// RPC Client
export { BitQuanClient } from './client';
export type { RPCResponse, BlockchainInfo, TransactionInfo, UTXO } from './client';

// Utilities
export { Utils } from './utils';
export type { SighashType, ScriptType } from './utils';

// Version
export const VERSION = '0.1.0';

/**
 * BitQuan SDK main class
 */
export class BitQuanSDK {
  static version = VERSION;
  
  /**
   * Create a new wallet instance
   */
  static createWallet(config?: Partial<WalletConfig>): Wallet {
    return new HDWallet({
      network: Network.Mainnet,
      signatureAlgorithms: [SignatureAlgorithm.Dilithium3],
      ...config
    });
  }
  
  /**
   * Generate mnemonic phrase
   */
  static generateMnemonic(entropyBits: number = 256, quantumEnhanced: boolean = true): Mnemonic {
    return Mnemonic.generate(entropyBits, quantumEnhanced);
  }
  
  /**
   * Create RPC client
   */
  static createClient(url: string, options?: any): BitQuanClient {
    return new BitQuanClient(url, options);
  }
  
  /**
   * Validate address
   */
  static validateAddress(address: string, network?: Network): ValidationResult {
    return Address.validate(address, network || Network.Mainnet);
  }
  
  /**
   * Create PSBT builder
   */
  static createPSBT(): PQPSBTBuilder {
    return new PQPSBTBuilder();
  }
}

// Re-export enums for convenience
export { Network, AddressType, SignatureAlgorithm, ValidationResult } from './types';