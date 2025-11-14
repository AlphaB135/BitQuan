import React, { useState, useEffect } from 'react';
import { WalletAPI, EncryptedKeystoreData, calculateTransactionSighash } from '../api/wallet';

export const WalletPage: React.FC = () => {
  const [isLocked, setIsLocked] = useState(true);
  const [address, setAddress] = useState<string>('');
  const [keystoreData, setKeystoreData] = useState<EncryptedKeystoreData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>('');

  useEffect(() => {
    loadWalletStatus();
  }, []);

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
    <div className="wallet-page">
      <div className="wallet-header">
        <h2>BitQuan PQC Wallet</h2>
        <div className="wallet-status">
          <span className={`status ${isLocked ? 'locked' : 'unlocked'}`}>
            {isLocked ? '🔒 Locked' : '🔓 Unlocked'}
          </span>
          {address && <span className="address">{address}</span>}
        </div>
      </div>
      {error && (
        <div className="error-message">
          {error}
        </div>
      )}
      {loading && (
        <div className="loading">
          Processing...
        </div>
      )}
      <div className="wallet-actions">
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
    <div className="wallet-form">
      <div className="mode-selector">
        <button 
          className={mode === 'unlock' ? 'active' : ''}
          onClick={() => setMode('unlock')}
        >
          Unlock Existing
        </button>
        <button 
          className={mode === 'create' ? 'active' : ''}
          onClick={() => setMode('create')}
        >
          Create New
        </button>
      </div>
      <form onSubmit={handleSubmit}>
        {mode === 'unlock' && (
          <div className="form-group">
            <label>Keystore JSON:</label>
            <textarea
              value={keystoreJson}
              onChange={(e) => setKeystoreJson(e.target.value)}
              placeholder="Paste your encrypted keystore JSON here..."
              rows={6}
              required
            />
          </div>
        )}
        {mode === 'create' && (
          <div className="form-group">
            <label>Address Hint (optional):</label>
            <input
              type="text"
              value={addressHint}
              onChange={(e) => setAddressHint(e.target.value)}
              placeholder="bq1..."
            />
          </div>
        )}
        <div className="form-group">
          <label>Password:</label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
        </div>
        {mode === 'create' && (
          <div className="form-group">
            <label>Confirm Password:</label>
            <input
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              required
            />
          </div>
        )}
        <button type="submit" disabled={loading}>
          {loading ? 'Processing...' : (mode === 'create' ? 'Create Wallet' : 'Unlock Wallet')}
        </button>
      </form>
    </div>
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
    <div className="unlocked-wallet">
      <div className="wallet-info">
        <p><strong>Address:</strong> {address}</p>
      </div>
      <div className="send-form">
        <h3>Send Transaction</h3>
        <form onSubmit={handleSend}>
          <div className="form-group">
            <label>To Address:</label>
            <input
              type="text"
              value={toAddress}
              onChange={(e) => setToAddress(e.target.value)}
              placeholder="bq1..."
              required
            />
          </div>
          <div className="form-group">
            <label>Amount:</label>
            <input
              type="number"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder="0.00"
              step="0.00000001"
              required
            />
          </div>
          <div className="form-group">
            <label>Confirm Password:</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Re-enter password for security"
              required
            />
          </div>
          <button type="submit" disabled={loading}>
            {loading ? 'Signing...' : 'Send Transaction'}
          </button>
        </form>
      </div>
      <div className="wallet-actions">
        <button onClick={onLock} className="lock-button">
          🔒 Lock Wallet
        </button>
      </div>
    </div>
  );
};