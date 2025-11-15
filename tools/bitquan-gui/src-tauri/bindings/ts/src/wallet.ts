import { Network, WalletExport, KeyPair } from './types';
import { BitQuanError } from './errors';

export class Wallet {
  private privateKey: string;
  private publicKey: string;
  private address: string;
  private network: Network;

  constructor(network: Network = Network.MAINNET) {
    this.network = network;
    const keyPair = this.generateKeyPair();
    this.privateKey = keyPair.private_key;
    this.publicKey = keyPair.public_key;
    this.address = keyPair.address;
  }

  static fromPrivateKey(privateKey: string, network: Network = Network.MAINNET): Wallet {
    const wallet = new Wallet(network);
    wallet.privateKey = privateKey;
    wallet.publicKey = wallet.derivePublicKey(privateKey);
    wallet.address = wallet.deriveAddress(wallet.publicKey, network);
    return wallet;
  }

  static fromExport(exportData: WalletExport): Wallet {
    const wallet = Wallet.fromPrivateKey(exportData.private_key, exportData.network);
    return wallet;
  }

  getAddress(): string {
    return this.address;
  }

  getPrivateKey(): string {
    return this.privateKey;
  }

  getPublicKey(): string {
    return this.publicKey;
  }

  getNetwork(): Network {
    return this.network;
  }

  signMessage(message: string): string {
    const signature = this.sign(message, this.privateKey);
    return signature;
  }

  verifyMessage(message: string, signature: string): boolean {
    return this.verify(this.publicKey, message, signature);
  }

  export(): WalletExport {
    return {
      address: this.address,
      private_key: this.privateKey,
      public_key: this.publicKey,
      network: this.network
    };
  }

  private generateKeyPair(): KeyPair {
    const seed = this.generateSeed();
    return this.generateKeyPairFromSeed(seed);
  }

  private generateSeed(): string {
    const array = new Uint8Array(32);
    crypto.getRandomValues(array);
    return Array.from(array, byte => byte.toString(16).padStart(2, '0')).join('');
  }

  private generateKeyPairFromSeed(seed: string): KeyPair {
    const privateKey = this.derivePrivateKey(seed);
    const publicKey = this.derivePublicKey(privateKey);
    const address = this.deriveAddress(publicKey, this.network);

    return {
      private_key: privateKey,
      public_key: publicKey,
      address
    };
  }

  private derivePrivateKey(seed: string): string {
    // Simplified key derivation - in production, use proper cryptographic functions
    return seed.substring(0, 64);
  }

  private derivePublicKey(privateKey: string): string {
    // Simplified public key derivation - in production, use proper elliptic curve cryptography
    return 'pub_' + privateKey.substring(0, 16);
  }

  private deriveAddress(publicKey: string, network: Network): string {
    // Simplified address derivation
    const hash = this.hash(publicKey);
    const prefix = network === Network.MAINNET ? 'bq' : 
                   network === Network.TESTNET ? 'tbq' : 'dbq';
    return `${prefix}1${hash.substring(0, 32)}`;
  }

  private hash(data: string): string {
    // Simplified hashing - in production, use proper cryptographic hash functions
    let hash = 0;
    for (let i = 0; i < data.length; i++) {
      const char = data.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return Math.abs(hash).toString(16);
  }

  private sign(message: string, privateKey: string): string {
    // Simplified signing - in production, use proper digital signatures
    const hash = this.hash(message);
    return `sig_${privateKey.substring(0, 8)}_${hash.substring(0, 16)}`;
  }

  private verify(publicKey: string, message: string, signature: string): boolean {
    // Simplified verification - in production, use proper signature verification
    const hash = this.hash(message);
    return signature.includes(hash.substring(0, 16)) && signature.includes('sig_');
  }
}

export function validateAddress(address: string): boolean {
  if (address.length !== 35) return false;
  
  const prefix = address.substring(0, 3);
  return prefix === 'bq1' || prefix === 'tbq1' || prefix === 'dbq1';
}

export function extractNetwork(address: string): Network | null {
  if (address.startsWith('bq1')) return Network.MAINNET;
  if (address.startsWith('tbq1')) return Network.TESTNET;
  if (address.startsWith('dbq1')) return Network.DEVNET;
  return null;
}

export function convertNetwork(address: string, targetNetwork: Network): string {
  if (!validateAddress(address)) {
    throw new BitQuanError('INVALID_ADDRESS', `Invalid address: ${address}`);
  }

  const currentNetwork = extractNetwork(address);
  if (currentNetwork === targetNetwork) {
    return address;
  }

  const suffix = address.substring(3);
  const prefix = targetNetwork === Network.MAINNET ? 'bq1' : 
                 targetNetwork === Network.TESTNET ? 'tbq1' : 'dbq1';
  
  return `${prefix}${suffix}`;
}