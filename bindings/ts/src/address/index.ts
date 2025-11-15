/**
 * Address utilities for BitQuan with Bech32m encoding
 */

import * as bech32 from 'bech32';
import { createHash } from 'crypto-js';
import { ValidationResult, Network, AddressType } from './types';

/**
 * Address errors
 */
export class AddressError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'AddressError';
  }
}

/**
 * BitQuan address with Bech32m encoding
 */
export class Address {
  public readonly network: Network;
  public readonly addressType: AddressType;
  public readonly data: Buffer;
  public readonly address: string;

  constructor(network: Network, addressType: AddressType, data: Buffer) {
    this.network = network;
    this.addressType = addressType;
    this.data = data;
    this.address = this.encode();
  }

  /**
   * Create P2PKH address from public key hash
   */
  static p2pkh(network: Network, pubkeyHash: Buffer): Address {
    if (pubkeyHash.length !== 20) {
      throw new AddressError(`Invalid public key hash length: ${pubkeyHash.length}`);
    }
    return new Address(network, AddressType.P2PKH, pubkeyHash);
  }

  /**
   * Create post-quantum P2PKH address from Dilithium public key
   */
  static pqP2pkh(network: Network, dilithiumPubkey: Buffer): Address {
    if (dilithiumPubkey.length !== 1952) {
      throw new AddressError(`Invalid Dilithium public key length: ${dilithiumPubkey.length.length}`);
    }

    // Hash Dilithium public key
    const hash = createHash('sha256').update(dilithiumPubkey).digest();
    const pubkeyHash = createHash('ripemd160').update(hash).digest();

    return new Address(network, AddressType.PQPP2PKH, pubkeyHash);
  }

  /**
   * Create P2WPKH address from public key hash
   */
  static p2wpkh(network: Network, pubkeyHash: Buffer): Address {
    if (pubkeyHash.length !== 20) {
      throw new AddressError(`Invalid public key hash length: ${pubkeyHash.length}`);
    }
    return new Address(network, AddressType.P2WPKH, pubkeyHash);
  }

  /**
   * Parse address from string
   */
  static fromString(address: string): Address {
    try {
      const { hrp, version, data } = bech32.decode(address, bech32.Encoding.Bech32m);
      
      const network = Network.fromHRP(hrp);
      if (!network) {
        throw new AddressError(`Invalid network: ${hrp}`);
      }

      const addressType = AddressType.fromVersion(version);
      if (!addressType) {
        throw new AddressError(`Invalid address version: ${version}`);
      }

      return new Address(network, addressType, Buffer.from(data));
    } catch (error) {
      throw new AddressError(`Invalid address format: ${error.message}`);
    }
  }

  /**
   * Validate address for specific network
   */
  static validate(address: string, expectedNetwork: Network = Network.Mainnet): ValidationResult {
    try {
      const addr = Address.fromString(address);
      
      if (addr.network !== expectedNetwork) {
        return ValidationResult.WrongNetwork;
      }
      
      return ValidationResult.Valid;
    } catch (error) {
      if (error.message.includes('checksum')) {
        return ValidationResult.InvalidChecksum;
      }
      if (error.message.includes('version')) {
        return ValidationResult.InvalidVersion;
      }
      if (error.message.includes('network')) {
        return ValidationResult.WrongNetwork;
      }
      return ValidationResult.InvalidFormat(error.message);
    }
  }

  /**
   * Get public key hash for P2PKH/P2WPKH addresses
   */
  get pubkeyHash(): Buffer | null {
    return this.data.length === 20 ? this.data : null;
  }

  /**
   * Get script hash for P2WSH addresses
   */
  get scriptHash(): Buffer | null {
    return this.data.length === 32 ? this.data : null;
  }

  /**
   * Check if this is a post-quantum address
   */
  isPostQuantum(): boolean {
    return this.addressType.isPostQuantum();
  }

  /**
   * Encode address to Bech32m string
   */
  private encode(): string {
    const hrp = this.network.hrp();
    const data = [this.addressType.version, ...Array.from(this.data)];
    return bech32.encode(hrp, data, bech32.Encoding.Bech32m);
  }

  /**
   * Convert to string
   */
  toString(): string {
    return this.address;
  }

  /**
   * Get JSON representation
   */
  toJSON(): any {
    return {
      network: this.network,
      addressType: this.addressType,
      data: this.data.toString('hex'),
      address: this.address
    };
  }

  /**
   * Create from JSON
   */
  static fromJSON(json: any): Address {
    return new Address(
      json.network,
      json.addressType,
      Buffer.from(json.data, 'hex')
    );
  }

  /**
   * Compare addresses
   */
  equals(other: Address): boolean {
    return this.address === other.address;
  }
}

/**
 * Network utilities
 */
export class NetworkUtils {
  /**
   * Get network from human-readable part
   */
  static fromHRP(hrp: string): Network | null {
    switch (hrp) {
      case 'bq': return Network.Mainnet;
      case 'tbq': return Network.Testnet;
      case 'rbq': return Network.Regtest;
      default: return null;
    }
  }
}

// Export types
export type { AddressError };