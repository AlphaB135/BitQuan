//! Wire protocol binary serialization and deserialization.
//!
//! This module implements the canonical binary format for transactions and blocks
//! that is used over the P2P network and for computing transaction/block hashes.

use crate::{Block, BlockHeader, CompactUint, NetworkId, Transaction, TxIn, TxOut, Witness};
use std::io::{self, Read, Write};

/// Error types for wire protocol serialization/deserialization.
#[derive(Debug, thiserror::Error)]
pub enum WireError {
    /// I/O error during reading or writing.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Invalid data format.
    #[error("invalid format: {0}")]
    InvalidFormat(String),

    /// Data exceeds size limits.
    #[error("size limit exceeded: {0}")]
    SizeLimit(String),
}

/// Maximum transaction size (1 MB).
pub const MAX_TX_SIZE: usize = 1_000_000;

/// Maximum block size (4 MB).
pub const MAX_BLOCK_SIZE: usize = 4_000_000;

/// Trait for types that can be serialized to wire format.
pub trait WireEncode {
    /// Encodes the value into the writer.
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), WireError>;

    /// Returns the encoded size in bytes.
    fn encoded_size(&self) -> usize;
}

/// Trait for types that can be deserialized from wire format.
pub trait WireDecode: Sized {
    /// Decodes the value from the reader.
    fn decode<R: Read>(reader: &mut R) -> Result<Self, WireError>;
}

// Helper functions for reading/writing primitives

fn write_u8<W: Write>(writer: &mut W, value: u8) -> Result<(), WireError> {
    writer.write_all(&[value])?;
    Ok(())
}

fn write_u16_le<W: Write>(writer: &mut W, value: u16) -> Result<(), WireError> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn write_u32_le<W: Write>(writer: &mut W, value: u32) -> Result<(), WireError> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn write_u64_le<W: Write>(writer: &mut W, value: u64) -> Result<(), WireError> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn write_bytes<W: Write>(writer: &mut W, bytes: &[u8]) -> Result<(), WireError> {
    writer.write_all(bytes)?;
    Ok(())
}

fn write_var_bytes<W: Write>(writer: &mut W, bytes: &[u8]) -> Result<(), WireError> {
    let len = CompactUint::from(bytes.len() as u64);
    len.encode(writer)?;
    write_bytes(writer, bytes)?;
    Ok(())
}

fn read_u8<R: Read>(reader: &mut R) -> Result<u8, WireError> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u16_le<R: Read>(reader: &mut R) -> Result<u16, WireError> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u32_le<R: Read>(reader: &mut R) -> Result<u32, WireError> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64_le<R: Read>(reader: &mut R) -> Result<u64, WireError> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_bytes<R: Read>(reader: &mut R, len: usize) -> Result<Vec<u8>, WireError> {
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

fn read_var_bytes<R: Read>(reader: &mut R, max_size: usize) -> Result<Vec<u8>, WireError> {
    let len = CompactUint::decode(reader)?;
    let len_usize = len.value() as usize;

    if len_usize > max_size {
        return Err(WireError::SizeLimit(format!(
            "var_bytes length {} exceeds max {}",
            len_usize, max_size
        )));
    }

    read_bytes(reader, len_usize)
}

// CompactUint implementation

impl WireEncode for CompactUint {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), WireError> {
        let value = self.value();

        if value < 253 {
            write_u8(writer, value as u8)?;
        } else if value <= 0xFFFF {
            write_u8(writer, 253)?;
            writer.write_all(&(value as u16).to_le_bytes())?;
        } else if value <= 0xFFFFFFFF {
            write_u8(writer, 254)?;
            writer.write_all(&(value as u32).to_le_bytes())?;
        } else {
            write_u8(writer, 255)?;
            writer.write_all(&value.to_le_bytes())?;
        }

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        self.encoded_length()
    }
}

impl WireDecode for CompactUint {
    fn decode<R: Read>(reader: &mut R) -> Result<Self, WireError> {
        let first = read_u8(reader)?;

        let value = match first {
            0..=252 => first as u64,
            253 => {
                let mut buf = [0u8; 2];
                reader.read_exact(&mut buf)?;
                let value = u16::from_le_bytes(buf) as u64;
                if value < 0xFD {
                    return Err(WireError::InvalidFormat(
                        "non-canonical compact uint (16-bit)".into(),
                    ));
                }
                value
            }
            254 => {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                let value = u32::from_le_bytes(buf) as u64;
                if value <= 0xFFFF {
                    return Err(WireError::InvalidFormat(
                        "non-canonical compact uint (32-bit)".into(),
                    ));
                }
                value
            }
            255 => {
                let mut buf = [0u8; 8];
                reader.read_exact(&mut buf)?;
                let value = u64::from_le_bytes(buf);
                if value <= 0xFFFF_FFFF {
                    return Err(WireError::InvalidFormat(
                        "non-canonical compact uint (64-bit)".into(),
                    ));
                }
                value
            }
        };

        Ok(CompactUint::from(value))
    }
}

// TxIn implementation

impl WireEncode for TxIn {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), WireError> {
        write_bytes(writer, &self.prev_txid)?;
        write_u32_le(writer, self.prev_vout)?;
        write_var_bytes(writer, &self.script_sig)?;
        write_u32_le(writer, self.sequence)?;
        Ok(())
    }

    fn encoded_size(&self) -> usize {
        32 + 4
            + CompactUint::from(self.script_sig.len() as u64).encoded_length()
            + self.script_sig.len()
            + 4
    }
}

impl WireDecode for TxIn {
    fn decode<R: Read>(reader: &mut R) -> Result<Self, WireError> {
        let mut prev_txid = [0u8; 32];
        reader.read_exact(&mut prev_txid)?;

        let prev_vout = read_u32_le(reader)?;
        let script_sig = read_var_bytes(reader, 10_000)?; // Max script size
        let sequence = read_u32_le(reader)?;

        Ok(TxIn {
            prev_txid,
            prev_vout,
            script_sig,
            sequence,
        })
    }
}

// TxOut implementation

impl WireEncode for TxOut {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), WireError> {
        write_u64_le(writer, self.value)?;
        write_var_bytes(writer, &self.script_pubkey)?;
        Ok(())
    }

    fn encoded_size(&self) -> usize {
        8 + CompactUint::from(self.script_pubkey.len() as u64).encoded_length()
            + self.script_pubkey.len()
    }
}

impl WireDecode for TxOut {
    fn decode<R: Read>(reader: &mut R) -> Result<Self, WireError> {
        let value = read_u64_le(reader)?;
        let script_pubkey = read_var_bytes(reader, 10_000)?;

        Ok(TxOut {
            value,
            script_pubkey,
        })
    }
}

// Witness implementation

impl WireEncode for Witness {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), WireError> {
        let sig_count = CompactUint::from(self.signatures.len() as u64);
        sig_count.encode(writer)?;

        for sig in &self.signatures {
            write_u16_le(writer, sig.signer_index)?;
            write_var_bytes(writer, &sig.signature)?;
            write_var_bytes(writer, &sig.public_key)?;

            match &sig.aux {
                Some(aux) => {
                    write_u8(writer, 1)?;
                    write_var_bytes(writer, &aux.payload)?;
                }
                None => {
                    write_u8(writer, 0)?;
                }
            }
        }

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        let mut size = CompactUint::from(self.signatures.len() as u64).encoded_length();

        for sig in &self.signatures {
            size += 2; // signer_index
            size += CompactUint::from(sig.signature.len() as u64).encoded_length()
                + sig.signature.len();
            size += CompactUint::from(sig.public_key.len() as u64).encoded_length()
                + sig.public_key.len();
            size += 1; // aux flag
            if let Some(aux) = &sig.aux {
                size += CompactUint::from(aux.payload.len() as u64).encoded_length()
                    + aux.payload.len();
            }
        }

        size
    }
}

impl WireDecode for Witness {
    fn decode<R: Read>(reader: &mut R) -> Result<Self, WireError> {
        let sig_count = CompactUint::decode(reader)?;
        let sig_count_usize = sig_count.value() as usize;

        if sig_count_usize > 10_000 {
            return Err(WireError::SizeLimit(
                "too many signatures in witness".into(),
            ));
        }

        let mut signatures = Vec::with_capacity(sig_count_usize);

        for _ in 0..sig_count_usize {
            let signer_index = read_u16_le(reader)?;
            let signature = read_var_bytes(reader, 10_000)?;
            let public_key = read_var_bytes(reader, 10_000)?;

            let has_aux = read_u8(reader)?;
            let aux = if has_aux == 1 {
                let aux_payload = read_var_bytes(reader, 1_000)?;
                Some(crate::AuxiliarySignatureData {
                    payload: aux_payload,
                })
            } else {
                None
            };

            signatures.push(crate::SignaturePayload {
                signer_index,
                signature,
                public_key,
                aux,
            });
        }

        Ok(Witness { signatures })
    }
}

// Transaction implementation

impl WireEncode for Transaction {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), WireError> {
        // Version
        write_u32_le(writer, self.version as u32)?;

        // Network identifier
        write_u8(writer, self.network.as_u8())?;

        // Genesis hash
        writer.write_all(&self.genesis_hash)?;

        // Lock time
        write_u32_le(writer, self.lock_time)?;

        // Inputs
        let input_count = CompactUint::from(self.inputs.len() as u64);
        input_count.encode(writer)?;
        for input in &self.inputs {
            input.encode(writer)?;
        }

        // Outputs
        let output_count = CompactUint::from(self.outputs.len() as u64);
        output_count.encode(writer)?;
        for output in &self.outputs {
            output.encode(writer)?;
        }

        // Signature algorithm
        write_u8(writer, self.sig_algo.code())?;

        // Witnesses
        let witness_count = CompactUint::from(self.witnesses.len() as u64);
        witness_count.encode(writer)?;
        for witness in &self.witnesses {
            witness.encode(writer)?;
        }

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        let mut size = 4; // version
        size += 1; // network id
        size += 32; // genesis hash
        size += 4; // lock_time

        size += CompactUint::from(self.inputs.len() as u64).encoded_length();
        size += self.inputs.iter().map(|i| i.encoded_size()).sum::<usize>();

        size += CompactUint::from(self.outputs.len() as u64).encoded_length();
        size += self.outputs.iter().map(|o| o.encoded_size()).sum::<usize>();

        size += 1; // sig_algo

        size += CompactUint::from(self.witnesses.len() as u64).encoded_length();
        size += self
            .witnesses
            .iter()
            .map(|w| w.encoded_size())
            .sum::<usize>();

        size
    }
}

impl WireDecode for Transaction {
    fn decode<R: Read>(reader: &mut R) -> Result<Self, WireError> {
        let version = read_u32_le(reader)? as i32;
        let network_code = read_u8(reader)?;
        let network = NetworkId::from_u8(network_code).ok_or_else(|| {
            WireError::InvalidFormat(format!("invalid network id: {network_code}"))
        })?;

        let mut genesis_hash = [0u8; 32];
        reader.read_exact(&mut genesis_hash)?;
        let lock_time = read_u32_le(reader)?;

        // Inputs
        let input_count = CompactUint::decode(reader)?;
        let input_count_usize = input_count.value() as usize;

        if input_count_usize > 100_000 {
            return Err(WireError::SizeLimit("too many inputs".into()));
        }

        let mut inputs = Vec::with_capacity(input_count_usize);
        for _ in 0..input_count_usize {
            inputs.push(TxIn::decode(reader)?);
        }

        // Outputs
        let output_count = CompactUint::decode(reader)?;
        let output_count_usize = output_count.value() as usize;

        if output_count_usize > 100_000 {
            return Err(WireError::SizeLimit("too many outputs".into()));
        }

        let mut outputs = Vec::with_capacity(output_count_usize);
        for _ in 0..output_count_usize {
            outputs.push(TxOut::decode(reader)?);
        }

        // Signature algorithm
        let sig_algo_code = read_u8(reader)?;
        let sig_algo = crate::SigAlgorithm::from_code(sig_algo_code);

        // Witnesses
        let witness_count = CompactUint::decode(reader)?;
        let witness_count_usize = witness_count.value() as usize;

        if witness_count_usize > 100_000 {
            return Err(WireError::SizeLimit("too many witnesses".into()));
        }

        let mut witnesses = Vec::with_capacity(witness_count_usize);
        for _ in 0..witness_count_usize {
            witnesses.push(Witness::decode(reader)?);
        }

        Ok(Transaction {
            version,
            network,
            genesis_hash,
            lock_time,
            inputs,
            outputs,
            sig_algo,
            witnesses,
        })
    }
}

// BlockHeader implementation

impl WireEncode for BlockHeader {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), WireError> {
        write_u32_le(writer, self.version as u32)?;
        write_bytes(writer, &self.prev_block)?;
        write_bytes(writer, &self.merkle_root)?;
        write_bytes(writer, &self.pqc_agg_hint)?;
        write_u32_le(writer, self.time)?;
        write_u32_le(writer, self.bits)?;
        write_u64_le(writer, self.nonce)?;
        writer.write_all(&[self.algo_id])?;
        Ok(())
    }

    fn encoded_size(&self) -> usize {
        4 + 32 + 32 + 32 + 4 + 4 + 8 + 1 // 117 bytes (added algo_id)
    }
}

impl WireDecode for BlockHeader {
    fn decode<R: Read>(reader: &mut R) -> Result<Self, WireError> {
        let version = read_u32_le(reader)? as i32;

        let mut prev_block = [0u8; 32];
        reader.read_exact(&mut prev_block)?;

        let mut merkle_root = [0u8; 32];
        reader.read_exact(&mut merkle_root)?;

        let mut pqc_agg_hint = [0u8; 32];
        reader.read_exact(&mut pqc_agg_hint)?;

        let time = read_u32_le(reader)?;
        let bits = read_u32_le(reader)?;
        let nonce = read_u64_le(reader)?;

        // Read algo_id (added for hybrid PoW support)
        let mut algo_id_buf = [0u8; 1];
        let algo_id = if reader.read_exact(&mut algo_id_buf).is_ok() {
            algo_id_buf[0]
        } else {
            0 // Default to SHA-256d for legacy headers without algo_id
        };

        Ok(BlockHeader {
            version,
            prev_block,
            merkle_root,
            pqc_agg_hint,
            time,
            bits,
            nonce,
            algo_id,
        })
    }
}

// Block implementation

impl WireEncode for Block {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), WireError> {
        self.header.encode(writer)?;

        let tx_count = CompactUint::from(self.transactions.len() as u64);
        tx_count.encode(writer)?;

        for tx in &self.transactions {
            tx.encode(writer)?;
        }

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        let mut size = self.header.encoded_size();
        size += CompactUint::from(self.transactions.len() as u64).encoded_length();
        size += self
            .transactions
            .iter()
            .map(|tx| tx.encoded_size())
            .sum::<usize>();
        size
    }
}

impl WireDecode for Block {
    fn decode<R: Read>(reader: &mut R) -> Result<Self, WireError> {
        let header = BlockHeader::decode(reader)?;

        let tx_count = CompactUint::decode(reader)?;
        let tx_count_usize = tx_count.value() as usize;

        if tx_count_usize > 100_000 {
            return Err(WireError::SizeLimit("too many transactions".into()));
        }

        let mut transactions = Vec::with_capacity(tx_count_usize);
        for _ in 0..tx_count_usize {
            transactions.push(Transaction::decode(reader)?);
        }

        Ok(Block {
            header,
            transactions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genesis::GENESIS_HASH_BYTES;
    use crate::SigAlgorithm;

    #[test]
    fn test_compact_uint_roundtrip() {
        let values = vec![0, 252, 253, 65535, 65536, 0xFFFFFFFF_u64];

        for &val in &values {
            let compact = CompactUint::from(val);
            let mut buf = Vec::new();
            compact
                .encode(&mut buf)
                .expect("Failed to encode compact uint");

            let decoded =
                CompactUint::decode(&mut &buf[..]).expect("Failed to decode compact uint");
            assert_eq!(decoded.value(), val);
        }
    }

    #[test]
    fn rejects_non_canonical_compact_uint() {
        // 253 encoding must not be used for values below 0xfd.
        let invalid_fd = [0xfd, 0xfc, 0x00];
        assert!(matches!(
            CompactUint::decode(&mut &invalid_fd[..]),
            Err(WireError::InvalidFormat(_))
        ));

        // 254 encoding must not be used for <= 0xffff.
        let invalid_fe = [0xfe, 0xff, 0xff, 0x00, 0x00];
        assert!(matches!(
            CompactUint::decode(&mut &invalid_fe[..]),
            Err(WireError::InvalidFormat(_))
        ));

        // 255 encoding must not be used for <= 0xffff_ffff.
        let invalid_ff = [0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00];
        assert!(matches!(
            CompactUint::decode(&mut &invalid_ff[..]),
            Err(WireError::InvalidFormat(_))
        ));
    }

    #[test]
    fn test_transaction_roundtrip() {
        let tx = Transaction {
            version: 1,
            network: NetworkId::Devnet,
            genesis_hash: GENESIS_HASH_BYTES,
            lock_time: 0,
            inputs: vec![TxIn {
                prev_txid: [1u8; 32],
                prev_vout: 0,
                script_sig: vec![0x48, 0x65, 0x6c, 0x6c, 0x6f],
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                value: 5000000000,
                script_pubkey: vec![0x76, 0xa9, 0x14],
            }],
            sig_algo: SigAlgorithm::Dilithium5,
            witnesses: vec![Witness {
                signatures: vec![crate::SignaturePayload {
                    signer_index: 0,
                    signature: vec![0xde; 4595],
                    public_key: vec![0xab; 2592],
                    aux: None,
                }],
            }],
        };

        let mut buf = Vec::new();
        tx.encode(&mut buf).expect("Failed to encode transaction");

        let decoded = Transaction::decode(&mut &buf[..]).expect("Failed to decode transaction");
        assert_eq!(decoded.version, tx.version);
        assert_eq!(decoded.inputs.len(), tx.inputs.len());
        assert_eq!(decoded.outputs.len(), tx.outputs.len());
        assert_eq!(decoded.witnesses.len(), tx.witnesses.len());
    }

    #[test]
    fn test_block_header_roundtrip() {
        let header = BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [1u8; 32],
            pqc_agg_hint: [2u8; 32],
            time: 1234567890,
            bits: 0x1d00ffff,
            nonce: 424242,
            algo_id: 0,
        };

        let mut buf = Vec::new();
        header
            .encode(&mut buf)
            .expect("Failed to encode block header");
        assert_eq!(buf.len(), 117); // Fixed header size (with algo_id)

        let decoded = BlockHeader::decode(&mut &buf[..]).expect("Failed to decode block header");
        assert_eq!(decoded.version, header.version);
        assert_eq!(decoded.time, header.time);
        assert_eq!(decoded.nonce, header.nonce);
    }

    #[test]
    fn test_block_roundtrip() {
        let block = Block {
            header: BlockHeader {
                version: 1,
                prev_block: [0u8; 32],
                merkle_root: [1u8; 32],
                pqc_agg_hint: [2u8; 32],
                time: 1234567890,
                bits: 0x1d00ffff,
                nonce: 424242,
                algo_id: 0,
            },
            transactions: vec![Transaction {
                version: 1,
                network: NetworkId::Devnet,
                genesis_hash: GENESIS_HASH_BYTES,
                lock_time: 0,
                inputs: vec![],
                outputs: vec![TxOut {
                    value: 5000000000,
                    script_pubkey: vec![],
                }],
                sig_algo: SigAlgorithm::Dilithium5,
                witnesses: vec![],
            }],
        };

        let mut buf = Vec::new();
        block.encode(&mut buf).expect("Failed to encode block");

        let decoded = Block::decode(&mut &buf[..]).expect("Failed to decode block");
        assert_eq!(decoded.transactions.len(), block.transactions.len());
    }
}
