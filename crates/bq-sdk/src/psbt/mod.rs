#![allow(clippy::large_enum_variant, clippy::type_complexity)]
//! Post-Quantum PSBT (PQ-PSBT) implementation for BitQuan
//!
//! Extends Bitcoin PSBT with Dilithium signature support.

use crate::{address::Address, Result, SDKError};
use bitquan_types::Transaction;
use pqc_dilithium_seeded::{PUBLICKEYBYTES, SIGNBYTES};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::collections::BTreeMap;
use thiserror::Error;

/// PSBT errors
#[derive(Debug, Error)]
pub enum PSBTError {
    /// Invalid format
    #[error("Invalid PSBT format: {0}")]
    InvalidFormat(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid signature
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Version mismatch
    #[error("Version mismatch: expected {0}, got {1}")]
    VersionMismatch(u8, u8),

    /// Too many inputs
    #[error("Too many inputs: {0} > {1}")]
    TooManyInputs(usize, usize),

    /// Too many outputs
    #[error("Too many outputs: {0} > {1}")]
    TooManyOutputs(usize, usize),
}

/// PQ-PSBT magic bytes
pub const PQ_PSBT_MAGIC: &[u8; 4] = b"PQPS";

/// PQ-PSBT version
pub const PQ_PSBT_VERSION: u8 = 0x00;

/// Signature algorithm flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureFlags(u8);

impl SignatureFlags {
    /// Create new flags
    pub fn new() -> Self {
        Self(0)
    }

    /// Set Dilithium signature flag
    pub fn with_dilithium(mut self) -> Self {
        self.0 |= 0x01;
        self
    }

    /// Set ECDSA fallback flag
    pub fn with_ecdsa(mut self) -> Self {
        self.0 |= 0x02;
        self
    }

    /// Set hybrid mode (both signatures required)
    pub fn with_hybrid(mut self) -> Self {
        self.0 |= 0x04;
        self
    }

    /// Check if Dilithium signature is present
    pub fn has_dilithium(self) -> bool {
        self.0 & 0x01 != 0
    }

    /// Check if ECDSA fallback is present
    pub fn has_ecdsa(self) -> bool {
        self.0 & 0x02 != 0
    }

    /// Check if hybrid mode is required
    pub fn is_hybrid(self) -> bool {
        self.0 & 0x04 != 0
    }
}

impl Default for SignatureFlags {
    fn default() -> Self {
        Self::new().with_dilithium()
    }
}

/// Global PSBT keys
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq)]
pub enum GlobalKey {
    /// Transaction version (CompactSize)
    Version(u32),
    /// Fallback fingerprint (32 bytes)
    FallbackFingerprint(#[serde(with = "BigArray")] [u8; 32]),
    /// Locktime (CompactSize)
    Locktime(u32),
    /// Proprietary data
    Proprietary(Vec<u8>, Vec<u8>),
}

/// Input PSBT keys
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq)]
pub enum InputKey {
    /// Previous TXID (32 bytes)
    PreviousTxid(#[serde(with = "BigArray")] [u8; 32]),
    /// Previous output index (CompactSize)
    PreviousOutputIndex(u32),
    /// Sequence (8 bytes)
    Sequence(u32),
    /// ScriptSig (variable)
    ScriptSig(Vec<u8>),
    /// Dilithium public key
    DilithiumPublicKey(#[serde(with = "BigArray")] [u8; PUBLICKEYBYTES]),
    /// Dilithium signature
    DilithiumSignature(#[serde(with = "BigArray")] [u8; SIGNBYTES]),
    /// ECDSA fallback signature (variable)
    ECDSASignature(Vec<u8>),
    /// Proprietary data
    Proprietary(Vec<u8>, Vec<u8>),
}

/// Output PSBT keys
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq)]
pub enum OutputKey {
    /// Amount (16 bytes for u128)
    Amount(u128),
    /// ScriptPubkey (variable)
    ScriptPubkey(Vec<u8>),
    /// Proprietary data
    Proprietary(Vec<u8>, Vec<u8>),
}

/// PSBT input data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PSBTInput {
    /// Input data fields
    pub fields: BTreeMap<InputKey, Vec<u8>>,
}

impl PSBTInput {
    /// Create new input
    pub fn new() -> Self {
        Self::default()
    }

    /// Add field to input
    pub fn add_field(&mut self, key: InputKey, value: Vec<u8>) {
        self.fields.insert(key, value);
    }

    /// Get field from input
    pub fn get_field(&self, key: &InputKey) -> Option<&Vec<u8>> {
        self.fields.get(key)
    }

    /// Set previous TXID
    pub fn set_previous_txid(&mut self, txid: [u8; 32]) {
        self.add_field(InputKey::PreviousTxid(txid), txid.to_vec());
    }

    /// Set previous output index
    pub fn set_previous_output_index(&mut self, index: u32) {
        let mut bytes = vec![];
        bytes.extend_from_slice(&index.to_le_bytes());
        self.add_field(InputKey::PreviousOutputIndex(index), bytes);
    }

    /// Set sequence
    pub fn set_sequence(&mut self, sequence: u32) {
        let mut bytes = vec![];
        bytes.extend_from_slice(&sequence.to_le_bytes());
        self.add_field(InputKey::Sequence(sequence), bytes);
    }

    /// Set Dilithium public key
    pub fn set_dilithium_public_key(&mut self, pubkey: [u8; PUBLICKEYBYTES]) {
        self.add_field(InputKey::DilithiumPublicKey(pubkey), pubkey.to_vec());
    }

    /// Set Dilithium signature
    pub fn set_dilithium_signature(&mut self, signature: [u8; SIGNBYTES]) {
        self.add_field(InputKey::DilithiumSignature(signature), signature.to_vec());
    }

    /// Get Dilithium public key
    pub fn get_dilithium_public_key(&self) -> Option<[u8; PUBLICKEYBYTES]> {
        self.fields.iter().find_map(|(key, value)| {
            if let InputKey::DilithiumPublicKey(_pubkey) = key {
                if value.len() == PUBLICKEYBYTES {
                    let mut array = [0u8; PUBLICKEYBYTES];
                    array.copy_from_slice(value);
                    Some(array)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// Get Dilithium signature
    pub fn get_dilithium_signature(&self) -> Option<[u8; SIGNBYTES]> {
        self.fields.iter().find_map(|(key, value)| {
            if let InputKey::DilithiumSignature(_sig) = key {
                if value.len() == SIGNBYTES {
                    let mut array = [0u8; SIGNBYTES];
                    array.copy_from_slice(value);
                    Some(array)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
}

/// PSBT output data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PSBTOutput {
    /// Output data fields
    pub fields: BTreeMap<OutputKey, Vec<u8>>,
}

impl PSBTOutput {
    /// Create new output
    pub fn new() -> Self {
        Self::default()
    }

    /// Add field to output
    pub fn add_field(&mut self, key: OutputKey, value: Vec<u8>) {
        self.fields.insert(key, value);
    }

    /// Get field from output
    pub fn get_field(&self, key: &OutputKey) -> Option<&Vec<u8>> {
        self.fields.get(key)
    }

    /// Set amount
    pub fn set_amount(&mut self, amount: u128) {
        let mut bytes = vec![];
        bytes.extend_from_slice(&amount.to_le_bytes());
        self.add_field(OutputKey::Amount(amount), bytes);
    }

    /// Set script pubkey
    pub fn set_script_pubkey(&mut self, script: Vec<u8>) {
        self.add_field(OutputKey::ScriptPubkey(script.clone()), script);
    }

    /// Get amount
    pub fn get_amount(&self) -> Option<u128> {
        self.fields.iter().find_map(|(key, _value)| {
            if let OutputKey::Amount(amount) = key {
                Some(*amount)
            } else {
                None
            }
        })
    }

    /// Get script pubkey
    pub fn get_script_pubkey(&self) -> Option<Vec<u8>> {
        self.fields.iter().find_map(|(key, value)| {
            if let OutputKey::ScriptPubkey(_script) = key {
                Some(value.clone())
            } else {
                None
            }
        })
    }
}

/// Post-Quantum PSBT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PQPSBT {
    /// PSBT version
    pub version: u8,
    /// Signature flags
    pub signature_flags: SignatureFlags,
    /// Global data
    pub global: BTreeMap<GlobalKey, Vec<u8>>,
    /// Inputs
    pub inputs: Vec<PSBTInput>,
    /// Outputs
    pub outputs: Vec<PSBTOutput>,
}

impl PQPSBT {
    /// Create new PSBT
    pub fn new() -> Self {
        Self {
            version: PQ_PSBT_VERSION,
            signature_flags: SignatureFlags::default(),
            global: BTreeMap::new(),
            inputs: vec![],
            outputs: vec![],
        }
    }

    /// Create PSBT builder
    pub fn builder() -> PQPSBTBuilder {
        PQPSBTBuilder::new()
    }

    /// Serialize PSBT to bytes
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut buffer = vec![];

        // Write magic
        buffer.extend_from_slice(PQ_PSBT_MAGIC);

        // Write version
        buffer.push(self.version);

        // Write flags
        buffer.push(self.signature_flags.0);

        // Write global data
        self.serialize_map(&mut buffer, &self.global)?;

        // Write input count
        self.write_compact_size(&mut buffer, self.inputs.len())?;

        // Write inputs
        for input in &self.inputs {
            self.serialize_map(&mut buffer, &input.fields)?;
        }

        // Write output count
        self.write_compact_size(&mut buffer, self.outputs.len())?;

        // Write outputs
        for output in &self.outputs {
            self.serialize_map(&mut buffer, &output.fields)?;
        }

        Ok(buffer)
    }

    /// Deserialize PSBT from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 6 {
            return Err(SDKError::PSBT(PSBTError::InvalidFormat(
                "Too short".to_string(),
            )));
        }

        // Check magic
        if &data[0..4] != PQ_PSBT_MAGIC {
            return Err(SDKError::PSBT(PSBTError::InvalidFormat(
                "Invalid magic".to_string(),
            )));
        }

        let version = data[4];
        if version != PQ_PSBT_VERSION {
            return Err(SDKError::PSBT(PSBTError::VersionMismatch(
                PQ_PSBT_VERSION,
                version,
            )));
        }

        let signature_flags = SignatureFlags(data[5]);

        let mut offset = 6;

        // Read global data
        let temp_psbt = PQPSBT {
            version: 0,
            signature_flags: SignatureFlags(0),
            global: BTreeMap::new(),
            inputs: vec![],
            outputs: vec![],
        };
        let (global, new_offset) = temp_psbt.deserialize_map(&data[offset..])?;
        offset += new_offset;

        // Read input count
        let (input_count, new_offset) = temp_psbt.read_compact_size(&data[offset..])?;
        offset += new_offset;

        // Read inputs
        let mut inputs = vec![];
        for _ in 0..input_count {
            let (fields, new_offset) = temp_psbt.deserialize_map(&data[offset..])?;
            // Convert raw map to typed map
            let typed_fields = fields
                .into_iter()
                .filter_map(|(k, v)| {
                    // Try to deserialize as InputKey
                    bincode::deserialize::<InputKey>(&k)
                        .ok()
                        .map(|key| (key, v))
                })
                .collect();
            inputs.push(PSBTInput {
                fields: typed_fields,
            });
            offset += new_offset;
        }

        // Read output count
        let (output_count, new_offset) = temp_psbt.read_compact_size(&data[offset..])?;
        offset += new_offset;

        // Read outputs
        let mut outputs = vec![];
        for _ in 0..output_count {
            let (fields, new_offset) = temp_psbt.deserialize_map(&data[offset..])?;
            // Convert raw map to typed map
            let typed_fields = fields
                .into_iter()
                .filter_map(|(k, v)| {
                    // Try to deserialize as OutputKey
                    bincode::deserialize::<OutputKey>(&k)
                        .ok()
                        .map(|key| (key, v))
                })
                .collect();
            outputs.push(PSBTOutput {
                fields: typed_fields,
            });
            offset += new_offset;
        }

        Ok(Self {
            version,
            signature_flags,
            // Convert raw map to typed map
            global: global
                .into_iter()
                .filter_map(|(k, v)| {
                    // Try to deserialize as GlobalKey
                    bincode::deserialize::<GlobalKey>(&k)
                        .ok()
                        .map(|key| (key, v))
                })
                .collect(),
            inputs,
            outputs,
        })
    }

    /// Finalize PSBT and extract transaction
    ///
    /// This method constructs the final transaction from the PSBT data
    /// by extracting all signatures and witness data.
    ///
    /// According to BIP 174, finalization requires all inputs to have
    /// complete signature data before the transaction can be extracted.
    pub fn finalize(self) -> Result<Transaction> {
        use bitquan_types::{SignaturePayload, TxIn, TxOut, Witness};

        // Extract version from global data (default to 1 if not set)
        let version = self
            .global
            .iter()
            .find_map(|(key, _value)| {
                if let GlobalKey::Version(v) = key {
                    Some(*v as i32)
                } else {
                    None
                }
            })
            .unwrap_or(1);

        // Extract locktime from global data (default to 0 if not set)
        let lock_time = self
            .global
            .iter()
            .find_map(|(key, _value)| {
                if let GlobalKey::Locktime(l) = key {
                    Some(*l)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        // Build inputs and witnesses from PSBT input data
        let mut inputs = Vec::with_capacity(self.inputs.len());
        let mut witnesses = Vec::with_capacity(self.inputs.len());

        for (input_index, psbt_input) in self.inputs.iter().enumerate() {
            // Extract previous txid (required)
            let prev_txid = psbt_input
                .get_field(&InputKey::PreviousTxid([0u8; 32]))
                .ok_or_else(|| {
                    SDKError::PSBT(PSBTError::MissingField("PreviousTxid".to_string()))
                })?;

            if prev_txid.len() != 32 {
                return Err(SDKError::PSBT(PSBTError::InvalidFormat(format!(
                    "PreviousTxid must be 32 bytes, got {}",
                    prev_txid.len()
                ))));
            }

            let mut txid = [0u8; 32];
            txid.copy_from_slice(prev_txid);

            // Extract previous output index (required)
            let prev_vout_bytes = psbt_input
                .get_field(&InputKey::PreviousOutputIndex(0))
                .ok_or_else(|| {
                    SDKError::PSBT(PSBTError::MissingField("PreviousOutputIndex".to_string()))
                })?;

            if prev_vout_bytes.len() < 4 {
                return Err(SDKError::PSBT(PSBTError::InvalidFormat(
                    "PreviousOutputIndex must be 4 bytes".to_string(),
                )));
            }

            let prev_vout = u32::from_le_bytes([
                prev_vout_bytes[0],
                prev_vout_bytes[1],
                prev_vout_bytes[2],
                prev_vout_bytes[3],
            ]);

            // Extract sequence (default to 0xffffffff if not set)
            let sequence = psbt_input
                .get_field(&InputKey::Sequence(0xffffffff))
                .and_then(|bytes| {
                    if bytes.len() >= 4 {
                        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                    } else {
                        None
                    }
                })
                .unwrap_or(0xffffffff);

            // Extract Dilithium signature (required for finalization)
            let signature = psbt_input
                .get_field(&InputKey::DilithiumSignature([0u8; SIGNBYTES]))
                .ok_or_else(|| {
                    SDKError::PSBT(PSBTError::InvalidFormat(
                        "Missing Dilithium signature - cannot finalize incomplete PSBT".to_string(),
                    ))
                })?;

            if signature.len() != SIGNBYTES {
                return Err(SDKError::PSBT(PSBTError::InvalidFormat(format!(
                    "Dilithium signature must be {} bytes, got {}",
                    SIGNBYTES,
                    signature.len()
                ))));
            }

            // Extract Dilithium public key (required for finalization)
            let public_key = psbt_input.get_dilithium_public_key().ok_or_else(|| {
                SDKError::PSBT(PSBTError::InvalidFormat(
                    "Missing Dilithium public key - cannot finalize incomplete PSBT".to_string(),
                ))
            })?;

            // Create witness with SignaturePayload struct (not enum)
            let sig_payload = SignaturePayload {
                signer_index: input_index as u16,
                signature: signature.to_vec(),
                public_key: public_key.to_vec(),
                aux: None,
            };

            witnesses.push(Witness {
                signatures: vec![sig_payload],
            });

            // Create TxIn
            inputs.push(TxIn {
                prev_txid: txid,
                prev_vout,
                script_sig: vec![], // Script sig is empty for witness transactions
                sequence,
            });
        }

        // Build outputs from PSBT output data
        let mut outputs = Vec::with_capacity(self.outputs.len());
        for psbt_output in &self.outputs {
            // Extract amount (required)
            let amount = psbt_output
                .get_amount()
                .ok_or_else(|| SDKError::PSBT(PSBTError::MissingField("Amount".to_string())))?;

            // Extract script pubkey (required)
            let script_pubkey = psbt_output.get_script_pubkey().ok_or_else(|| {
                SDKError::PSBT(PSBTError::MissingField("ScriptPubkey".to_string()))
            })?;

            outputs.push(TxOut {
                value: amount,
                script_pubkey,
            });
        }

        // Determine signature algorithm from flags
        let sig_algo = if self.signature_flags.has_dilithium() {
            bitquan_types::SigAlgorithm::Dilithium5
        } else {
            return Err(SDKError::PSBT(PSBTError::InvalidFormat(
                "No valid signature algorithm specified in flags".to_string(),
            )));
        };

        // Extract genesis hash from global data or use default
        let genesis_hash = self
            .global
            .iter()
            .find_map(|(key, _value)| {
                if let GlobalKey::FallbackFingerprint(hash) = key {
                    Some(*hash)
                } else {
                    None
                }
            })
            .unwrap_or(bitquan_types::GENESIS_HASH_BYTES);

        // Build final transaction
        let tx = Transaction {
            version,
            network: bitquan_types::NetworkId::Devnet,
            genesis_hash,
            lock_time,
            inputs,
            outputs,
            sig_algo,
            witnesses,
        };

        Ok(tx)
    }

    // Helper methods for serialization

    fn serialize_map<K: Ord + Serialize, V: AsRef<[u8]>>(
        &self,
        buffer: &mut Vec<u8>,
        map: &BTreeMap<K, V>,
    ) -> Result<()> {
        self.write_compact_size(buffer, map.len())?;

        for (key, value) in map {
            // Serialize key
            let key_bytes = bincode::serialize(key)
                .map_err(|e| SDKError::PSBT(PSBTError::Serialization(e.to_string())))?;
            self.write_compact_size(buffer, key_bytes.len())?;
            buffer.extend_from_slice(&key_bytes);

            // Serialize value
            self.write_compact_size(buffer, value.as_ref().len())?;
            buffer.extend_from_slice(value.as_ref());
        }

        Ok(())
    }

    fn deserialize_map(&self, data: &[u8]) -> Result<(BTreeMap<Vec<u8>, Vec<u8>>, usize)> {
        let mut offset = 0;

        // Read count
        let (count, new_offset) = self.read_compact_size(&data[offset..])?;
        offset += new_offset;

        let mut map = BTreeMap::new();

        for _ in 0..count {
            // Read key length
            let (key_len, new_offset) = self.read_compact_size(&data[offset..])?;
            offset += new_offset;

            // Read key
            if offset + key_len > data.len() {
                return Err(SDKError::PSBT(PSBTError::InvalidFormat(
                    "Key too long".to_string(),
                )));
            }
            let key = data[offset..offset + key_len].to_vec();
            offset += key_len;

            // Read value length
            let (value_len, new_offset) = self.read_compact_size(&data[offset..])?;
            offset += new_offset;

            // Read value
            if offset + value_len > data.len() {
                return Err(SDKError::PSBT(PSBTError::InvalidFormat(
                    "Value too long".to_string(),
                )));
            }
            let value = data[offset..offset + value_len].to_vec();
            offset += value_len;

            map.insert(key, value);
        }

        Ok((map, offset))
    }

    fn write_compact_size(&self, buffer: &mut Vec<u8>, size: usize) -> Result<()> {
        if size < 0xfd {
            buffer.push(size as u8);
        } else if size <= 0xffff {
            buffer.push(0xfd);
            buffer.extend_from_slice(&(size as u16).to_le_bytes());
        } else if size <= 0xffffffff {
            buffer.push(0xfe);
            buffer.extend_from_slice(&(size as u32).to_le_bytes());
        } else {
            buffer.push(0xff);
            buffer.extend_from_slice(&(size as u64).to_le_bytes());
        }
        Ok(())
    }

    fn read_compact_size(&self, data: &[u8]) -> Result<(usize, usize)> {
        if data.is_empty() {
            return Err(SDKError::PSBT(PSBTError::InvalidFormat(
                "No size data".to_string(),
            )));
        }

        let first = data[0];
        match first {
            0xff => {
                if data.len() < 9 {
                    return Err(SDKError::PSBT(PSBTError::InvalidFormat(
                        "Incomplete size".to_string(),
                    )));
                }
                let size = u64::from_le_bytes([
                    data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
                ]) as usize;
                Ok((size, 9))
            }
            0xfe => {
                if data.len() < 5 {
                    return Err(SDKError::PSBT(PSBTError::InvalidFormat(
                        "Incomplete size".to_string(),
                    )));
                }
                let size = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as usize;
                Ok((size, 5))
            }
            0xfd => {
                if data.len() < 3 {
                    return Err(SDKError::PSBT(PSBTError::InvalidFormat(
                        "Incomplete size".to_string(),
                    )));
                }
                let size = u16::from_le_bytes([data[1], data[2]]) as usize;
                Ok((size, 3))
            }
            _ => Ok((first as usize, 1)),
        }
    }
}

impl Default for PQPSBT {
    fn default() -> Self {
        Self::new()
    }
}

/// PSBT builder for convenient construction
pub struct PQPSBTBuilder {
    psbt: PQPSBT,
}

impl PQPSBTBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            psbt: PQPSBT::new(),
        }
    }

    /// Set transaction version
    pub fn version(mut self, version: u32) -> Self {
        let mut bytes = vec![];
        bytes.extend_from_slice(&version.to_le_bytes());
        self.psbt.global.insert(GlobalKey::Version(version), bytes);
        self
    }

    /// Set locktime
    pub fn locktime(mut self, locktime: u32) -> Self {
        let mut bytes = vec![];
        bytes.extend_from_slice(&locktime.to_le_bytes());
        self.psbt
            .global
            .insert(GlobalKey::Locktime(locktime), bytes);
        self
    }

    /// Set signature flags
    pub fn signature_flags(mut self, flags: SignatureFlags) -> Self {
        self.psbt.signature_flags = flags;
        self
    }

    /// Add input
    pub fn add_input(mut self, txid: &str, vout: u32) -> Result<Self> {
        let txid_bytes = hex::decode(txid)
            .map_err(|e| SDKError::PSBT(PSBTError::InvalidFormat(e.to_string())))?;

        if txid_bytes.len() != 32 {
            return Err(SDKError::PSBT(PSBTError::InvalidFormat(
                "Invalid TXID length".to_string(),
            )));
        }

        let mut txid_array = [0u8; 32];
        txid_array.copy_from_slice(&txid_bytes);

        let mut input = PSBTInput::new();
        input.set_previous_txid(txid_array);
        input.set_previous_output_index(vout);
        input.set_sequence(0xffffffff);

        self.psbt.inputs.push(input);
        Ok(self)
    }

    /// Add output
    pub fn add_output(mut self, address: &str, amount: u128) -> Result<Self> {
        let addr = Address::parse(address)?;

        let mut output = PSBTOutput::new();
        output.set_amount(amount);

        // Build script pubkey from address
        let script_pubkey = self.build_script_pubkey(&addr)?;
        output.set_script_pubkey(script_pubkey);

        self.psbt.outputs.push(output);
        Ok(self)
    }

    /// Build PSBT
    pub fn build(self) -> Result<PQPSBT> {
        Ok(self.psbt)
    }

    /// Build script pubkey from address
    fn build_script_pubkey(&self, address: &Address) -> Result<Vec<u8>> {
        match address.address_type {
            crate::address::AddressType::P2PKH => {
                let mut script = vec![0x76, 0xa9, 0x14]; // OP_DUP OP_HASH160 OP_DATA_20
                script.extend_from_slice(&address.data);
                script.extend_from_slice(&[0x88, 0xac]); // OP_EQUALVERIFY OP_CHECKSIG
                Ok(script)
            }
            crate::address::AddressType::P2WPKH => {
                let mut script = vec![0x00, 0x14]; // OP_0 OP_DATA_20
                script.extend_from_slice(&address.data);
                Ok(script)
            }
            crate::address::AddressType::PQP2PKH => {
                // Similar to P2PKH but with different version
                let mut script = vec![0x76, 0xa9, 0x14]; // OP_DUP OP_HASH160 OP_DATA_20
                script.extend_from_slice(&address.data);
                script.extend_from_slice(&[0x88, 0xac]); // OP_EQUALVERIFY OP_CHECKSIG
                Ok(script)
            }
            _ => Err(SDKError::PSBT(PSBTError::InvalidFormat(
                "Unsupported address type".to_string(),
            ))),
        }
    }
}

impl Default for PQPSBTBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psbt_builder() {
        let pubkey_hash = [0x12; 20];
        let address =
            crate::address::Address::p2pkh(crate::address::Network::Mainnet, &pubkey_hash).unwrap();

        let psbt = PQPSBT::builder()
            .version(1)
            .locktime(0)
            .add_input(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                0,
            )
            .unwrap()
            .add_output(&address.to_string(), 1000000)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(psbt.version, 0);
        assert_eq!(psbt.inputs.len(), 1);
        assert_eq!(psbt.outputs.len(), 1);
    }

    #[test]
    fn test_psbt_serialization() {
        let pubkey_hash = [0x12; 20];
        let address =
            crate::address::Address::p2pkh(crate::address::Network::Mainnet, &pubkey_hash).unwrap();

        let psbt = PQPSBT::builder()
            .version(1)
            .add_input(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                0,
            )
            .unwrap()
            .add_output(&address.to_string(), 1000000)
            .unwrap()
            .build()
            .unwrap();

        let serialized = psbt.serialize().unwrap();
        let deserialized = PQPSBT::deserialize(&serialized).unwrap();

        assert_eq!(psbt.version, deserialized.version);
        assert_eq!(psbt.inputs.len(), deserialized.inputs.len());
        assert_eq!(psbt.outputs.len(), deserialized.outputs.len());
    }

    #[test]
    fn test_signature_flags() {
        let flags = SignatureFlags::new()
            .with_dilithium()
            .with_ecdsa()
            .with_hybrid();

        assert!(flags.has_dilithium());
        assert!(flags.has_ecdsa());
        assert!(flags.is_hybrid());
    }
}
