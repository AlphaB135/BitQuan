import React, { useState, useEffect } from 'react';
import Card from '../components/Card';
import { Alert } from '../types';
import { BellIcon } from '../components/icons';
import { invoke } from '@tauri-apps/api';

const AlertItem: React.FC<{ alert: Alert; onClose?: (id: string) => void }> = ({ alert, onClose }) => {
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

    const handleClose = () => {
        if (onClose) {
            onClose(alert.id);
        }
    };

    return (
        <div className={`p-4 rounded-lg flex items-start gap-4 border-l-4 ${classes.bg} ${classes.border}`}>
            <div className={`mt-1 ${classes.text}`}>
                <BellIcon/>
            </div>
            <div className="flex-1">
                <p className="font-semibold text-gray-800 dark:text-gray-100">{alert.message}</p>
                <p className="text-sm text-gray-500 dark:text-gray-400">{alert.timestamp}</p>
            </div>
            <button 
                onClick={handleClose}
                className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 text-xl leading-none"
                aria-label="Close alert"
            >
                &times;
            </button>
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

    const handleCloseAlert = (id: string) => {
        setAlerts(prevAlerts => prevAlerts.filter(alert => alert.id !== id));
    };

    const handleClearAll = () => {
        setAlerts([]);
    };

    if (loading) {
        return (
            <div className="flex items-center justify-center h-64">
                <div className="flex flex-col items-center space-y-4">
                    <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-cyan-500"></div>
                    <div className="text-gray-500 dark:text-gray-400">Loading alerts...</div>
                </div>
            </div>
        );
    }

    return (
        <div className="space-y-8">
            <div className="flex justify-between items-center">
                <h1 className="text-3xl font-bold text-gray-900 dark:text-white">🔔 Alerts Log</h1>
                {alerts.length > 0 && (
                    <button 
                        onClick={handleClearAll}
                        className="text-sm text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 transition-colors"
                    >
                        Clear All
                    </button>
                )}
            </div>
            <Card>
                {alerts.length === 0 ? (
                    <div className="text-center py-8">
                        <BellIcon />
                        <p className="mt-4 text-gray-500 dark:text-gray-400">No alerts at this time</p>
                    </div>
                ) : (
                    <div className="space-y-4">
                        {alerts.map(alert => (
                            <AlertItem key={alert.id} alert={alert} onClose={handleCloseAlert} />
                        ))}
                    </div>
                )}
            </Card>
        </div>
    );
};

export default AlertsPage;
