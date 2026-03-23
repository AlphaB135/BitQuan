import React, { useState, useEffect } from 'react';
import Card from '../components/Card';
import { Miner, Balance } from '../types';
import { ActivityIcon, TrendingUpIcon, ZapIcon } from '../components/icons';
import { invoke } from '@tauri-apps/api/tauri';

const DashboardPage: React.FC = () => {
  const [miners, setMiners] = useState<Miner[]>([]);
  const [balances, setBalances] = useState<Balance[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentTime, setCurrentTime] = useState(new Date());

  useEffect(() => {
    const fetchData = async () => {
      try {
        const [minersData, balancesData] = await Promise.all([
          invoke<Miner[]>('get_miners'),
          invoke<Balance[]>('get_balances')
        ]);
        setMiners(minersData);
        setBalances(balancesData);
      } catch (error) {
        console.error('Failed to fetch data:', error);
      } finally {
        setLoading(false);
      }
    };

    fetchData();
    
    // Update time every second for real-time feel
    const timeInterval = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);
    
    return () => clearInterval(timeInterval);
  }, []);

  const totalBalance = balances.reduce((acc, curr) => ({
      bq: acc.bq + curr.bq,
      btc: acc.btc + curr.btc,
      usd: acc.usd + curr.usd,
  }), { bq: 0, btc: 0, usd: 0 });
  
  // Calculate mining statistics
  const activeMiners = miners.filter(m => m.profit > 0);
  const totalHashrate = activeMiners.reduce((sum, m) => {
    const hashValue = parseFloat(m.speed.split(' ')[0]);
    return sum + hashValue;
  }, 0);
  const totalProfit = miners.reduce((sum, m) => sum + m.profit, 0);
  const avgProfit = activeMiners.length > 0 ? totalProfit / activeMiners.length : 0;

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-500 dark:text-gray-400">Loading...</div>
      </div>
    );
  }
  return (
    <div className="space-y-8">
      {/* Header with real-time status */}
      <div className="flex justify-between items-center">
        <h1 className="text-3xl font-bold text-gray-900 dark:text-white">Mining Dashboard</h1>
        <div className="text-sm text-gray-500 dark:text-gray-400">
          Last updated: {currentTime.toLocaleTimeString()}
        </div>
      </div>
      
      {/* Mining Stats Overview */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <Card className="bg-gradient-to-br from-cyan-50 to-cyan-100 dark:from-cyan-900/20 dark:to-cyan-800/30 border-cyan-200 dark:border-cyan-700">
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-lg font-semibold text-cyan-700 dark:text-cyan-300">Active Miners</h3>
            <ActivityIcon />
          </div>
          <p className="text-3xl font-bold text-cyan-800 dark:text-cyan-200">{activeMiners.length}</p>
          <p className="text-sm text-cyan-600 dark:text-cyan-400">of {miners.length} total</p>
        </Card>
        
        <Card className="bg-gradient-to-br from-green-50 to-green-100 dark:from-green-900/20 dark:to-green-800/30 border-green-200 dark:border-green-700">
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-lg font-semibold text-green-700 dark:text-green-300">Total Hashrate</h3>
            <ZapIcon />
          </div>
          <p className="text-3xl font-bold text-green-800 dark:text-green-200">{totalHashrate.toFixed(1)}</p>
          <p className="text-sm text-green-600 dark:text-green-400">MH/s combined</p>
        </Card>
        
        <Card className="bg-gradient-to-br from-purple-50 to-purple-100 dark:from-purple-900/20 dark:to-purple-800/30 border-purple-200 dark:border-purple-700">
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-lg font-semibold text-purple-700 dark:text-purple-300">Daily Profit</h3>
            <TrendingUpIcon />
          </div>
          <p className="text-3xl font-bold text-purple-800 dark:text-purple-200">+{totalProfit.toFixed(2)}</p>
          <p className="text-sm text-purple-600 dark:text-purple-400">BQ per day</p>
        </Card>
        
        <Card className="bg-gradient-to-br from-orange-50 to-orange-100 dark:from-orange-900/20 dark:to-orange-800/30 border-orange-200 dark:border-orange-700">
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-lg font-semibold text-orange-700 dark:text-orange-300">Avg Profit/Miner</h3>
            <div className="w-6 h-6 bg-orange-500 rounded-full flex items-center justify-center">
              <span className="text-white text-xs font-bold">Ø</span>
            </div>
          </div>
          <p className="text-3xl font-bold text-orange-800 dark:text-orange-200">{avgProfit.toFixed(2)}</p>
          <p className="text-sm text-orange-600 dark:text-orange-400">BQ per miner</p>
        </Card>
      </div>

      {/* Balances Section */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {balances.map((balance) => (
          <Card key={balance.pool}>
            <h3 className="text-xl font-semibold text-cyan-500 dark:text-cyan-400 mb-4">{balance.pool} Balance</h3>
            <div className="space-y-2 text-lg">
              <p><span className="font-bold">{balance.bq.toFixed(2)}</span> BQ</p>
              <p className="text-sm text-gray-500 dark:text-gray-400">{balance.btc.toFixed(4)} BTC</p>
              <p className="text-sm text-gray-500 dark:text-gray-400">${balance.usd.toFixed(2)} USD</p>
            </div>
          </Card>
        ))}
        <Card className="bg-cyan-500/10 dark:bg-cyan-500/20 border border-cyan-500">
          <h3 className="text-xl font-semibold text-cyan-500 dark:text-cyan-400 mb-4">Total Balance From All Pools</h3>
          <div className="space-y-2 text-lg">
              <p><span className="font-bold">{totalBalance.bq.toFixed(2)}</span> BQ</p>
              <p className="text-sm text-gray-500 dark:text-gray-400">{totalBalance.btc.toFixed(4)} BTC</p>
              <p className="text-sm text-gray-500 dark:text-gray-400">${totalBalance.usd.toFixed(2)} USD</p>
          </div>
        </Card>
      </div>

      {/* Running Miners Section */}
      <Card>
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">Running Miners</h2>
        <div className="overflow-x-auto">
          <table className="w-full text-left">
            <thead className="border-b border-gray-200 dark:border-gray-600 text-gray-600 dark:text-gray-400 uppercase text-xs">
              <tr>
                <th className="p-3 font-semibold">Name</th>
                <th className="p-3 font-semibold">Pool</th>
                <th className="p-3 font-semibold">Devices</th>
                <th className="p-3 font-semibold">Profit (BQ/day)</th>
                <th className="p-3 font-semibold">Algorithm</th>
                <th className="p-3 font-semibold">Speed</th>
              </tr>
            </thead>
            <tbody>
              {miners.map((miner) => (
                <tr key={miner.id} className="border-b border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-700/50">
                  <td className="p-3 font-medium text-gray-900 dark:text-gray-100">{miner.name}</td>
                  <td className="p-3 text-gray-700 dark:text-gray-300">{miner.pool}</td>
                  <td className="p-3 text-gray-700 dark:text-gray-300">{miner.devices}</td>
                  <td className="p-3 text-green-500 dark:text-green-400 font-semibold">+{miner.profit.toFixed(2)}</td>
                  <td className="p-3 text-gray-700 dark:text-gray-300">{miner.algo}</td>
                  <td className="p-3 font-mono text-gray-700 dark:text-gray-300">{miner.speed}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
};

export default DashboardPage;