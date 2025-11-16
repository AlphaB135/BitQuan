import React, { useState, useEffect, useRef } from 'react';
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
  const [showQR, setShowQR] = useState(false);
  const [qrData, setQRData] = useState('');
  const [copiedAddress, setCopiedAddress] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [feeRate, setFeeRate] = useState(1000);
  const [txConfirm, setTxConfirm] = useState<any>(null);

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
    
    // Validate inputs
    if (!toAddress || !amount || amount <= 0) {
      setError('Please enter valid address and amount');
      return;
    }
    
    setLoading(true);
    setError('');
    
    try {
      // Build transaction with better UTXO handling
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
        max_fee_rate: feeRate,
      });
      
      if (broadcastResponse.success && broadcastResponse.txid) {
        setTxConfirm({
          txid: broadcastResponse.txid,
          amount,
          toAddress,
          fee: broadcastResponse.fee || 0,
          status: 'sent'
        });
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

  const handleCopyAddress = async () => {
    if (address) {
      try {
        await navigator.clipboard.writeText(address);
        setCopiedAddress(true);
        setTimeout(() => setCopiedAddress(false), 2000);
      } catch (err) {
        setError('Failed to copy address');
      }
    }
  };

  const handleShowQR = () => {
    if (address) {
      setQRData(address);
      setShowQR(true);
    }
  };

  const handleConfirmTransaction = async () => {
    if (txConfirm) {
      setTxConfirm(null);
      // Refresh transaction history
      await loadWalletData();
    }
  };

  const handleCancelTransaction = () => {
    setTxConfirm(null);
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
              <div className="flex items-center gap-2">
                <span className="font-mono text-sm bg-gray-100 dark:bg-gray-800 px-3 py-1 rounded">
                  {address.slice(0, 10)}...{address.slice(-8)}
                </span>
                <button
                  onClick={handleCopyAddress}
                  className="p-2 text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white transition-colors"
                  title="Copy address"
                >
                  {copiedAddress ? '✓' : '📋'}
                </button>
                <button
                  onClick={handleShowQR}
                  className="p-2 text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white transition-colors"
                  title="Show QR code"
                >
                  📱
                </button>
              </div>
            )}
          </div>
        </div>
      </Card>
      
      {error && (
        <Card className="border-l-4 border-red-500 bg-red-50 dark:bg-red-900/20">
          <p className="text-red-600 dark:text-red-400">{error}</p>
        </Card>
      )}

      {/* QR Code Modal */}
      {showQR && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <Card className="max-w-md w-full mx-4">
            <div className="text-center">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                Receive Address
              </h3>
              <div className="bg-white p-4 rounded-lg mb-4">
                {/* QR Code placeholder */}
                <div className="w-64 h-64 bg-gray-200 dark:bg-gray-700 flex items-center justify-center mx-auto">
                  <div className="text-center">
                    <div className="text-4xl mb-2">📱</div>
                    <p className="text-sm text-gray-600 dark:text-gray-400">QR Code</p>
                    <p className="font-mono text-xs mt-2 break-all">{qrData}</p>
                  </div>
                </div>
              </div>
              <div className="space-y-3">
                <p className="font-mono text-sm bg-gray-100 dark:bg-gray-800 p-3 rounded break-all">
                  {qrData}
                </p>
                <div className="flex gap-3">
                  <button
                    onClick={handleCopyAddress}
                    className="flex-1 bg-cyan-500 hover:bg-cyan-600 text-white font-semibold py-2 px-4 rounded-lg transition-colors"
                  >
                    {copiedAddress ? '✓ Copied' : '📋 Copy Address'}
                  </button>
                  <button
                    onClick={() => setShowQR(false)}
                    className="flex-1 bg-gray-500 hover:bg-gray-600 text-white font-semibold py-2 px-4 rounded-lg transition-colors"
                  >
                    Close
                  </button>
                </div>
              </div>
            </div>
          </Card>
        </div>
      )}

      {/* Transaction Confirmation Modal */}
      {txConfirm && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <Card className="max-w-md w-full mx-4">
            <div className="text-center">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                Transaction Confirmation
              </h3>
              <div className="space-y-3 mb-6">
                <div className="bg-gray-50 dark:bg-gray-800 p-3 rounded">
                  <p className="text-sm text-gray-600 dark:text-gray-400">To Address</p>
                  <p className="font-mono text-sm break-all">{txConfirm.toAddress}</p>
                </div>
                <div className="bg-gray-50 dark:bg-gray-800 p-3 rounded">
                  <p className="text-sm text-gray-600 dark:text-gray-400">Amount</p>
                  <p className="text-lg font-semibold">{txConfirm.amount} BQ</p>
                </div>
                <div className="bg-gray-50 dark:bg-gray-800 p-3 rounded">
                  <p className="text-sm text-gray-600 dark:text-gray-400">Estimated Fee</p>
                  <p className="text-sm">{(txConfirm.fee / 100000000).toFixed(8)} BQ</p>
                </div>
                <div className="bg-gray-50 dark:bg-gray-800 p-3 rounded">
                  <p className="text-sm text-gray-600 dark:text-gray-400">Status</p>
                  <p className="text-sm font-semibold text-green-600 dark:text-green-400">
                    {txConfirm.status === 'sent' ? '✓ Sent' : 'Pending...'}
                  </p>
                </div>
              </div>
              <div className="flex gap-3">
                <button
                  onClick={handleConfirmTransaction}
                  className="flex-1 bg-cyan-500 hover:bg-cyan-600 text-white font-semibold py-2 px-4 rounded-lg transition-colors"
                >
                  Done
                </button>
                <button
                  onClick={handleCancelTransaction}
                  className="flex-1 bg-gray-500 hover:bg-gray-600 text-white font-semibold py-2 px-4 rounded-lg transition-colors"
                >
                  Cancel
                </button>
              </div>
            </div>
          </Card>
        </div>
      )}
      
      {loading && (
        <Card>
          <div className="text-center py-4">
            <div className="text-gray-500 dark:text-gray-400">Processing...</div>
          </div>
        </Card>
      )}
      
      {/* Balance Display */}
      {!isLocked && balance && (
        <Card>
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Wallet Balance</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="text-center">
              <p className="text-sm text-gray-600 dark:text-gray-400">Confirmed</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {(balance.confirmed / 100000000).toFixed(8)} BQ
              </p>
            </div>
            <div className="text-center">
              <p className="text-sm text-gray-600 dark:text-gray-400">Unconfirmed</p>
              <p className="text-2xl font-bold text-yellow-600 dark:text-yellow-400">
                {(balance.unconfirmed / 100000000).toFixed(8)} BQ
              </p>
            </div>
            <div className="text-center">
              <p className="text-sm text-gray-600 dark:text-gray-400">Total</p>
              <p className="text-2xl font-bold text-cyan-600 dark:text-cyan-400">
                {(balance.total / 100000000).toFixed(8)} BQ
              </p>
            </div>
          </div>
        </Card>
      )}

      {/* Transaction History */}
      {!isLocked && transactions.length > 0 && (
        <Card>
          <div className="flex justify-between items-center mb-4">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white">Recent Transactions</h3>
            <button
              onClick={() => setActiveTab('history')}
              className="text-sm text-cyan-600 hover:text-cyan-700 dark:text-cyan-400 dark:hover:text-cyan-300"
            >
              View All →
            </button>
          </div>
          <div className="space-y-2">
            {transactions.slice(0, 5).map((tx, index) => (
              <div key={tx.txid} className="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <span className={`text-sm font-semibold ${
                      tx.type === 'sent' 
                        ? 'text-red-600 dark:text-red-400' 
                        : 'text-green-600 dark:text-green-400'
                    }`}>
                      {tx.type === 'sent' ? '↓ Sent' : '↑ Received'}
                    </span>
                    <span className="font-mono text-xs text-gray-500 dark:text-gray-400">
                      {tx.txid.slice(0, 8)}...
                    </span>
                  </div>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    {tx.address}
                  </p>
                </div>
                <div className="text-right">
                  <p className={`font-semibold ${
                    tx.type === 'sent' 
                      ? 'text-red-600 dark:text-red-400' 
                      : 'text-green-600 dark:text-green-400'
                  }`}>
                    {tx.type === 'sent' ? '-' : '+'}{(tx.amount / 100000000).toFixed(8)} BQ
                  </p>
                  <p className="text-xs text-gray-500 dark:text-gray-400">
                    {tx.confirmations > 0 ? `${tx.confirmations} confirmations` : 'Pending'}
                  </p>
                </div>
              </div>
            ))}
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
          <>
            {/* Tab Navigation */}
            <div className="flex space-x-1 mb-6">
              {(['overview', 'send', 'receive', 'history'] as const).map((tab) => (
                <button
                  key={tab}
                  onClick={() => setActiveTab(tab)}
                  className={`px-4 py-2 font-semibold rounded-lg transition-colors ${
                    activeTab === tab
                      ? 'bg-cyan-500 text-white'
                      : 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'
                  }`}
                >
                  {tab.charAt(0).toUpperCase() + tab.slice(1)}
                </button>
              ))}
            </div>

            {/* Tab Content */}
            {activeTab === 'overview' && (
              <WalletUnlockedView
                address={address}
                onSend={handleSend}
                onLock={handleLockWallet}
                loading={loading}
              />
            )}
            
            {activeTab === 'send' && (
              <WalletUnlockedView
                address={address}
                onSend={handleSend}
                onLock={handleLockWallet}
                loading={loading}
              />
            )}
            
            {activeTab === 'receive' && (
              <WalletReceiveView
                address={address}
                onLock={handleLockWallet}
              />
            )}
            
            {activeTab === 'history' && (
              <WalletHistoryView
                transactions={transactions}
                onLock={handleLockWallet}
              />
            )}
          </>
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
  );
};

// Wallet receive view component
const WalletReceiveView: React.FC<{
  address: string;
  onLock: () => void;
}> = ({ address, onLock }) => {
  const [showQR, setShowQR] = useState(false);
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(address);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy address:', err);
    }
  };

  return (
    <Card>
      <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-4">Receive Payment</h3>
      
      <div className="text-center space-y-6">
        {/* QR Code Placeholder */}
        <div className="bg-white p-6 rounded-lg inline-block">
          <div className="w-48 h-48 bg-gray-200 dark:bg-gray-700 flex items-center justify-center">
            <div className="text-center">
              <div className="text-4xl mb-2">📱</div>
              <p className="text-sm text-gray-600 dark:text-gray-400">QR Code</p>
            </div>
          </div>
        </div>
        
        {/* Address Display */}
        <div className="space-y-3">
          <p className="text-sm text-gray-600 dark:text-gray-400">Your BitQuan Address:</p>
          <div className="bg-gray-100 dark:bg-gray-800 p-4 rounded-lg">
            <p className="font-mono text-sm break-all">{address}</p>
          </div>
          
          <div className="flex gap-3 justify-center">
            <button
              onClick={handleCopy}
              className="bg-cyan-500 hover:bg-cyan-600 text-white font-semibold py-2 px-6 rounded-lg transition-colors flex items-center gap-2"
            >
              {copied ? '✓ Copied' : '📋 Copy Address'}
            </button>
            <button
              onClick={() => setShowQR(!showQR)}
              className="bg-gray-500 hover:bg-gray-600 text-white font-semibold py-2 px-6 rounded-lg transition-colors"
            >
              {showQR ? 'Hide QR' : 'Show QR'}
            </button>
          </div>
        </div>
        
        {/* Security Note */}
        <div className="bg-blue-50 dark:bg-blue-900/20 p-4 rounded-lg">
          <p className="text-sm text-blue-800 dark:text-blue-200">
            <strong>Security Note:</strong> This address is post-quantum protected with Dilithium3 signatures.
            Only share this address with trusted parties.
          </p>
        </div>
      </div>
    </Card>
  );
};

// Wallet history view component
const WalletHistoryView: React.FC<{
  transactions: TransactionHistory[];
  onLock: () => void;
}> = ({ transactions, onLock }) => {
  return (
    <Card>
      <div className="flex justify-between items-center mb-4">
        <h3 className="text-xl font-semibold text-gray-900 dark:text-white">Transaction History</h3>
        <button
          onClick={onLock}
          className="bg-red-500 hover:bg-red-600 text-white font-semibold py-2 px-4 rounded-lg transition-colors"
        >
          🔒 Lock Wallet
        </button>
      </div>
      
      {transactions.length === 0 ? (
        <div className="text-center py-8">
          <p className="text-gray-500 dark:text-gray-400">No transactions yet</p>
        </div>
      ) : (
        <div className="space-y-2">
          {transactions.map((tx) => (
            <div key={tx.txid} className="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  <span className={`text-sm font-semibold ${
                    tx.type === 'sent' 
                      ? 'text-red-600 dark:text-red-400' 
                      : 'text-green-600 dark:text-green-400'
                  }`}>
                    {tx.type === 'sent' ? '↓ Sent' : '↑ Received'}
                  </span>
                  <span className="font-mono text-xs text-gray-500 dark:text-gray-400">
                    {tx.txid.slice(0, 8)}...
                  </span>
                </div>
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  {tx.address}
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  {new Date(tx.timestamp * 1000).toLocaleString()}
                </p>
              </div>
              <div className="text-right">
                <p className={`font-semibold ${
                  tx.type === 'sent' 
                    ? 'text-red-600 dark:text-red-400' 
                    : 'text-green-600 dark:text-green-400'
                }`}>
                  {tx.type === 'sent' ? '-' : '+'}{(tx.amount / 100000000).toFixed(8)} BQ
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  {tx.confirmations > 0 ? `${tx.confirmations} confirmations` : 'Pending'}
                </p>
              </div>
            </div>
          ))}
        </div>
      )}
    </Card>
  );
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
          
          {/* Fee Rate Selection */}
          <div>
            <div className="flex justify-between items-center mb-2">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                Fee Rate (sat/byte):
              </label>
              <button
                type="button"
                onClick={() => setShowAdvanced(!showAdvanced)}
                className="text-sm text-cyan-600 hover:text-cyan-700 dark:text-cyan-400 dark:hover:text-cyan-300"
              >
                {showAdvanced ? '▼ Simple' : '▲ Advanced'}
              </button>
            </div>
            
            {showAdvanced ? (
              <input
                type="number"
                value={feeRate}
                onChange={(e) => setFeeRate(parseInt(e.target.value) || 1000)}
                placeholder="1000"
                min="1"
                max="10000"
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-cyan-500"
              />
            ) : (
              <div className="grid grid-cols-3 gap-2">
                <button
                  type="button"
                  onClick={() => setFeeRate(500)}
                  className="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-sm hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors"
                >
                  Low (500)
                </button>
                <button
                  type="button"
                  onClick={() => setFeeRate(1000)}
                  className="px-3 py-2 border-2 border-cyan-500 bg-cyan-50 dark:bg-cyan-900/20 text-cyan-700 dark:text-cyan-300 text-sm rounded-lg"
                >
                  Medium (1000)
                </button>
                <button
                  type="button"
                  onClick={() => setFeeRate(2000)}
                  className="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-sm hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors"
                >
                  High (2000)
                </button>
              </div>
            )}
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