export enum Network {
  MAINNET = 'mainnet',
  TESTNET = 'testnet',
  DEVNET = 'devnet'
}

export interface Block {
  hash: string;
  height: number;
  timestamp: string;
  transactions: string[];
  previous_hash: string;
  merkle_root: string;
  difficulty: number;
  nonce: number;
}

export interface Account {
  address: string;
  balance: number;
  nonce: number;
  stake?: number;
  created_at: string;
}

export interface Transaction {
  version: number;
  inputs: TransactionInput[];
  outputs: TransactionOutput[];
  lock_time: number;
  fee: number;
  signature?: string;
  timestamp?: number;
}

export interface TransactionInput {
  previous_output: OutPoint;
  script_sig?: string;
  sequence: number;
}

export interface TransactionOutput {
  value: number;
  script_pubkey: string;
  address?: string;
}

export interface OutPoint {
  hash: string;
  index: number;
}

export interface TransactionStatus {
  PENDING: 'pending';
  CONFIRMED: 'confirmed';
  FAILED: 'failed';
}

export interface Config {
  rpc_url: string;
  network: Network;
  timeout_seconds: number;
  max_retries: number;
}

export interface WalletExport {
  address: string;
  private_key: string;
  public_key: string;
  network: Network;
}

export interface KeyPair {
  private_key: string;
  public_key: string;
  address: string;
}

export interface MerkleProof {
  leaf: string;
  proof: string[];
  root: string;
}

export interface BlockchainInfo {
  chain: string;
  blocks: number;
  headers: number;
  best_block_hash: string;
  difficulty: number;
  median_time: number;
  verification_progress: number;
  chainwork: string;
  size_on_disk: number;
  pruned: boolean;
}

export interface TransactionInfo {
  txid: string;
  hash: string;
  version: number;
  size: number;
  vsize: number;
  weight: number;
  locktime: number;
  vin: Vin[];
  vout: Vout[];
  blockhash?: string;
  confirmations: number;
  time: number;
  blocktime?: number;
}

export interface Vin {
  txid: string;
  vout: number;
  script_sig?: ScriptSig;
  sequence: number;
}

export interface ScriptSig {
  asm: string;
  hex: string;
}

export interface Vout {
  value: number;
  n: number;
  script_pub_key: ScriptPubKey;
}

export interface ScriptPubKey {
  asm: string;
  hex: string;
  req_sigs?: number;
  type: string;
  addresses?: string[];
}

export interface PeerInfo {
  id: number;
  addr: string;
  addr_local?: string;
  services: string;
  relaytxes: boolean;
  lastsend: number;
  lastrecv: number;
  pingtime?: number;
  version: number;
  subver: string;
  inbound: boolean;
  startingheight: number;
  banscore: number;
  synced_headers: number;
  synced_blocks: number;
  inflight: string[];
  whitelisted: boolean;
}

export interface MempoolInfo {
  size: number;
  bytes: number;
  usage: number;
  maxmempool: number;
  mempoolminfee: number;
  minrelaytxfee: number;
}

export interface BitQuanError {
  code: string;
  message: string;
  details?: any;
}

export interface RpcRequest<T> {
  jsonrpc: string;
  method: string;
  params: T;
  id: number;
}

export interface RpcResponse<T> {
  jsonrpc: string;
  result?: T;
  error?: RpcError;
  id: number;
}

export interface RpcError {
  code: number;
  message: string;
}