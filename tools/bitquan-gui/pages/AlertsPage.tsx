import React, { useState, useEffect } from 'react';
import Card from '../components/Card';
import { Alert } from '../types';
import { BellIcon } from '../components/icons';
import { invoke } from '@tauri-apps/api/tauri';

const AlertItem: React.FC<{ alert: Alert }> = ({ alert }) => {
    const typeClasses = {
        info: {
            bg: 'bg-blue-500/10 dark:bg-blue-500/20',
            border: 'border-blue-500',
            text: 'text-blue-600 dark:text-blue-400',
        },
        warning: {
            bg: 'bg-yellow-500/10 dark:bg-yellow-500/20',
            border: 'border-yellow-500',
            text: 'text-yellow-600 dark:text-yellow-400',
        },
        error: {
            bg: 'bg-red-500/10 dark:bg-red-500/20',
            border: 'border-red-500',
            text: 'text-red-600 dark:text-red-400',
        }
    };
    
    const classes = typeClasses[alert.type];

    return (
        <div className={`p-4 rounded-lg flex items-start gap-4 border-l-4 ${classes.bg} ${classes.border}`}>
            <div className={`mt-1 ${classes.text}`}>
                <BellIcon/>
            </div>
            <div className="flex-1">
                <p className="font-semibold text-gray-800 dark:text-gray-100">{alert.message}</p>
                <p className="text-sm text-gray-500 dark:text-gray-400">{alert.timestamp}</p>
            </div>
            <button className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 text-xl">&times;</button>
        </div>
    );
}

const AlertsPage: React.FC = () => {
    const [alerts, setAlerts] = useState<Alert[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        const fetchAlerts = async () => {
            try {
                const alertsData = await invoke<Alert[]>('get_alerts');
                setAlerts(alertsData);
            } catch (error) {
                console.error('Failed to fetch alerts:', error);
            } finally {
                setLoading(false);
            }
        };

        fetchAlerts();
    }, []);

    if (loading) {
        return (
            <div className="flex items-center justify-center h-64">
                <div className="text-gray-500 dark:text-gray-400">Loading...</div>
            </div>
        );
    }

    return (
        <div className="space-y-8">
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white">🔔 Alerts Log</h1>
            <Card>
                <div className="space-y-4">
                    {alerts.map(alert => (
                        <AlertItem key={alert.id} alert={alert} />
                    ))}
                </div>
            </Card>
        </div>
    );
};

export default AlertsPage;
