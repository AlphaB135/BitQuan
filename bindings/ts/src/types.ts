/**
 * Types and enums for BitQuan SDK
 */

/**
 * Supported networks
 */
export enum Network {
  Mainnet = 'mainnet',
  Testnet = 'testnet',
  Regtest = 'regtest'
}

export namespace Network {
  /**
   * Get the human-readable part for Bech32m encoding
   */
  export function hrp(network: Network): string {
    switch (network) {
      case Network.Mainnet: return 'bq';
      case Network.Testnet: return 'tbq';
      case Network.Regtest: return 'rbq';
      default: throw new Error(`Unknown network: ${network}`);
    }
  }

  /**
   * Get network from human-readable part
   */
  export function fromHRP(hrp: string): Network | null {
    switch (hrp) {
      case 'bq': return Network.Mainnet;
      case 'tbq': return Network.Testnet;
      case 'rbq': return Network.Regtest;
      default: return null;
    }
  }
}

/**
 * Address types supported by BitQuan
 */
export enum AddressType {
  P2PKH = 0x00,
  P2SH = 0x01,
  P2WPKH = 0x02,
  P2WSH = 0x03,
  PQPP2PKH = 0x10,
  PQP2WSH = 0x11
}

export namespace AddressType {
  /**
   * Get version byte for this address type
   */
  export function version(type: AddressType): number {
    return type;
  }

  /**
   * Check if this is a post-quantum address type
   */
  export function isPostQuantum(type: AddressType): boolean {
    return type === AddressType.PQPP2PKH || type === AddressType.PQP2WSH;
  }

  /**
   * Get expected data length for this address type
   */
  export function dataLength(type: AddressType): number {
    switch (type) {
      case AddressType.P2PKH:
      case AddressType.P2SH:
      case AddressType.P2WPKH:
      case AddressType.PQPP2PKH:
        return 20;
      case AddressType.P2WSH:
      case AddressType.PQP2WSH:
        return 32;
      default:
        throw new Error(`Unknown address type: ${type}`);
    }
  }

  /**
   * Get address type from version
   */
  export function fromVersion(version: number): AddressType | null {
    for (const type of Object.values(AddressType)) {
      if (typeof type === 'number' && type === version) {
        return type as AddressType;
      }
    }
    return null;
  }
}

/**
 * Address validation result
 */
export enum ValidationResult {
  Valid = 'valid',
  InvalidFormat = 'invalid_format',
  WrongNetwork = 'wrong_network',
  InvalidVersion = 'invalid_version',
  InvalidChecksum = 'invalid_checksum',
  InvalidLength = 'invalid_length'
}

/**
 * Signature algorithms supported by the wallet
 */
export enum SignatureAlgorithm {
  ECDSA = 'ecdsa',
  Dilithium3 = 'dilithium3',
  Hybrid = 'hybrid'
}

export namespace SignatureAlgorithm {
  /**
   * Check if this is a post-quantum algorithm
   */
  export function isPostQuantum(algorithm: SignatureAlgorithm): boolean {
    return algorithm === SignatureAlgorithm.Dilithium3 || algorithm === SignatureAlgorithm.Hybrid;
  }
}

/**
 * Derivation path for HD wallets
 */
export class DerivationPath {
  public readonly path: number[];
  public readonly hardened: boolean[];

  constructor(path: number[] = [], hardened: boolean[] = []) {
    this.path = path;
    this.hardened = hardened;
  }

  /**
   * Create new derivation path
   */
  static new(): DerivationPath {
    return new DerivationPath();
  }

  /**
   * Add component to path
   */
  push(index: number, hardened: boolean): DerivationPath {
    return new DerivationPath(
      [...this.path, index],
      [...this.hardened, hardened]
    );
  }

  /**
   * Get BIP32 standard path for account
   */
  static bip44Standard(account: number, change: number, addressIndex: number): DerivationPath {
    return DerivationPath.new()
      .push(44, true)   // purpose
      .push(0, true)    // coin_type (Bitcoin)
      .push(account, true) // account
      .push(change, false) // change
      .push(addressIndex, false); // address_index
  }

  /**
   * Get BIP84 standard path (native SegWit)
   */
  static bip84Standard(account: number, change: number, addressIndex: number): DerivationPath {
    return DerivationPath.new()
      .push(84, true)   // purpose
      .push(0, true)    // coin_type (Bitcoin)
      .push(account, true) // account
      .push(change, false) // change
      .push(addressIndex, false); // address_index
  }

  /**
   * Get BitQuan post-quantum path
   */
  static bqStandard(account: number, change: number, addressIndex: number): DerivationPath {
    return DerivationPath.new()
      .push(123, true)  // BitQuan purpose
      .push(0, true)    // coin_type
      .push(account, true) // account
      .push(change, false) // change
      .push(addressIndex, false); // address_index
  }

  /**
   * Convert to string representation
   */
  toString(): string {
    if (this.path.length === 0) {
      return 'm';
    }

    let result = 'm';
    for (let i = 0; i < this.path.length; i++) {
      result += '/' + this.path[i];
      if (this.hardened[i]) {
        result += "'";
      }
    }
    return result;
  }

  /**
   * Parse from string representation
   */
  static fromString(path: string): DerivationPath {
    if (!path.startsWith('m')) {
      throw new Error("Path must start with 'm'");
    }

    const parts = path.split('/');
    const pathArray: number[] = [];
    const hardenedArray: boolean[] = [];

    for (let i = 1; i < parts.length; i++) {
      const part = parts[i];
      if (part === '') continue;

      const hardened = part.endsWith('\'');
      const indexStr = hardened ? part.slice(0, -1) : part;
      const index = parseInt(indexStr, 10);

      if (isNaN(index)) {
        throw new Error(`Invalid index: ${indexStr}`);
      }

      pathArray.push(index);
      hardenedArray.push(hardened);
    }

    return new DerivationPath(pathArray, hardenedArray);
  }

  /**
   * Compare derivation paths
   */
  equals(other: DerivationPath): boolean {
    if (this.path.length !== other.path.length) {
      return false;
    }

    for (let i = 0; i < this.path.length; i++) {
      if (this.path[i] !== other.path[i] || this.hardened[i] !== other.hardened[i]) {
        return false;
      }
    }

    return true;
  }
}

/**
 * Wallet configuration interfaces
 */
export interface WalletConfig {
  network: Network;
  signatureAlgorithms: SignatureAlgorithm[];
  derivation: DerivationConfig;
  security: SecurityConfig;
  performance: PerformanceConfig;
}

export interface DerivationConfig {
  bip32Standard: boolean;
  customPath?: DerivationPath;
  gapLimit: number;
}

export interface SecurityConfig {
  hybridSignatures: boolean;
  memoryLocking: boolean;
  cacheTimeout?: number; // Duration in ms
  quantumEntropy: boolean;
}

export interface PerformanceConfig {
  enableCache: boolean;
  maxCacheEntries: number;
  pregenerateAddresses: number;
}

/**
 * PSBT related types
 */
export interface PSBTInput {
  previousTxid: Buffer;
  previousOutputIndex: number;
  sequence: number;
  scriptSig: Buffer;
  dilithiumPublicKey?: Buffer;
  dilithiumSignature?: Buffer;
  ecdsaSignature?: Buffer;
}

export interface PSBTOutput {
  amount: number;
  scriptPubkey: Buffer;
}

export interface PSBTGlobal {
  version: number;
  fallbackFingerprint?: Buffer;
  locktime?: number;
}

/**
 * Transaction types
 */
export interface TransactionInput {
  txid: string;
  vout: number;
  value?: number;
  scriptSig?: string;
  sequence?: number;
}

export interface TransactionOutput {
  value: number;
  scriptPubkey: string;
  address?: string;
}

export interface Transaction {
  version: number;
  inputs: TransactionInput[];
  outputs: TransactionOutput[];
  locktime: number;
  sigAlgorithm: SignatureAlgorithm;
  witnesses?: string[][];
}

/**
 * Script types
 */
export enum ScriptType {
  P2PKH = 'p2pkh',
  P2SH = 'p2sh',
  P2WPKH = 'p2wpkh',
  P2WSH = 'p2wsh',
  PQPP2PKH = 'pq-p2pkh',
  PQP2WSH = 'pq-p2wsh'
}

/**
 * Sighash types
 */
export enum SighashType {
  ALL = 0x01,
  NONE = 0x02,
  SINGLE = 0x03,
  ANYONECANPAY = 0x80,
  ALL_ANYONECANPAY = 0x81,
  NONE_ANYONECANPAY = 0x82,
  SINGLE_ANYONECANPAY = 0x83
}

/**
 * RPC response types
 */
export interface RPCResponse<T = any> {
  result?: T;
  error?: {
    code: number;
    message: string;
  };
  id: string | number;
}

export interface BlockchainInfo {
  chain: string;
  blocks: number;
  headers: number;
  bestBlockHash: string;
  difficulty: number;
  medianTime: number;
  verificationProgress: number;
  chainWork: string;
  sizeOnDisk: number;
  pruned: boolean;
}

export interface TransactionInfo {
  txid: string;
  version: number;
  size: number;
  vsize: number;
  weight: number;
  locktime: number;
  vin: TransactionInput[];
  vout: TransactionOutput[];
  hex: string;
  hash: string;
  time: number;
  blocktime?: number;
  blockhash?: string;
  confirmations?: number;
}

export interface UTXO {
  txid: string;
  vout: number;
  value: number;
  scriptPubkey: string;
  address?: string;
  confirmations?: number;
}

/**
 * Error types
 */
export interface BitQuanError extends Error {
  code: string;
  details?: any;
}

/**
 * Utility types
 */
export type Buffer = Uint8Array;

/**
 * Event types
 */
export interface WalletEvent {
  type: 'address_generated' | 'transaction_signed' | 'wallet_locked' | 'wallet_unlocked';
  data?: any;
}

export interface HardwareWalletEvent {
  type: 'device_connected' | 'device_disconnected' | 'operation_completed' | 'error';
  deviceId?: string;
  data?: any;
}