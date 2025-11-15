import React, { useState, useEffect } from 'react';
import Card from '../components/Card';
import ToggleSwitch from '../components/ToggleSwitch';
import { Rig } from '../types';
import { ThermometerIcon, BoltIcon, CpuChipIcon } from '../components/icons';
import { invoke } from '@tauri-apps/api';

const RigCard: React.FC<{ rig: Rig; onToggle: (id: number, isActive: boolean) => void }> = ({ rig, onToggle }) => {
  const statusColor = rig.is_active ? 'border-l-4 border-cyan-500' : 'border-l-4 border-gray-300 dark:border-gray-600';
  const textColor = rig.is_active ? 'text-gray-900 dark:text-white' : 'text-gray-500 dark:text-gray-400';

  return (
    <Card className={`transition-all duration-300 ${statusColor} ${!rig.is_active ? 'bg-gray-50 dark:bg-gray-800/50' : ''}`}>
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
        <div className="flex-1">
          <div className="flex items-center gap-4">
            <h3 className={`text-2xl font-bold ${textColor}`}>{rig.name}</h3>
            <span className={`px-3 py-1 text-xs font-semibold rounded-full ${rig.is_active ? 'bg-green-500/20 text-green-500 dark:text-green-400' : 'bg-red-500/20 text-red-500 dark:text-red-400'}`}>
              {rig.is_active ? 'Online' : 'Offline'}
            </span>
          </div>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">{rig.device_type} | {rig.algorithm} | Miner {rig.miner_version}</p>
        </div>
        <div className="flex items-center gap-4">
          <ToggleSwitch initialChecked={rig.is_active} onChange={(checked) => onToggle(rig.id, checked)} />
        </div>
      </div>
      
      {rig.is_active && (
        <div className="mt-6 grid grid-cols-2 md:grid-cols-4 gap-4 text-center">
            <div className="bg-gray-100 dark:bg-gray-700/50 p-3 rounded-lg">
                <p className="text-sm text-gray-500 dark:text-gray-400 flex items-center justify-center"><ThermometerIcon/> Temp</p>
                <p className="text-xl font-bold">{rig.temp}°C</p>
            </div>
            <div className="bg-gray-100 dark:bg-gray-700/50 p-3 rounded-lg">
                <p className="text-sm text-gray-500 dark:text-gray-400 flex items-center justify-center"><BoltIcon/> Power</p>
                <p className="text-xl font-bold">{rig.power}W</p>
            </div>
            <div className="bg-gray-100 dark:bg-gray-700/50 p-3 rounded-lg">
                <p className="text-sm text-gray-500 dark:text-gray-400 flex items-center justify-center"><CpuChipIcon/> Hashrate</p>
                <p className="text-xl font-bold">{rig.hashrate.toFixed(2)} <span className="text-base font-normal">{rig.hashrate_unit}</span></p>
            </div>
            <div className="bg-gray-100 dark:bg-gray-700/50 p-3 rounded-lg">
                <p className="text-sm text-gray-500 dark:text-gray-400">Earnings</p>
                <p className="text-xl font-bold text-green-500 dark:text-green-400">+{rig.earnings.toFixed(2)}</p>
            </div>
        </div>
      )}
    </Card>
  );
};

const RigsPage: React.FC = () => {
    const [rigs, setRigs] = useState<Rig[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        const fetchRigs = async () => {
            try {
                const rigsData = await invoke<Rig[]>('get_rigs');
                setRigs(rigsData);
            } catch (error) {
                console.error('Failed to fetch rigs:', error);
            } finally {
                setLoading(false);
            }
        };

        fetchRigs();
    }, []);

    const handleToggle = (id: number, isActive: boolean) => {
        setRigs(prevRigs => 
            prevRigs.map(rig => rig.id === id ? { ...rig, is_active: isActive } : rig)
        );
    };

    const activeRigs = rigs.filter(r => r.is_active);

    if (loading) {
        return (
            <div className="flex items-center justify-center h-64">
                <div className="flex flex-col items-center space-y-4">
                    <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-cyan-500"></div>
                    <div className="text-gray-500 dark:text-gray-400">Loading rigs...</div>
                </div>
            </div>
        );
    }
    const totalDailyEarnings = activeRigs.reduce((sum, rig) => sum + rig.earnings, 0);
    const totalMonthlyEarnings = totalDailyEarnings * 30;
    const totalBalance = 3821.25; // Mock data

  return (
    <div className="space-y-8">
      <h1 className="text-3xl font-bold text-gray-900 dark:text-white">Your Rigs/Nodes</h1>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <Card>
            <h2 className="text-lg font-semibold text-cyan-500 dark:text-cyan-400 mb-2">Overall Earnings</h2>
            <p className="text-2xl font-bold">{totalDailyEarnings.toFixed(2)} BQ/day</p>
            <p className="text-md text-gray-500 dark:text-gray-400">{totalMonthlyEarnings.toFixed(2)} BQ/month</p>
        </Card>
        <Card>
            <h2 className="text-lg font-semibold text-cyan-500 dark:text-cyan-400 mb-2">Total Wallet Balance</h2>
            <p className="text-2xl font-bold">{totalBalance.toFixed(2)} BQ</p>
            <p className="text-md text-gray-500 dark:text-gray-400">~${(totalBalance * 0.27).toFixed(2)} USD</p>
        </Card>
        <Card>
            <h2 className="text-lg font-semibold text-cyan-500 dark:text-cyan-400 mb-2">Active Rigs</h2>
            <p className="text-2xl font-bold">{activeRigs.length} <span className="text-lg font-normal text-gray-500 dark:text-gray-400">/ {rigs.length}</span></p>
            <p className="text-md text-gray-500 dark:text-gray-400">Currently Online</p>
        </Card>
      </div>
      
      <div className="space-y-6">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white">BitQuan Nodes</h2>
        {rigs.map(rig => (
            <RigCard key={rig.id} rig={rig} onToggle={handleToggle} />
        ))}
      </div>
    </div>
  );
};

export default RigsPage;