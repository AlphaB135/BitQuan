//! Transaction-related data structures.

use crate::compact_uint::CompactUint;
use serde::{
    de::{Error as DeError, Unexpected},
    Deserialize, Deserializer, Serialize, Serializer,
};

/// Variable-length byte buffer used throughout wire-level data.
pub type VarBytes = Vec<u8>;

/// Additional metadata attached to a signature, reserved for advanced schemes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuxiliarySignatureData {
    /// Raw payload bytes, to be interpreted based on the signature scheme.
    pub payload: VarBytes,
}

/// Supported signature algorithms for BitQuan transactions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SigAlgorithm {
    /// CRYSTALS-Dilithium level 3.
    Dilithium3,
    /// Falcon-512 lattice signature scheme.
    Falcon512,
    /// SPHINCS+ stateless hash-based signature.
    SphincsPlus,
    /// Reserved/unknown code for forward compatibility.
    Reserved(u8),
}

impl SigAlgorithm {
    /// Returns the protocol code associated with the algorithm.
    pub const fn code(self) -> u8 {
        match self {
            SigAlgorithm::Dilithium3 => 0x01,
            SigAlgorithm::Falcon512 => 0x02,
            SigAlgorithm::SphincsPlus => 0x03,
            SigAlgorithm::Reserved(value) => value,
        }
    }

    /// Constructs an algorithm variant from the assigned protocol code.
    pub const fn from_code(code: u8) -> Self {
        match code {
            0x01 => SigAlgorithm::Dilithium3,
            0x02 => SigAlgorithm::Falcon512,
            0x03 => SigAlgorithm::SphincsPlus,
            other => SigAlgorithm::Reserved(other),
        }
    }
}

impl From<SigAlgorithm> for u8 {
    fn from(value: SigAlgorithm) -> Self {
        value.code()
    }
}

impl Serialize for SigAlgorithm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(self.code())
    }
}

impl<'de> Deserialize<'de> for SigAlgorithm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        let algo = SigAlgorithm::from_code(value);

        if let SigAlgorithm::Reserved(0x00) = algo {
            return Err(D::Error::invalid_value(
                Unexpected::Unsigned(value as u64),
                &"non-zero reserved signature algorithm code",
            ));
        }

        Ok(algo)
    }
}

/// Transaction input referencing previous outputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxIn {
    /// Previous transaction identifier (little-endian on disk, big-endian in display).
    pub prev_txid: [u8; 32],
    /// Index into the previous transaction's outputs.
    pub prev_vout: u32,
    /// Sequence number used for lock-time semantics.
    pub sequence: u32,
    /// Unlocking script or PQC witness payload.
    pub script_sig: VarBytes,
}

impl TxIn {
    /// Returns a heuristic byte length for serialization planning.
    pub fn serialized_size_hint(&self) -> usize {
        32 + 4
            + 4
            + CompactUint::from_usize(self.script_sig.len()).encoded_length()
            + self.script_sig.len()
    }
}

/// Transaction output representing new spendable units.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxOut {
    /// Amount in the smallest denomination (1 BQ = 10^8 units).
    pub value: u64,
    /// Locking script describing redemption conditions.
    pub script_pubkey: VarBytes,
}

impl TxOut {
    /// Returns a heuristic byte length for serialization planning.
    pub fn serialized_size_hint(&self) -> usize {
        8 + CompactUint::from_usize(self.script_pubkey.len()).encoded_length()
            + self.script_pubkey.len()
    }
}

/// Signature payload attached to an input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignaturePayload {
    /// Index of the input this signature satisfies.
    pub signer_index: u16,
    /// Raw signature bytes.
    pub signature: VarBytes,
    /// Corresponding public key bytes.
    pub public_key: VarBytes,
    /// Optional auxiliary data.
    pub aux: Option<AuxiliarySignatureData>,
}

impl SignaturePayload {
    /// Returns the total byte length contributed by this payload (best-effort estimate).
    pub fn serialized_size_hint(&self) -> usize {
        let mut len = 2;
        len +=
            CompactUint::from_usize(self.signature.len()).encoded_length() + self.signature.len();
        len +=
            CompactUint::from_usize(self.public_key.len()).encoded_length() + self.public_key.len();

        match &self.aux {
            Some(aux) => {
                len += 1;
                len +=
                    CompactUint::from_usize(aux.payload.len()).encoded_length() + aux.payload.len();
            }
            None => len += 1,
        }

        len
    }
}

/// Core transaction structure as transmitted on the wire.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction format version.
    pub version: i32,
    /// Absolute or relative lock time.
    pub lock_time: u32,
    /// Transaction inputs.
    pub inputs: Vec<TxIn>,
    /// Transaction outputs.
    pub outputs: Vec<TxOut>,
    /// Signature algorithm for all attached signatures.
    pub sig_algo: SigAlgorithm,
    /// Witness data carrying signature payloads and auxiliary metadata.
    pub witnesses: Vec<Witness>,
}

impl Transaction {
    /// Returns the number of inputs.
    pub fn inputs_len(&self) -> usize { self.inputs.len() }
    /// Returns the number of outputs.
    pub fn outputs_len(&self) -> usize { self.outputs.len() }
    /// Returns the number of explicit signatures across all witnesses.
    pub fn signature_count(&self) -> usize { self.witnesses.iter().map(|w| w.signatures.len()).sum() }
    /// Provides a heuristic serialized size used by consensus weight calculations.
    pub fn serialized_size_hint(&self) -> usize {
        let mut len = 4 + 4; // version + lock_time
        len += CompactUint::from_usize(self.inputs.len()).encoded_length();
        len += self.inputs.iter().map(TxIn::serialized_size_hint).sum::<usize>();
        len += CompactUint::from_usize(self.outputs.len()).encoded_length();
        len += self.outputs.iter().map(TxOut::serialized_size_hint).sum::<usize>();
        len += 1; // sig_algo code
        len += CompactUint::from_usize(self.witnesses.len()).encoded_length();
        len += self.witnesses.iter().map(Witness::serialized_size_hint).sum::<usize>();
        len
    }
}

impl Transaction {
    /// Returns the estimated size of witness-only data for this transaction.
    pub fn witness_size_hint(&self) -> usize {
        CompactUint::from_usize(self.witnesses.len()).encoded_length()
            + self.witnesses.iter().map(Witness::serialized_size_hint).sum::<usize>()
    }

    /// Serializes the transaction body without witness.
    pub fn to_bytes_base(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.serialized_size_hint());
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&self.lock_time.to_le_bytes());
        write_compact(&mut out, self.inputs.len() as u64);
        for i in &self.inputs {
            out.extend_from_slice(&i.prev_txid);
            out.extend_from_slice(&i.prev_vout.to_le_bytes());
            out.extend_from_slice(&i.sequence.to_le_bytes());
            write_varbytes(&mut out, &i.script_sig);
        }
        write_compact(&mut out, self.outputs.len() as u64);
        for o in &self.outputs {
            out.extend_from_slice(&o.value.to_le_bytes());
            write_varbytes(&mut out, &o.script_pubkey);
        }
        out.push(self.sig_algo.code());
        out
    }

    /// Serializes the transaction including witness.
    pub fn to_bytes_with_witness(&self) -> Vec<u8> {
        let mut out = self.to_bytes_base();
        write_compact(&mut out, self.witnesses.len() as u64);
        for w in &self.witnesses {
            write_compact(&mut out, w.signatures.len() as u64);
            for s in &w.signatures {
                out.extend_from_slice(&s.signer_index.to_le_bytes());
                write_varbytes(&mut out, &s.signature);
                write_varbytes(&mut out, &s.public_key);
                match &s.aux {
                    Some(aux) => { out.push(1); write_varbytes(&mut out, &aux.payload); }
                    None => out.push(0),
                }
            }
        }
        out
    }

    /// Double-SHA256 over base serialization (txid).
    pub fn txid(&self) -> [u8; 32] { sha256d(&self.to_bytes_base()) }
    /// Double-SHA256 over full serialization (wtxid).
    pub fn wtxid(&self) -> [u8; 32] { sha256d(&self.to_bytes_with_witness()) }
}

fn write_compact(out: &mut Vec<u8>, value: u64) {
    if value <= 0xFC { out.push(value as u8); }
    else if value <= 0xFFFF { out.push(0xFD); out.extend_from_slice(&(value as u16).to_le_bytes()); }
    else if value <= 0xFFFF_FFFF { out.push(0xFE); out.extend_from_slice(&(value as u32).to_le_bytes()); }
    else { out.push(0xFF); out.extend_from_slice(&value.to_le_bytes()); }
}

fn write_varbytes(out: &mut Vec<u8>, data: &[u8]) { write_compact(out, data.len() as u64); out.extend_from_slice(data); }

fn sha256d(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let first = Sha256::digest(data);
    let second = Sha256::digest(first);
    let mut out = [0u8; 32];
    out.copy_from_slice(&second);
    out
}

/// Witness container for PQC signatures and future extensions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Witness {
    /// Signatures included in this witness.
    pub signatures: Vec<SignaturePayload>,
}

impl Witness {
    /// Returns a heuristic serialized size for the witness.
    pub fn serialized_size_hint(&self) -> usize {
        CompactUint::from_usize(self.signatures.len()).encoded_length()
            + self
                .signatures
                .iter()
                .map(SignaturePayload::serialized_size_hint)
                .sum::<usize>()
    }
}
