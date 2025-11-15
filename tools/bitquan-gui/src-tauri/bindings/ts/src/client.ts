import axios, { AxiosInstance, AxiosResponse } from 'axios';
import { 
  Config, 
  Network, 
  Block, 
  Account, 
  TransactionInfo,
  BlockchainInfo,
  PeerInfo,
  MempoolInfo,
  RpcRequest,
  RpcResponse
} from './types';
import { BitQuanError } from './errors';

export class BitQuanClient {
  private client: AxiosInstance;
  private config: Config;

  constructor(config?: Partial<Config>) {
    this.config = {
      rpc_url: 'https://api.bitquan.network/rpc',
      network: Network.MAINNET,
      timeout_seconds: 30,
      max_retries: 3,
      ...config
    };

    this.client = axios.create({
      baseURL: this.config.rpc_url,
      timeout: this.config.timeout_seconds * 1000,
      headers: {
        'Content-Type': 'application/json',
      },
    });
  }

  private async rpcCall<T, R>(method: string, params: T): Promise<R> {
    const request: RpcRequest<T> = {
      jsonrpc: '2.0',
      method,
      params,
      id: Math.floor(Math.random() * 1000000)
    };

    let retries = 0;
    while (retries < this.config.max_retries) {
      try {
        const response: AxiosResponse<RpcResponse<R>> = await this.client.post('', request);
        
        if (response.data.error) {
          throw new BitQuanError(
            'RPC_ERROR',
            response.data.error.message,
            response.data.error
          );
        }
        
        if (!response.data.result) {
          throw new BitQuanError('NO_RESULT', 'No result in RPC response');
        }
        
        return response.data.result;
      } catch (error) {
        retries++;
        if (retries >= this.config.max_retries) {
          if (error instanceof BitQuanError) {
            throw error;
          }
          const errorMessage = error instanceof Error ? error.message : String(error);
          throw new BitQuanError('NETWORK_ERROR', `Network error: ${errorMessage}`);
        }
        await new Promise(resolve => setTimeout(resolve, 1000 * retries));
      }
    }

    throw new BitQuanError('TIMEOUT', 'Request timeout after retries');
  }

  async getBlockchainInfo(): Promise<BlockchainInfo> {
    return this.rpcCall('getblockchaininfo', {});
  }

  async getBlockByHeight(height: number): Promise<Block> {
    return this.rpcCall('getblockbyheight', [height]);
  }

  async getBlockByHash(hash: string): Promise<Block> {
    return this.rpcCall('getblockbyhash', [hash]);
  }

  async getAccount(address: string): Promise<Account> {
    return this.rpcCall('getaccount', [address]);
  }

  async getBalance(address: string): Promise<number> {
    return this.rpcCall('getbalance', [address]);
  }

  async getTransaction(txHash: string): Promise<TransactionInfo> {
    return this.rpcCall('gettransaction', [txHash]);
  }

  async sendRawTransaction(txHex: string): Promise<string> {
    return this.rpcCall('sendrawtransaction', [txHex]);
  }

  async getBlockHeight(): Promise<number> {
    return this.rpcCall('getblockheight', {});
  }

  async getNetworkHashRate(): Promise<number> {
    return this.rpcCall('getnetworkhashps', {});
  }

  async getPeerInfo(): Promise<PeerInfo[]> {
    return this.rpcCall('getpeerinfo', {});
  }

  async estimateFee(txSize: number): Promise<number> {
    return this.rpcCall('estimatefee', [txSize]);
  }

  async validateAddress(address: string): Promise<boolean> {
    return this.rpcCall('validateaddress', [address]);
  }

  async getMempoolInfo(): Promise<MempoolInfo> {
    return this.rpcCall('getmempoolinfo', {});
  }

  async getRawMempool(): Promise<string[]> {
    return this.rpcCall('getrawmempool', {});
  }

  getConfig(): Config {
    return { ...this.config };
  }

  static create(network?: Network): BitQuanClient {
    const rpcUrls = {
      [Network.MAINNET]: 'https://api.bitquan.network/rpc',
      [Network.TESTNET]: 'https://testnet-api.bitquan.network/rpc',
      [Network.DEVNET]: 'https://devnet-api.bitquan.network/rpc'
    };

    return new BitQuanClient({
      rpc_url: rpcUrls[network || Network.MAINNET],
      network: network || Network.MAINNET
    });
  }
}