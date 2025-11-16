export interface Miner {
  id: number;
  name: string;
  pool: string;
  devices: string;
  profit: number;
  algo: string;
  speed: string;
}

export interface Balance {
  pool: string;
  bq: number;
  btc: number;
  usd: number;
}

export interface Rig {
  id: number;
  name: string;
  is_active: boolean;
  device_type: string;
  temp: number;
  power: number;
  hashrate: number;
  hashrate_unit: string;
  algorithm: string;
  miner_version: string;
  earnings: number;
}

export interface Transaction {
    id: string;
    type: 'sent' | 'received';
    date: string;
    address: string;
    amount: number;
}

export interface Alert {
  id: string;
  type: 'info' | 'warning' | 'error';
  message: string;
  timestamp: string;
}