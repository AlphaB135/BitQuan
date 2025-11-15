import React, { useState, useEffect } from 'react';
import { WalletAPI, EncryptedKeystoreData, calculateTransactionSighash, TransactionHistory, WalletBalance, NetworkInfo } from '../api/wallet';

export const WalletPage: React.FC = () => {
  const [isLocked, setIsLocked] = useState(true);
  const [address, setAddress] = useState<string>('');
  const [keystoreData, setKeystoreData] = useState<EncryptedKeystoreData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>('');
  const [balance, setBalance] = useState<WalletBalance | null>(null);
  const [transactions, setTransactions] = useState<TransactionHistory[]>([]);
  const [networkInfo, setNetworkInfo] = useState<NetworkInfo | null>(null);
  const [activeTab, setActiveTab] = useState<'overview' | 'send' | 'receive' | 'history' | 'settings'>('overview');

  useEffect(() => {
    loadWalletStatus();
    if (!isLocked) {
      loadWalletData();
    }
  }, [isLocked]);

  const loadWalletData = async () => {
    try {
      const [balanceData, txHistory, netInfo] = await Promise.all([
        WalletAPI.getWalletBalance(),
        WalletAPI.getTransactionHistory(10),
        WalletAPI.getNetworkInfo()
      ]);
      setBalance(balanceData);
      setTransactions(txHistory);
      setNetworkInfo(netInfo);
    } catch (err) {
      console.error('Failed to load wallet data:', err);
    }
  };

  const loadWalletStatus = async () => {
    try {
      const status = await WalletAPI.getWalletStatus();
      setIsLocked(status.is_locked);
      setAddress(status.address || '');
    } catch (err) {
      setError(`Failed to load wallet status: ${err}`);
    }
  };

  const handleCreateWallet = async (password: string, addressHint?: string) => {
    setLoading(true);
    setError('');
    
    try {
      const response = await WalletAPI.createWallet({
        password,
        address_hint: addressHint,
      });
      if (response.success && response.keystore_data) {
        setKeystoreData(response.keystore_data);
        setAddress(response.keystore_data.address);
        await handleUnlockWallet(response.keystore_data, password);
      } else {
        setError(response.error || 'Failed to create wallet');
      }
    } catch (err) {
      setError(`Wallet creation failed: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleUnlockWallet = async (keystore: EncryptedKeystoreData, password: string) => {
    setLoading(true);
    setError('');
    
    try {
      const response = await WalletAPI.unlockWallet({
        keystore_data: keystore,
        password,
      });
      if (response.success) {
        setIsLocked(false);
        setAddress(response.address);
        await loadWalletStatus();
      } else {
        setError(response.error || 'Failed to unlock wallet');
      }
    } catch (err) {
      setError(`Wallet unlock failed: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleSend = async (toAddress: string, amount: number, password: string) => {
    if (isLocked) {
      setError('Please unlock wallet first');
      return;
    }
    setLoading(true);
    setError('');
    
    try {
      // Build transaction
      const transaction = {
        version: 1,
        inputs: [
          {
            prev_txid: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            prev_index: 0,
            script_sig: "",
            sequence: 0xffffffff,
          }
        ],
        outputs: [
          {
            value: Math.floor(amount * 100000000),
            script_pubkey: `76a914${toAddress.slice(3, 23)}88ac`,
          }
        ],
        locktime: 0,
      };
      
      // Calculate sighash
      const sighashHex = await calculateTransactionSighash(transaction);
      
      // Sign with PQC
      const signResponse = await WalletAPI.signTransaction({
        sighash_hex: sighashHex,
        password,
      });
      
      if (!signResponse.success) {
        setError(signResponse.error || 'Failed to sign transaction');
        return;
      }
      
      // Build final transaction
      const finalTx = {
        ...transaction,
        inputs: [
          {
            ...transaction.inputs[0],
            script_sig: signResponse.signature_hex!,
          }
        ],
      };
      
      // Serialize and broadcast
      const txHex = await serializeTransaction(finalTx);
      
      const broadcastResponse = await WalletAPI.sendRawTransaction({
        tx_hex: txHex,
        max_fee_rate: 1000,
      });
      
      if (broadcastResponse.success && broadcastResponse.txid) {
        alert(`Transaction sent! TXID: ${broadcastResponse.txid}`);
        await loadWalletStatus();
      } else {
        setError(broadcastResponse.error || 'Failed to broadcast transaction');
      }
    } catch (err) {
      setError(`Transaction failed: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleLockWallet = async () => {
    try {
      await WalletAPI.lockWallet();
      setIsLocked(true);
      await loadWalletStatus();
    } catch (err) {
      setError(`Failed to lock wallet: ${err}`);
    }
  };

  // Helper functions
  const hexToBytes = (hex: string): Uint8Array => {
    const result = new Uint8Array(hex.length / 2);
    for (let i = 0; i < hex.length; i += 2) {
      result[i / 2] = parseInt(hex.substr(i, 2), 16);
    }
    return result;
  };

  const bytesToHex = (bytes: Uint8Array): string => {
    return Array.from(bytes)
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');
  };

  const encodeVarInt = (n: number): Uint8Array => {
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
  };

  const serializeTransaction = async (tx: any): Promise<string> => {
    const data: Uint8Array[] = [];
    
    // Version
    const versionView = new DataView(new ArrayBuffer(4));
    versionView.setUint32(0, tx.version, true);
    data.push(new Uint8Array(versionView.buffer));
    
    // Inputs
    data.push(encodeVarInt(tx.inputs.length));
    for (const input of tx.inputs) {
      const txidBytes = hexToBytes(input.prev_txid);
      data.push(txidBytes.reverse());
      
      const indexView = new DataView(new ArrayBuffer(4));
      indexView.setUint32(0, input.prev_index, true);
      data.push(new Uint8Array(indexView.buffer));
      
      const scriptBytes = hexToBytes(input.script_sig);
      data.push(encodeVarInt(scriptBytes.length));
      data.push(scriptBytes);
      
      const seqView = new DataView(new ArrayBuffer(4));
      seqView.setUint32(0, input.sequence, true);
      data.push(new Uint8Array(seqView.buffer));
    }
    
    // Outputs
    data.push(encodeVarInt(tx.outputs.length));
    for (const output of tx.outputs) {
      const valueView = new DataView(new ArrayBuffer(8));
      valueView.setBigUint64(0, BigInt(output.value), true);
      data.push(new Uint8Array(valueView.buffer));
      
      const scriptBytes = hexToBytes(output.script_pubkey);
      data.push(encodeVarInt(scriptBytes.length));
      data.push(scriptBytes);
    }
    
    // Locktime
    const lockView = new DataView(new ArrayBuffer(4));
    lockView.setUint32(0, tx.locktime, true);
    data.push(new Uint8Array(lockView.buffer));
    
    // Combine and hex encode
    const totalLength = data.reduce((sum, arr) => sum + arr.length, 0);
    const combined = new Uint8Array(totalLength);
    let offset = 0;
    
    for (const arr of data) {
      combined.set(arr, offset);
      offset += arr.length;
    }
    
    return bytesToHex(combined);
  };

  return (
    <div className="max-w-6xl mx-auto p-6 space-y-6">
      {/* Header */}
      <Card>
        <div className="flex justify-between items-center">
          <div>
            <h2 className="text-2xl font-bold text-gray-900 dark:text-white">BitQuan PQC Wallet</h2>
            <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
              Post-Quantum Cryptography Protected
            </p>
          </div>
          <div className="flex items-center gap-4">
            <span className={`px-3 py-1 rounded-full text-sm font-semibold ${
              isLocked 
                ? 'bg-red-100 text-red-600 dark:bg-red-900/20 dark:text-red-400' 
                : 'bg-green-100 text-green-600 dark:bg-green-900/20 dark:text-green-400'
            }`}>
              {isLocked ? '🔒 Locked' : '🔓 Unlocked'}
            </span>
            {networkInfo && (
              <span className="px-3 py-1 rounded-full text-sm font-semibold bg-blue-100 text-blue-600 dark:bg-blue-900/20 dark:text-blue-400">
                {networkInfo.network.toUpperCase()}
              </span>
            )}
            {address && (
              <span className="font-mono text-sm bg-gray-100 dark:bg-gray-800 px-3 py-1 rounded">
                {address.slice(0, 10)}...{address.slice(-8)}
              </span>
            )}
          </div>
        </div>
      </Card>
      
      {error && (
        <Card className="border-l-4 border-red-500 bg-red-50 dark:bg-red-900/20">
          <p className="text-red-600 dark:text-red-400">{error}</p>
        </Card>
      )}
      
      {loading && (
        <Card>
          <div className="text-center py-4">
            <div className="text-gray-500 dark:text-gray-400">Processing...</div>
          </div>
        </Card>
      )}
      
      <div className="space-y-6">
        {isLocked ? (
          <WalletUnlockForm 
            onUnlock={handleUnlockWallet}
            onCreate={handleCreateWallet}
            loading={loading}
          />
        ) : (
          <WalletUnlockedView
            address={address}
            onSend={handleSend}
            onLock={handleLockWallet}
            loading={loading}
          />
        )}
      </div>
    </div>
  );
};

// Wallet unlock/create form component
const WalletUnlockForm: React.FC<{
  onUnlock: (keystore: EncryptedKeystoreData, password: string) => void;
  onCreate: (password: string, addressHint?: string) => void;
  loading: boolean;
}> = ({ onUnlock, onCreate, loading }) => {
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [mode, setMode] = useState<'unlock' | 'create'>('unlock');
  const [keystoreJson, setKeystoreJson] = useState('');
  const [addressHint, setAddressHint] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (mode === 'create') {
      if (password !== confirmPassword) {
        alert('Passwords do not match');
        return;
      }
      await onCreate(password, addressHint || undefined);
    } else {
      try {
        const keystore = JSON.parse(keystoreJson) as EncryptedKeystoreData;
        await onUnlock(keystore, password);
      } catch (err) {
        alert('Invalid keystore format');
      }
    }
  };

  return (
    <Card>
      <div className="flex rounded-lg overflow-hidden border border-gray-200 dark:border-gray-600 mb-6">
        <button 
          className={`flex-1 px-4 py-3 font-semibold transition-colors ${
            mode === 'unlock' 
              ? 'bg-cyan-500 text-white' 
              : 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'
          }`}
          onClick={() => setMode('unlock')}
        >
          Unlock Existing
        </button>
        <button 
          className={`flex-1 px-4 py-3 font-semibold transition-colors ${
            mode === 'create' 
              ? 'bg-cyan-500 text-white' 
              : 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'
          }`}
          onClick={() => setMode('create')}
        >
          Create New
        </button>
      </div>
      
      <form onSubmit={handleSubmit} className="space-y-4">
        {mode === 'unlock' && (
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Keystore JSON:
            </label>
            <textarea
              value={keystoreJson}
              onChange={(e) => setKeystoreJson(e.target.value)}
              placeholder="Paste your encrypted keystore JSON here..."
              rows={6}
              required
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-cyan-500 font-mono text-sm"
            />
          </div>
        )}
        {mode === 'create' && (
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Address Hint (optional):
            </label>
            <input
              type="text"
              value={addressHint}
              onChange={(e) => setAddressHint(e.target.value)}
              placeholder="bq1..."
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-cyan-500"
            />
          </div>
        )}
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            Password:
          </label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-cyan-500"
          />
        </div>
        {mode === 'create' && (
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Confirm Password:
            </label>
            <input
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              required
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-cyan-500"
            />
          </div>
        )}
        <button 
          type="submit" 
          disabled={loading}
          className="w-full bg-cyan-500 hover:bg-cyan-600 disabled:bg-gray-400 text-white font-bold py-3 px-4 rounded-lg transition-colors"
        >
          {loading ? 'Processing...' : (mode === 'create' ? 'Create Wallet' : 'Unlock Wallet')}
        </button>
      </form>
    </Card>
  );
};

// Wallet unlocked view component
const WalletUnlockedView: React.FC<{
  address: string;
  onSend: (toAddress: string, amount: number, password: string) => void;
  onLock: () => void;
  loading: boolean;
}> = ({ address, onSend, onLock, loading }) => {
  const [toAddress, setToAddress] = useState('');
  const [amount, setAmount] = useState('');
  const [password, setPassword] = useState('');

  const handleSend = (e: React.FormEvent) => {
    e.preventDefault();
    onSend(toAddress, parseFloat(amount), password);
  };

  return (
    <div className="space-y-6">
      <Card>
        <div className="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-lg">
          <p className="text-sm text-gray-600 dark:text-gray-400 mb-1">Wallet Address:</p>
          <p className="font-mono text-sm break-all">{address}</p>
        </div>
      </Card>
      
      <Card>
        <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-4">Send Transaction</h3>
        <form onSubmit={handleSend} className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              To Address:
            </label>
            <input
              type="text"
              value={toAddress}
              onChange={(e) => setToAddress(e.target.value)}
              placeholder="bq1..."
              required
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-cyan-500 font-mono"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Amount:
            </label>
            <input
              type="number"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder="0.00"
              step="0.00000001"
              required
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-cyan-500"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Confirm Password:
            </label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Re-enter password for security"
              required
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-cyan-500"
            />
          </div>
          <button 
            type="submit" 
            disabled={loading}
            className="w-full bg-cyan-500 hover:bg-cyan-600 disabled:bg-gray-400 text-white font-bold py-3 px-4 rounded-lg transition-colors"
          >
            {loading ? 'Signing...' : 'Send Transaction'}
          </button>
        </form>
      </Card>
      
      <div className="text-center">
        <button 
          onClick={onLock} 
          className="bg-red-500 hover:bg-red-600 text-white font-bold py-2 px-6 rounded-lg transition-colors"
        >
          🔒 Lock Wallet
        </button>
      </div>
    </div>
  );
};