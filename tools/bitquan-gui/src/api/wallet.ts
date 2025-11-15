import { invoke } from '@tauri-apps/api/core';

export interface EncryptedKeystoreData {
  address: string;
  encrypted_data: string;
  created_at: number;
}

export interface WalletCreateRequest {
  password: string;
  address_hint?: string;
}

export interface WalletCreateResponse {
  success: boolean;
  keystore_data?: EncryptedKeystoreData;
  error?: string;
}

export interface WalletUnlockRequest {
  keystore_data: EncryptedKeystoreData;
  password: string;
}

export interface WalletUnlockResponse {
  success: boolean;
  address: string;
  error?: string;
}

export interface TransactionSignRequest {
  sighash_hex: string;
  password: string;
}

export interface TransactionSignResponse {
  success: boolean;
  signature_hex?: string;
  error?: string;
}

export interface WalletStatusResponse {
  is_locked: boolean;
  address?: string;
  cache_stats: {
    active_entries: number;
    total_entries: number;
    memory_usage_bytes: number;
  };
  key_info?: {
    algorithm: string;
    security_level: string;
    created_at: number;
  };
}

export interface RawTransactionRequest {
  tx_hex: string;
  max_fee_rate?: number;
}

export interface RawTransactionResponse {
  success: boolean;
  txid?: string;
  error?: string;
  fee?: number;
  size?: number;
}

export interface TransactionHistory {
  txid: string;
  type: 'sent' | 'received';
  amount: number;
  address: string;
  timestamp: number;
  confirmations: number;
  fee?: number;
  status: 'pending' | 'confirmed' | 'failed';
}

export interface WalletBalance {
  confirmed: number;
  unconfirmed: number;
  total: number;
  address: string;
}

export interface NetworkInfo {
  network: 'mainnet' | 'testnet' | 'devnet';
  block_height: number;
  difficulty: number;
  connections: number;
  version: string;
}

export interface BackupRequest {
  password: string;
  include_metadata?: boolean;
}

export interface BackupResponse {
  success: boolean;
  backup_data?: string;
  filename?: string;
  error?: string;
}

export interface RestoreRequest {
  backup_data: string;
  password: string;
}

export interface RestoreResponse {
  success: boolean;
  keystore_data?: EncryptedKeystoreData;
  error?: string;
}

// MUST MATCH Rust sighash.rs EXACTLY!
export const DOMAIN_SEPARATOR = "BitQuanSigHashV1";
export const MAGIC_BYTES = new Uint8Array([0x42, 0x51, 0x54, 0x58]); // "BQTX"
export const GENESIS_HASH = "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f";

// PERFECT PORT of Rust transaction_sighash logic
export async function calculateTransactionSighash(transaction: {
  version: number;
  inputs: Array<{
    prev_txid: string;
    prev_index: number;
    script_sig: string;
    sequence: number;
  }>;
  outputs: Array<{
    value: number;
    script_pubkey: string;
  }>;
  locktime: number;
}): Promise<string> {
  const data: Uint8Array[] = [];
  
  // 1. Write domain separator
  const domainEncoder = new TextEncoder();
  data.push(domainEncoder.encode(DOMAIN_SEPARATOR));
  
  // 2. Write magic bytes
  data.push(MAGIC_BYTES);
  
  // 3. Write transaction version
  const versionView = new DataView(new ArrayBuffer(4));
  versionView.setUint32(0, transaction.version, true);
  data.push(new Uint8Array(versionView.buffer));
  
  // 4. Write inputs count
  data.push(encodeVarInt(transaction.inputs.length));
  
  // 5. Write each input
  for (const input of transaction.inputs) {
    // Prev txid (reversed)
    const txidBytes = hexToBytes(input.prev_txid);
    data.push(txidBytes.reverse());
    
    // Prev index
    const indexView = new DataView(new ArrayBuffer(4));
    indexView.setUint32(0, input.prev_index, true);
    data.push(new Uint8Array(indexView.buffer));
    
    // Script sig length
    const scriptBytes = hexToBytes(input.script_sig);
    data.push(encodeVarInt(scriptBytes.length));
    data.push(scriptBytes);
    
    // Sequence
    const seqView = new DataView(new ArrayBuffer(4));
    seqView.setUint32(0, input.sequence, true);
    data.push(new Uint8Array(seqView.buffer));
  }
  
  // 6. Write outputs count
  data.push(encodeVarInt(transaction.outputs.length));
  
  // 7. Write each output
  for (const output of transaction.outputs) {
    // Value
    const valueView = new DataView(new ArrayBuffer(8));
    valueView.setBigUint64(0, BigInt(output.value), true);
    data.push(new Uint8Array(valueView.buffer));
    
    // Script pubkey length
    const scriptBytes = hexToBytes(output.script_pubkey);
    data.push(encodeVarInt(scriptBytes.length));
    data.push(scriptBytes);
  }
  
  // 8. Write locktime
  const lockView = new DataView(new ArrayBuffer(4));
  lockView.setUint32(0, transaction.locktime, true);
  data.push(new Uint8Array(lockView.buffer));
  
  // 9. Write genesis hash
  data.push(hexToBytes(GENESIS_HASH));
  
  // Concatenate and hash
  const totalLength = data.reduce((sum, arr) => sum + arr.length, 0);
  const combined = new Uint8Array(totalLength);
  let offset = 0;
  
  for (const arr of data) {
    combined.set(arr, offset);
    offset += arr.length;
  }
  
  const hashBuffer = await crypto.subtle.digest('SHA-256', combined);
  return bytesToHex(new Uint8Array(hashBuffer));
}

// Helper functions
function encodeVarInt(n: number): Uint8Array {
  if (n < 0xfd) {
    return new Uint8Array([n]);
  } else if (n <= 0xffff) {
    const result = new Uint8Array(3);
    result[0] = 0xfd;
    new DataView(result.buffer).setUint16(1, n, true);
    return result;
  } else if (n <= 0xffffffff) {
    const result = new Uint8Array(5);
    result[0] = 0xfe;
    new DataView(result.buffer).setUint32(1, n, true);
    return result;
  } else {
    const result = new Uint8Array(9);
    result[0] = 0xff;
    new DataView(result.buffer).setBigUint64(1, BigInt(n), true);
    return result;
  }
}

function hexToBytes(hex: string): Uint8Array {
  const result = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    result[i / 2] = parseInt(hex.substr(i, 2), 16);
  }
  return result;
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

// Wallet API class
export class WalletAPI {
  static async createWallet(request: WalletCreateRequest): Promise<WalletCreateResponse> {
    return await invoke('create_wallet', { request });
  }

  static async unlockWallet(request: WalletUnlockRequest): Promise<WalletUnlockResponse> {
    return await invoke('unlock_wallet', { request });
  }

  static async signTransaction(request: TransactionSignRequest): Promise<TransactionSignResponse> {
    return await invoke('sign_transaction', { request });
  }

  static async getWalletStatus(): Promise<WalletStatusResponse> {
    return await invoke('get_wallet_status');
  }

  static async lockWallet(): Promise<boolean> {
    return await invoke('lock_wallet');
  }

  static async clearCache(): Promise<boolean> {
    return await invoke('clear_cache');
  }

  static async sendRawTransaction(request: RawTransactionRequest): Promise<RawTransactionResponse> {
    return await invoke('send_raw_transaction', { request });
  }

  static async getTransactionHistory(limit?: number): Promise<TransactionHistory[]> {
    return await invoke('get_transaction_history', { limit });
  }

  static async getWalletBalance(): Promise<WalletBalance> {
    return await invoke('get_wallet_balance');
  }

  static async getNetworkInfo(): Promise<NetworkInfo> {
    return await invoke('get_network_info');
  }

  static async estimateFee(targetBlocks?: number): Promise<{ feeRate: number; confidence: number }> {
    return await invoke('estimate_fee', { targetBlocks });
  }

  static async getTransactionStatus(txid: string): Promise<{
    status: 'pending' | 'confirmed' | 'failed';
    confirmations: number;
    blockHeight?: number;
    timestamp?: number;
  }> {
    return await invoke('get_transaction_status', { txid });
  }

  static async getUtxos(): Promise<Array<{
    txid: string;
    vout: number;
    value: number;
    scriptPubkey: string;
    confirmations: number;
  }>> {
    return await invoke('get_utxos');
  }

  static async validateAddress(address: string): Promise<{ valid: boolean; error?: string }> {
    return await invoke('validate_address', { address });
  }

  static async backupWallet(password: string): Promise<{ success: boolean; backupData?: string; error?: string }> {
    return await invoke('backup_wallet', { password });
  }

  static async getMnemonicPhrase(password: string): Promise<{ success: boolean; mnemonic?: string; error?: string }> {
    return await invoke('get_mnemonic_phrase', { password });
  }

  static async createBackup(request: BackupRequest): Promise<BackupResponse> {
    return await invoke('create_backup', { request });
  }

  static async restoreWallet(request: RestoreRequest): Promise<RestoreResponse> {
    return await invoke('restore_wallet', { request });
  }

  static async validateAddress(address: string): Promise<{ valid: boolean; error?: string }> {
    return await invoke('validate_address', { address });
  }

  static async estimateFee(targetBlocks?: number): Promise<{ feeRate: number; confidence: number }> {
    return await invoke('estimate_fee', { targetBlocks });
  }

  static async getTransactionStatus(txid: string): Promise<{
    status: 'pending' | 'confirmed' | 'failed';
    confirmations: number;
    blockHeight?: number;
    timestamp?: number;
  }> {
    return await invoke('get_transaction_status', { txid });
  }

  static async exportKeystore(password: string): Promise<{ success: boolean; data?: string; error?: string }> {
    return await invoke('export_keystore', { password });
  }

  static async importKeystore(keystoreData: string, password: string): Promise<{ success: boolean; error?: string }> {
    return await invoke('import_keystore', { keystoreData, password });
  }

  static async changePassword(oldPassword: string, newPassword: string): Promise<{ success: boolean; error?: string }> {
    return await invoke('change_password', { oldPassword, newPassword });
  }

  static async getSecurityInfo(): Promise<{
    algorithm: string;
    securityLevel: string;
    memoryLocked: boolean;
    cacheEnabled: boolean;
    lastActivity: number;
  }> {
    return await invoke('get_security_info');
  }
}