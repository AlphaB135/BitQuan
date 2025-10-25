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
        32 + 4 + 4 + CompactUint::from_usize(self.script_sig.len()).encoded_length()
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
        8 + CompactUint::from_usize(self.script_pubkey.len()).encoded_length() + self.script_pubkey.len()
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
        len += CompactUint::from_usize(self.signature.len()).encoded_length() + self.signature.len();
        len += CompactUint::from_usize(self.public_key.len()).encoded_length() + self.public_key.len();

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
    /// Signature payloads confirming the inputs.
    pub signatures: Vec<SignaturePayload>,
}

impl Transaction {
    /// Returns the number of inputs.
    pub fn inputs_len(&self) -> usize {
        self.inputs.len()
    }

    /// Returns the number of outputs.
    pub fn outputs_len(&self) -> usize {
        self.outputs.len()
    }

    /// Returns the number of explicit signatures.
    pub fn signature_count(&self) -> usize {
        self.signatures.len()
    }

    /// Provides a heuristic serialized size used by consensus weight calculations.
    pub fn serialized_size_hint(&self) -> usize {
        let mut len = 4 + 4; // version + lock_time

        len += CompactUint::from_usize(self.inputs.len()).encoded_length();
        len += self
            .inputs
            .iter()
            .map(TxIn::serialized_size_hint)
            .sum::<usize>();

        len += CompactUint::from_usize(self.outputs.len()).encoded_length();
        len += self
            .outputs
            .iter()
            .map(TxOut::serialized_size_hint)
            .sum::<usize>();

        len += 1; // sig_algo code

        len += CompactUint::from_usize(self.signatures.len()).encoded_length();
        len += self
            .signatures
            .iter()
            .map(SignaturePayload::serialized_size_hint)
            .sum::<usize>();

        len
    }
}
