import React, { useState, useEffect } from 'react';
import Card from '../components/Card';
import { Theme } from '../App';
import { invoke } from '@tauri-apps/api';

interface SettingsPageProps {
  currentTheme: Theme;
  setTheme: (theme: Theme) => void;
}

const SettingsPage: React.FC<SettingsPageProps> = ({ currentTheme, setTheme }) => {
  const [rpcConfig, setRpcConfig] = useState<{ rpc_url: string; rpc_user: string; rpc_password: string }>({
    rpc_url: 'http://localhost:8332',
    rpc_user: '',
    rpc_password: ''
  });
  const [testingConnection, setTestingConnection] = useState(false);
  const [connectionStatus, setConnectionStatus] = useState<'idle' | 'success' | 'error'>('idle');
  const [selectedNetwork, setSelectedNetwork] = useState<'mainnet' | 'testnet'>('mainnet');
  const [networkStatus, setNetworkStatus] = useState<Record<string, string>>({});

  const SettingRow: React.FC<{ title: string, description: string, children: React.ReactNode }> = ({ title, description, children }) => (
    <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center py-4 border-b border-gray-200 dark:border-gray-700 gap-4">
      <div className="flex-1">
        <h3 className="text-lg font-semibold text-gray-800 dark:text-gray-100">{title}</h3>
        <p className="text-sm text-gray-500 dark:text-gray-400">{description}</p>
      </div>
      <div className="mt-2 sm:mt-0 sm:ml-4">
        {children}
      </div>
    </div>
  );

  useEffect(() => {
    const loadConfig = async () => {
      try {
        const config = await invoke<Record<string, string>>('get_rpc_config');
        setRpcConfig({
          rpc_url: config.rpc_url || 'http://localhost:8332',
          rpc_user: config.rpc_user || '',
          rpc_password: config.rpc_password || ''
        });
        
        // Load network status
        const status = await invoke<Record<string, string>>('get_network_status');
        setNetworkStatus(status);
        
        // Set selected network based on availability
        if (status.testnet && status.testnet !== 'Disconnected') {
          setSelectedNetwork('testnet');
        } else {
          setSelectedNetwork('mainnet');
        }
      } catch (error) {
        console.error('Failed to load RPC config:', error);
      }
    };
    loadConfig();
  }, []);

  const testConnection = async () => {
    setTestingConnection(true);
    setConnectionStatus('idle');
    try {
      const success = await invoke<boolean>('test_rpc_connection', {
        rpcUrl: rpcConfig.rpc_url,
        rpcUser: rpcConfig.rpc_user || null,
        rpcPassword: rpcConfig.rpc_password || null
      });
      setConnectionStatus(success ? 'success' : 'error');
    } catch (error) {
      setConnectionStatus('error');
      console.error('Connection test failed:', error);
    } finally {
      setTestingConnection(false);
    }
  };

  const switchNetwork = async (network: 'mainnet' | 'testnet') => {
    try {
      const result = await invoke<string>('switch_network', { network });
      console.log('Network switch result:', result);
      
      // Reload network status
      const status = await invoke<Record<string, string>>('get_network_status');
      setNetworkStatus(status);
      setSelectedNetwork(network);
      
      // Update RPC config based on selected network
      if (network === 'testnet') {
        setRpcConfig({
          rpc_url: 'http://localhost:19443',
          rpc_user: 'admin',
          rpc_password: 'testnet123'
        });
      } else {
        setRpcConfig({
          rpc_url: 'http://localhost:8332',
          rpc_user: 'admin',
          rpc_password: 'mysecretjwtkey'
        });
      }
    } catch (error) {
      console.error('Network switch failed:', error);
    }
  };

  return (
    <div className="space-y-8">
      <h1 className="text-3xl font-bold text-gray-900 dark:text-white">⚙️ Settings</h1>

      {/* Network Selection */}
      <Card>
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">🌐 Network Selection</h2>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Select Network
            </label>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <button
                onClick={() => switchNetwork('mainnet')}
                className={`p-3 sm:p-4 rounded-lg border-2 transition-all ${
                  selectedNetwork === 'mainnet'
                    ? 'border-cyan-500 bg-cyan-50 dark:bg-cyan-900/20 text-cyan-700 dark:text-cyan-300'
                    : 'border-gray-300 dark:border-gray-600 hover:border-gray-400 dark:hover:border-gray-500'
                }`}
              >
                <div className="text-base sm:text-lg font-bold">🟢 Mainnet</div>
                <div className="text-xs sm:text-sm text-gray-600 dark:text-gray-400 mt-1">
                  {networkStatus.mainnet || 'Checking...'}
                </div>
              </button>
              <button
                onClick={() => switchNetwork('testnet')}
                className={`p-3 sm:p-4 rounded-lg border-2 transition-all ${
                  selectedNetwork === 'testnet'
                    ? 'border-orange-500 bg-orange-50 dark:bg-orange-900/20 text-orange-700 dark:text-orange-300'
                    : 'border-gray-300 dark:border-gray-600 hover:border-gray-400 dark:hover:border-gray-500'
                }`}
              >
                <div className="text-base sm:text-lg font-bold">🧪 Testnet</div>
                <div className="text-xs sm:text-sm text-gray-600 dark:text-gray-400 mt-1">
                  {networkStatus.testnet || 'Checking...'}
                </div>
              </button>
            </div>
          </div>
        </div>
      </Card>

      {/* RPC Configuration */}
      <Card>
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">BitQuan RPC Configuration</h2>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              RPC URL
            </label>
            <input
              type="text"
              value={rpcConfig.rpc_url}
              onChange={(e) => setRpcConfig({ ...rpcConfig, rpc_url: e.target.value })}
              className="w-full bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-white px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-cyan-500"
              placeholder="http://localhost:8332"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              RPC Username (optional)
            </label>
            <input
              type="text"
              value={rpcConfig.rpc_user}
              onChange={(e) => setRpcConfig({ ...rpcConfig, rpc_user: e.target.value })}
              className="w-full bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-white px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-cyan-500"
              placeholder="username"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              RPC Password (optional)
            </label>
            <input
              type="password"
              value={rpcConfig.rpc_password}
              onChange={(e) => setRpcConfig({ ...rpcConfig, rpc_password: e.target.value })}
              className="w-full bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-white px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-cyan-500"
              placeholder="password"
            />
          </div>
          <div className="flex items-center space-x-4">
            <button
              onClick={testConnection}
              disabled={testingConnection}
              className="bg-cyan-500 hover:bg-cyan-600 disabled:bg-gray-400 text-white font-bold py-2 px-4 rounded-lg transition-colors"
            >
              {testingConnection ? 'Testing...' : 'Test Connection'}
            </button>
            {connectionStatus === 'success' && (
              <span className="text-green-500 dark:text-green-400">✓ Connection successful</span>
            )}
            {connectionStatus === 'error' && (
              <span className="text-red-500 dark:text-red-400">✗ Connection failed</span>
            )}
          </div>
        </div>
      </Card>

      {/* Appearance Settings */}
      <Card>
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">Appearance</h2>
        <SettingRow
            title="Theme"
            description="Choose between light and dark mode."
        >
            <div className="flex space-x-2 p-1 bg-gray-200 dark:bg-gray-700 rounded-lg">
                <button 
                    onClick={() => setTheme('light')}
                    className={`px-4 py-1 rounded-md text-sm font-semibold transition-colors ${currentTheme === 'light' ? 'bg-white shadow text-cyan-600' : 'text-gray-600 dark:text-gray-300'}`}
                >
                    Light
                </button>
                <button 
                    onClick={() => setTheme('dark')}
                    className={`px-4 py-1 rounded-md text-sm font-semibold transition-colors ${currentTheme === 'dark' ? 'bg-gray-800 shadow text-cyan-400' : 'text-gray-600 dark:text-gray-300'}`}
                >
                    Dark
                </button>
            </div>
        </SettingRow>
         <SettingRow
            title="Primary Currency"
            description="Select the currency for displaying converted balances."
        >
            <select className="bg-gray-200 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg p-2 focus:ring-2 focus:ring-cyan-500 focus:outline-none">
                <option>USD</option>
                <option>EUR</option>
                <option>BTC</option>
            </select>
        </SettingRow>
      </Card>

      {/* Pools Management */}
      <Card>
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">Pools Management</h2>
         <SettingRow
            title="BitPool API Key"
            description="API key for fetching balance information."
        >
            <input type="text" placeholder="Enter API Key" className="w-full sm:w-64 bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-white px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-cyan-500"/>
        </SettingRow>
        <div className="mt-4">
            <button className="bg-cyan-500 hover:bg-cyan-600 text-white font-bold py-2 px-4 rounded-lg transition-colors">
                Save Changes
            </button>
        </div>
      </Card>
    </div>
  );
};

export default SettingsPage;
