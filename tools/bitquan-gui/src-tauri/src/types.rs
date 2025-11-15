use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Miner {
    pub id: u32,
    pub name: String,
    pub pool: String,
    pub devices: String,
    pub profit: f64,
    pub algo: String,
    pub speed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub pool: String,
    pub bq: f64,
    pub btc: f64,
    pub usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rig {
    pub id: u32,
    pub name: String,
    pub is_active: bool,
    pub device_type: String,
    pub temp: f64,
    pub power: f64,
    pub hashrate: f64,
    pub hashrate_unit: String,
    pub algorithm: String,
    pub miner_version: String,
    pub earnings: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub date: String,
    pub address: String,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    #[serde(rename = "type")]
    pub alert_type: String,
    pub message: String,
    pub timestamp: String,
}