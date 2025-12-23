//! Hardware wallet integration for BitQuan

use crate::{address::Address, psbt::PQPSBT, Result, SDKError};

#[cfg(feature = "hardware")]
use pqc_dilithium_seeded::{PUBLICKEYBYTES, SIGNBYTES};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[cfg(feature = "hardware")]
use crate::psbt::PSBTError;
#[cfg(feature = "hardware")]
use std::str::FromStr;

/// Hardware wallet errors
#[derive(Debug, Error)]
pub enum HardwareError {
    /// Device not found
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    /// Communication error
    #[error("Communication error: {0}")]
    CommunicationError(String),

    /// Invalid response
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Operation cancelled
    #[error("Operation cancelled by user")]
    OperationCancelled,

    /// Unsupported operation
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    /// Device locked
    #[error("Device is locked")]
    DeviceLocked,

    /// Firmware version too old
    #[error("Firmware version too old: {0}")]
    FirmwareTooOld(String),

    /// Operation failed
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

/// Device capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// Supports Dilithium signatures
    pub supports_dilithium: bool,
    /// Supports ECDSA fallback
    pub supports_ecdsa: bool,
    /// Has secure display
    pub has_display: bool,
    /// Has physical buttons
    pub has_buttons: bool,
    /// Maximum message size
    pub max_message_size: usize,
    /// Firmware version
    pub firmware_version: String,
    /// Device model
    pub device_model: String,
    /// Serial number
    pub serial_number: String,
}

/// Hardware wallet command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    /// Get device info
    GetInfo = 0x01,
    /// Get public key
    GetPublicKey = 0x02,
    /// Sign transaction
    SignTransaction = 0x03,
    /// Sign message
    SignMessage = 0x04,
    /// Backup wallet
    BackupWallet = 0x05,
    /// Restore wallet
    RestoreWallet = 0x06,
    /// Wipe device
    WipeDevice = 0x07,
    /// Get address
    GetAddress = 0x08,
    /// Display address on screen
    DisplayAddress = 0x09,
    /// Confirm transaction
    ConfirmTransaction = 0x0A,
}

/// Hardware wallet response status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResponseStatus {
    /// Success
    Success = 0x00,
    /// Invalid parameter
    InvalidParameter = 0x01,
    /// Operation failed
    OperationFailed = 0x02,
    /// User cancelled
    UserCancelled = 0x03,
    /// Device locked
    DeviceLocked = 0x04,
    /// Not supported
    NotSupported = 0x05,
    /// Busy
    Busy = 0x06,
}

impl From<u8> for ResponseStatus {
    fn from(byte: u8) -> Self {
        match byte {
            0x00 => ResponseStatus::Success,
            0x01 => ResponseStatus::InvalidParameter,
            0x02 => ResponseStatus::OperationFailed,
            0x03 => ResponseStatus::UserCancelled,
            0x04 => ResponseStatus::DeviceLocked,
            0x05 => ResponseStatus::NotSupported,
            0x06 => ResponseStatus::Busy,
            _ => ResponseStatus::OperationFailed,
        }
    }
}

/// Hardware wallet interface
pub trait HardwareWallet {
    /// Get device capabilities
    fn get_capabilities(&self) -> Result<DeviceCapabilities>;

    /// Get public key at derivation path
    fn get_public_key(&self, derivation_path: &str) -> Result<Vec<u8>>;

    /// Get address at derivation path
    fn get_address(&self, derivation_path: &str, display: bool) -> Result<Address>;

    /// Sign transaction
    fn sign_transaction(&self, psbt: &mut PQPSBT) -> Result<()>;

    /// Sign message
    fn sign_message(&self, message: &[u8], derivation_path: &str) -> Result<Vec<u8>>;

    /// Backup wallet
    fn backup_wallet(&self) -> Result<Vec<u8>>;

    /// Restore wallet
    fn restore_wallet(&self, backup_data: &[u8]) -> Result<()>;

    /// Wipe device
    fn wipe_device(&self) -> Result<()>;

    /// Check if device is locked
    fn is_locked(&self) -> Result<bool>;

    /// Unlock device
    fn unlock(&self, pin: &str) -> Result<()>;

    /// Get device info
    fn get_device_info(&self) -> Result<DeviceInfo>;
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Manufacturer
    pub manufacturer: String,
    /// Product name
    pub product_name: String,
    /// Serial number
    pub serial_number: String,
    /// Firmware version
    pub firmware_version: String,
    /// Capabilities
    pub capabilities: DeviceCapabilities,
}

/// USB HID hardware wallet implementation
#[cfg(feature = "hardware")]
pub struct USBHardwareWallet {
    /// Device handle
    device: hidapi::HidDevice,
    /// Device info
    device_info: DeviceInfo,
    /// Communication timeout
    timeout: std::time::Duration,
}

#[cfg(feature = "hardware")]
impl USBHardwareWallet {
    /// Create new USB hardware wallet
    pub fn new(vendor_id: u16, product_id: u16) -> Result<Self> {
        let api = hidapi::HidApi::new()
            .map_err(|e| SDKError::Hardware(HardwareError::DeviceNotFound(e.to_string())))?;

        let device = api
            .open(vendor_id, product_id)
            .map_err(|e| SDKError::Hardware(HardwareError::DeviceNotFound(e.to_string())))?;

        // Get device info
        let device_info = Self::get_device_info(&device)?;

        Ok(Self {
            device,
            device_info,
            timeout: std::time::Duration::from_secs(10),
        })
    }

    /// List available devices
    pub fn list_devices() -> Result<Vec<DeviceInfo>> {
        let api = hidapi::HidApi::new()
            .map_err(|e| SDKError::Hardware(HardwareError::CommunicationError(e.to_string())))?;

        let mut devices = vec![];

        for device_info in api.device_list() {
            if let Ok(device) = device_info.open_device(&api) {
                if let Ok(info) = Self::get_device_info(&device) {
                    devices.push(info);
                }
            }
        }

        Ok(devices)
    }

    /// Send command to device
    fn send_command(&self, command: Command, data: &[u8]) -> Result<Vec<u8>> {
        let mut packet = vec![command as u8];
        packet.extend_from_slice(data);

        // Send packet
        self.device
            .write(&packet)
            .map_err(|e| SDKError::Hardware(HardwareError::CommunicationError(e.to_string())))?;

        // Read response
        let mut response = vec![0u8; 4096];
        let bytes_read = self
            .device
            .read_timeout(&mut response, self.timeout.as_millis() as i32)
            .map_err(|e| SDKError::Hardware(HardwareError::CommunicationError(e.to_string())))?;

        response.truncate(bytes_read);

        // Check status
        if response.is_empty() {
            return Err(SDKError::Hardware(HardwareError::InvalidResponse(
                "Empty response".to_string(),
            )));
        }

        let status = ResponseStatus::from(response[0]);
        match status {
            ResponseStatus::Success => Ok(response[1..].to_vec()),
            ResponseStatus::UserCancelled => {
                Err(SDKError::Hardware(HardwareError::OperationCancelled))
            }
            ResponseStatus::DeviceLocked => Err(SDKError::Hardware(HardwareError::DeviceLocked)),
            ResponseStatus::NotSupported => Err(SDKError::Hardware(
                HardwareError::UnsupportedOperation("Command not supported".to_string()),
            )),
            _ => Err(SDKError::Hardware(HardwareError::OperationFailed(format!(
                "Command failed with status: {:?}",
                status
            )))),
        }
    }

    /// Get device info from device
    fn get_device_info(device: &hidapi::HidDevice) -> Result<DeviceInfo> {
        // Send GetInfo command
        let mut buffer = [0u8; 64];
        buffer[0] = Command::GetInfo as u8;
        let response = device
            .get_feature_report(&mut buffer)
            .map_err(|e| SDKError::Hardware(HardwareError::CommunicationError(e.to_string())))?;

        if response < 10 {
            return Err(SDKError::Hardware(HardwareError::InvalidResponse(
                "Too short response".to_string(),
            )));
        }

        // Parse response (simplified)
        let vendor_id = u16::from_le_bytes([buffer[1], buffer[2]]);
        let product_id = u16::from_le_bytes([buffer[3], buffer[4]]);

        let capabilities = DeviceCapabilities {
            supports_dilithium: buffer[5] & 0x01 != 0,
            supports_ecdsa: buffer[5] & 0x02 != 0,
            has_display: buffer[5] & 0x04 != 0,
            has_buttons: buffer[5] & 0x08 != 0,
            max_message_size: u16::from_le_bytes([buffer[6], buffer[7]]) as usize,
            firmware_version: format!("{}.{}.{}", buffer[8], buffer[9], buffer[10]),
            device_model: "BitQuan Hardware".to_string(),
            serial_number: "Unknown".to_string(),
        };

        Ok(DeviceInfo {
            vendor_id,
            product_id,
            manufacturer: "BitQuan".to_string(),
            product_name: "Hardware Wallet".to_string(),
            serial_number: "Unknown".to_string(),
            firmware_version: capabilities.firmware_version.clone(),
            capabilities,
        })
    }
}

#[cfg(feature = "hardware")]
impl HardwareWallet for USBHardwareWallet {
    fn get_capabilities(&self) -> Result<DeviceCapabilities> {
        Ok(self.device_info.capabilities.clone())
    }

    fn get_public_key(&self, derivation_path: &str) -> Result<Vec<u8>> {
        let path_bytes = derivation_path.as_bytes();
        let response = self.send_command(Command::GetPublicKey, path_bytes)?;

        if response.len() < PUBLICKEYBYTES {
            return Err(SDKError::Hardware(HardwareError::InvalidResponse(
                "Invalid public key length".to_string(),
            )));
        }

        Ok(response[..PUBLICKEYBYTES].to_vec())
    }

    fn get_address(&self, derivation_path: &str, display: bool) -> Result<Address> {
        let mut data = vec![if display { 1 } else { 0 }];
        data.extend_from_slice(derivation_path.as_bytes());

        let response = self.send_command(Command::GetAddress, &data)?;

        let address_str = String::from_utf8(response)
            .map_err(|e| SDKError::Hardware(HardwareError::InvalidResponse(e.to_string())))?;

        Address::from_str(&address_str)
    }

    fn sign_transaction(&self, psbt: &mut PQPSBT) -> Result<()> {
        let psbt_bytes = <PQPSBT>::serialize(psbt)
            .map_err(|e| SDKError::PSBT(PSBTError::Serialization(e.to_string())))?;
        let response = self.send_command(Command::SignTransaction, &psbt_bytes)?;

        // Parse response and update PSBT with signatures
        // This is a simplified implementation
        for (i, input) in psbt.inputs.iter_mut().enumerate() {
            if input.get_dilithium_signature().is_none() {
                // Extract signature from response (simplified)
                let sig_start = i * SIGNBYTES;
                if sig_start + SIGNBYTES <= response.len() {
                    let mut signature = [0u8; SIGNBYTES];
                    signature.copy_from_slice(&response[sig_start..sig_start + SIGNBYTES]);
                    input.set_dilithium_signature(signature);
                }
            }
        }

        Ok(())
    }

    fn sign_message(&self, message: &[u8], derivation_path: &str) -> Result<Vec<u8>> {
        let mut data = vec![];
        data.extend_from_slice(&(message.len() as u32).to_le_bytes());
        data.extend_from_slice(message);
        data.extend_from_slice(derivation_path.as_bytes());

        let response = self.send_command(Command::SignMessage, &data)?;

        if response.len() < SIGNBYTES {
            return Err(SDKError::Hardware(HardwareError::InvalidResponse(
                "Invalid signature length".to_string(),
            )));
        }

        Ok(response[..SIGNBYTES].to_vec())
    }

    fn backup_wallet(&self) -> Result<Vec<u8>> {
        let response = self.send_command(Command::BackupWallet, &[])?;
        Ok(response)
    }

    fn restore_wallet(&self, backup_data: &[u8]) -> Result<()> {
        let _response = self.send_command(Command::RestoreWallet, backup_data)?;
        Ok(())
    }

    fn wipe_device(&self) -> Result<()> {
        let _response = self.send_command(Command::WipeDevice, &[])?;
        Ok(())
    }

    fn is_locked(&self) -> Result<bool> {
        let response = self.send_command(Command::GetInfo, &[])?;

        if response.is_empty() {
            return Err(SDKError::Hardware(HardwareError::InvalidResponse(
                "Empty response".to_string(),
            )));
        }

        Ok(response[0] == 0x01) // Assume second byte indicates lock status
    }

    fn unlock(&self, pin: &str) -> Result<()> {
        let mut data = vec![];
        data.extend_from_slice(&(pin.len() as u32).to_le_bytes());
        data.extend_from_slice(pin.as_bytes());

        let _response = self.send_command(Command::GetInfo, &data)?;
        Ok(())
    }

    fn get_device_info(&self) -> Result<DeviceInfo> {
        Ok(self.device_info.clone())
    }
}

/// Hardware wallet manager
pub struct HardwareWalletManager {
    /// Connected devices
    devices: HashMap<String, Box<dyn HardwareWallet>>,
}

impl HardwareWalletManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    /// Scan for devices
    pub fn scan_devices(&mut self) -> Result<Vec<DeviceInfo>> {
        #[cfg(feature = "hardware")]
        {
            let devices = USBHardwareWallet::list_devices()?;

            // Connect to new devices
            for device_info in &devices {
                let key = format!("{}:{}", device_info.vendor_id, device_info.product_id);
                if let std::collections::hash_map::Entry::Vacant(e) = self.devices.entry(key) {
                    if let Ok(wallet) =
                        USBHardwareWallet::new(device_info.vendor_id, device_info.product_id)
                    {
                        e.insert(Box::new(wallet));
                    }
                }
            }

            Ok(devices)
        }

        #[cfg(not(feature = "hardware"))]
        {
            Err(SDKError::Hardware(HardwareError::UnsupportedOperation(
                "Hardware wallet support not enabled".to_string(),
            )))
        }
    }

    /// Get device by serial number
    pub fn find_device(&self, _serial_number: &str) -> Option<&dyn HardwareWallet> {
        self.devices
            .values()
            .find(|_d| {
                // This would need proper implementation
                false // Placeholder
            })
            .map(|d| d.as_ref())
    }

    /// Get all devices
    pub fn get_devices(&self) -> Vec<&dyn HardwareWallet> {
        self.devices.values().map(|d| d.as_ref()).collect()
    }

    /// Disconnect device
    pub fn disconnect_device(&mut self, serial_number: &str) -> Result<()> {
        let key = serial_number.to_string();
        if self.devices.remove(&key).is_some() {
            Ok(())
        } else {
            Err(SDKError::Hardware(HardwareError::DeviceNotFound(
                "Device not found".to_string(),
            )))
        }
    }
}

impl Default for HardwareWalletManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_capabilities() {
        let capabilities = DeviceCapabilities {
            supports_dilithium: true,
            supports_ecdsa: true,
            has_display: true,
            has_buttons: true,
            max_message_size: 4096,
            firmware_version: "1.0.0".to_string(),
            device_model: "BitQuan Pro".to_string(),
            serial_number: "12345678".to_string(),
        };

        assert!(capabilities.supports_dilithium);
        assert!(capabilities.has_display);
    }

    #[test]
    fn test_hardware_wallet_manager() {
        let manager = HardwareWalletManager::new();
        assert_eq!(manager.get_devices().len(), 0);
    }

    #[test]
    fn test_response_status() {
        assert_eq!(ResponseStatus::Success as u8, 0x00);
        assert_eq!(ResponseStatus::UserCancelled as u8, 0x03);
        assert_eq!(ResponseStatus::DeviceLocked as u8, 0x04);
    }
}
