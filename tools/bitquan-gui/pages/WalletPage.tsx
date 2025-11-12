import React, { useState, useEffect } from 'react';
import Card from '../components/Card';
import { Transaction } from '../types';
import { invoke } from '@tauri-apps/api/core';

const QRCodePlaceholder: React.FC = () => (
    <div className="p-2 bg-white rounded-lg">
        <svg viewBox="0 0 100 100" className="w-full h-full text-gray-900">
            <path fill="currentColor" d="M0 0h30v30H0z M70 0h30v30H70z M0 70h30v30H0z M10 10h10v10H10z M80 10h10v10H80z M10 80h10v10H10z M40 0h20v10H40z M0 40h10v20H0z M90 40h10v20H90z M40 90h20v10H40z M40 40h20v20H40z M70 40h10v10H70z M40 70h10v10H40z M70 70h20v20H70z"/>
        </svg>
    </div>
);


const WalletPage: React.FC = () => {
    const [walletState, setWalletState] = useState({
        isCreated: false,
        address: 'N/A',
        publicKeyHash: 'N/A',
        createdDate: 'N/A',
        balance: 0,
    });
    const [transactions, setTransactions] = useState<Transaction[]>([]);
    const [activeTab, setActiveTab] = useState<'send' | 'receive'>('send');
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        const fetchTransactions = async () => {
            try {
                const transactionsData = await invoke<Transaction[]>('get_transactions');
                setTransactions(transactionsData);
                setWalletState({
                    isCreated: true,
                    address: 'BQ' + Math.random().toString(36).substring(2, 15),
                    publicKeyHash: Math.random().toString(36).substring(2, 15),
                    createdDate: new Date().toLocaleString(),
                    balance: 3821.25,
                });
            } catch (error) {
                console.error('Failed to fetch transactions:', error);
            } finally {
                setLoading(false);
            }
        };

        fetchTransactions();
    }, []);

    const handleCreateWallet = () => {
        setWalletState({
            isCreated: true,
            address: 'BQ' + Math.random().toString(36).substring(2, 15),
            publicKeyHash: Math.random().toString(36).substring(2, 15),
            createdDate: new Date().toLocaleString(),
            balance: 1000.00,
        });
    };

    if (loading) {
        return (
            <div className="flex items-center justify-center h-64">
                <div className="text-gray-500 dark:text-gray-400">Loading...</div>
            </div>
        );
    }
    
    const BQ_USD_RATE = 0.27;

    return (
        <div className="space-y-8">
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white">💰 Wallet</h1>

            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
                {/* Wallet Information Card */}
                <Card className="lg:col-span-2">
                    <h2 className="text-xl font-semibold text-cyan-500 dark:text-cyan-400 mb-4">Wallet Information</h2>
                    <div className="space-y-3 text-sm">
                        <div className="flex justify-between items-center">
                            <span className="text-gray-500 dark:text-gray-400">Status</span>
                            <span className={`font-semibold px-2 py-0.5 rounded ${walletState.isCreated ? 'bg-green-500/20 text-green-400' : 'bg-red-500/20 text-red-400'}`}>
                                {walletState.isCreated ? 'Created' : 'Not Created'}
                            </span>
                        </div>
                        <div className="flex justify-between items-center gap-4">
                            <span className="text-gray-500 dark:text-gray-400">Address</span>
                            <span className="font-mono text-xs md:text-sm truncate">{walletState.address}</span>
                        </div>
                        <div className="flex justify-between items-center gap-4">
                            <span className="text-gray-500 dark:text-gray-400">Public Key Hash</span>
                            <span className="font-mono text-xs md:text-sm truncate">{walletState.publicKeyHash}</span>
                        </div>
                        <div className="flex justify-between items-center">
                            <span className="text-gray-500 dark:text-gray-400">Created</span>
                            <span className="font-mono text-xs md:text-sm">{walletState.createdDate}</span>
                        </div>
                    </div>
                    <div className="mt-6 border-t border-gray-200 dark:border-gray-700 pt-4 flex space-x-4">
                        {!walletState.isCreated ? (
                            <button onClick={handleCreateWallet} className="w-full bg-cyan-500 hover:bg-cyan-600 text-white font-bold py-2 px-4 rounded-lg transition-colors">
                                Create Wallet
                            </button>
                        ) : (
                             <button className="w-full bg-gray-600 hover:bg-gray-700 text-white font-bold py-2 px-4 rounded-lg transition-colors">
                                Backup
                            </button>
                        )}
                    </div>
                </Card>

                {/* Balance Card */}
                <Card>
                    <h2 className="text-xl font-semibold text-cyan-500 dark:text-cyan-400 mb-4">Balance</h2>
                    <p className="text-4xl font-bold">{walletState.balance.toFixed(2)} BQ</p>
                    <p className="text-lg text-gray-500 dark:text-gray-400 mt-1">~${(walletState.balance * BQ_USD_RATE).toFixed(2)} USD</p>
                </Card>
            </div>

            {/* Send/Receive Card */}
            {walletState.isCreated && (
                <Card>
                    <div className="flex border-b border-gray-200 dark:border-gray-700 mb-4">
                        <button onClick={() => setActiveTab('send')} className={`px-4 py-2 font-semibold ${activeTab === 'send' ? 'text-cyan-500 dark:text-cyan-400 border-b-2 border-cyan-500 dark:border-cyan-400' : 'text-gray-500 dark:text-gray-400'}`}>Send</button>
                        <button onClick={() => setActiveTab('receive')} className={`px-4 py-2 font-semibold ${activeTab === 'receive' ? 'text-cyan-500 dark:text-cyan-400 border-b-2 border-cyan-500 dark:border-cyan-400' : 'text-gray-500 dark:text-gray-400'}`}>Receive</button>
                    </div>

                    {activeTab === 'send' && (
                        <div className="space-y-4">
                            <div>
                                <label className="block text-sm font-medium text-gray-600 dark:text-gray-400 mb-1">Recipient Address</label>
                                <input type="text" placeholder="Enter BQ address" className="w-full bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-white px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-cyan-500"/>
                            </div>
                            <div>
                                <label className="block text-sm font-medium text-gray-600 dark:text-gray-400 mb-1">Amount</label>
                                <input type="number" placeholder="0.00 BQ" className="w-full bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-white px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-cyan-500"/>
                            </div>
                            <button className="w-full bg-cyan-500 hover:bg-cyan-600 text-white font-bold py-2 px-4 rounded-lg transition-colors">
                                Send
                            </button>
                        </div>
                    )}

                    {activeTab === 'receive' && (
                        <div className="flex flex-col md:flex-row items-center gap-6">
                           <div className="w-40 h-40 flex-shrink-0">
                                <QRCodePlaceholder />
                           </div>
                           <div>
                                <h3 className="text-lg font-semibold text-gray-900 dark:text-white">Your Wallet Address</h3>
                                <p className="text-sm text-gray-500 dark:text-gray-400 mb-2">Share this address to receive BQ.</p>
                                <div className="bg-gray-100 dark:bg-gray-900 p-3 rounded-lg font-mono text-cyan-500 dark:text-cyan-400 break-all text-sm">
                                    {walletState.address}
                                </div>
                           </div>
                        </div>
                    )}
                </Card>
            )}

            {/* Transaction History Card */}
            {walletState.isCreated && (
                 <Card>
                    <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">Transaction History</h2>
                    <div className="overflow-x-auto">
                    <table className="w-full text-left">
                        <thead className="border-b border-gray-200 dark:border-gray-600 text-gray-500 dark:text-gray-400">
                        <tr>
                            <th className="p-3">Type</th>
                            <th className="p-3">Date</th>
                            <th className="p-3">Address</th>
                            <th className="p-3 text-right">Amount</th>
                        </tr>
                        </thead>
                        <tbody>
                        {transactions.map((tx) => (
                            <tr key={tx.id} className="border-b border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-700/50">
                                <td className="p-3">
                                    <span className={`px-2 py-1 text-xs font-semibold rounded-full ${tx.type === 'received' ? 'bg-green-500/20 text-green-400' : 'bg-yellow-500/20 text-yellow-400'}`}>
                                        {tx.type.charAt(0).toUpperCase() + tx.type.slice(1)}
                                    </span>
                                </td>
                                <td className="p-3 text-gray-500 dark:text-gray-400">{tx.date}</td>
                                <td className="p-3 font-mono text-sm">{tx.address}</td>
                                <td className={`p-3 text-right font-semibold ${tx.type === 'received' ? 'text-green-500 dark:text-green-400' : 'text-yellow-500 dark:text-yellow-400'}`}>
                                    {tx.type === 'received' ? '+' : '-'}{tx.amount.toFixed(2)} BQ
                                </td>
                            </tr>
                        ))}
                        </tbody>
                    </table>
                    </div>
                </Card>
            )}
        </div>
    );
};

export default WalletPage;