import React, { useState, useEffect } from 'react';
import Card from '../components/Card';
import { Miner, Balance } from '../types';
import { invoke } from '@tauri-apps/api';

const DashboardPage: React.FC = () => {
  const [miners, setMiners] = useState<Miner[]>([]);
  const [balances, setBalances] = useState<Balance[]>([]);
  const [loading, setLoading] = useState(true);

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
  }, []);

  const totalBalance = balances.reduce((acc, curr) => ({
      bq: acc.bq + curr.bq,
      btc: acc.btc + curr.btc,
      usd: acc.usd + curr.usd,
  }), { bq: 0, btc: 0, usd: 0 });

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex flex-col items-center space-y-4">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-cyan-500"></div>
          <div className="text-gray-500 dark:text-gray-400">Loading dashboard data...</div>
        </div>
      </div>
    );
  }
  return (
    <div className="space-y-8">
      <h1 className="text-3xl font-bold text-gray-900 dark:text-white">Dashboard</h1>
      
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
          <table className="w-full text-left min-w-[500px]">
            <thead className="border-b border-gray-200 dark:border-gray-600 text-gray-600 dark:text-gray-400 uppercase text-xs">
              <tr>
                <th className="p-2 md:p-3 font-semibold">Name</th>
                <th className="p-2 md:p-3 font-semibold">Pool</th>
                <th className="p-2 md:p-3 font-semibold">Devices</th>
                <th className="p-2 md:p-3 font-semibold">Profit</th>
                <th className="p-2 md:p-3 font-semibold">Algorithm</th>
                <th className="p-2 md:p-3 font-semibold">Speed</th>
              </tr>
            </thead>
            <tbody>
              {miners.map((miner) => (
                <tr key={miner.id} className="border-b border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-700/50">
                  <td className="p-2 md:p-3 font-medium text-gray-900 dark:text-gray-100 text-sm">{miner.name}</td>
                  <td className="p-2 md:p-3 text-gray-700 dark:text-gray-300 text-sm">{miner.pool}</td>
                  <td className="p-2 md:p-3 text-gray-700 dark:text-gray-300 text-sm">{miner.devices}</td>
                  <td className="p-2 md:p-3 text-green-500 dark:text-green-400 font-semibold text-sm">+{miner.profit.toFixed(2)}</td>
                  <td className="p-2 md:p-3 text-gray-700 dark:text-gray-300 text-sm">{miner.algo}</td>
                  <td className="p-2 md:p-3 font-mono text-gray-700 dark:text-gray-300 text-sm">{miner.speed}</td>
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