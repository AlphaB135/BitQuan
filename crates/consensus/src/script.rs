//! Simple script interpreter for BitQuan with PQC signature verification.
//!
//! This module implements a minimal stack-based script language focused on
//! post-quantum signature verification.

use bitquan_types::SignaturePayload;
use bq_crypto::CryptoRegistry;
use thiserror::Error;

/// Maximum script size in bytes (10 KB).
pub const MAX_SCRIPT_SIZE: usize = 10_000;

/// Maximum stack size.
pub const MAX_STACK_SIZE: usize = 1000;

/// Maximum number of operations per script.
pub const MAX_OPS: usize = 201;

/// Script execution errors.
#[derive(Debug, Error)]
pub enum ScriptError {
    /// Stack underflow.
    #[error("stack underflow")]
    StackUnderflow,

    /// Stack overflow.
    #[error("stack overflow")]
    StackOverflow,

    /// Invalid opcode.
    #[error("invalid opcode: {0:#x}")]
    InvalidOpcode(u8),

    /// Script too large.
    #[error("script too large: {0} bytes")]
    ScriptTooLarge(usize),

    /// Too many operations.
    #[error("too many operations: {0}")]
    TooManyOps(usize),

    /// Signature verification failed.
    #[error("signature verification failed")]
    SigVerifyFailed,

    /// Script did not leave true on stack.
    #[error("script evaluation failed")]
    EvalFalse,

    /// Invalid signature format.
    #[error("invalid signature format")]
    InvalidSignature,

    /// Invalid public key format.
    #[error("invalid public key format")]
    InvalidPubKey,
}

/// Script opcodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    /// Push 1 byte of data.
    Push1 = 0x01,
    /// Push 2 bytes (Dilithium3 signature ~3293 bytes, encoded as push opcodes).
    PushData1 = 0x4c,
    /// Push 4 bytes.
    PushData2 = 0x4d,
    /// Push empty array (OP_0).
    False = 0x00,
    /// Push 1.
    True = 0x51,
    /// Duplicate top stack item.
    Dup = 0x76,
    /// Hash top stack item with SHA-256d (legacy).
    Hash256 = 0xaa,
    /// Hash top stack item with BLAKE3 (quantum-safe, high-performance).
    HashBLAKE3 = 0xaf,
    /// Verify PQC signature (Dilithium).
    CheckSigPQC = 0xac,
    /// Verify and leave result on stack.
    CheckSigPQCVerify = 0xad,
}

impl OpCode {
    /// Converts a byte to an opcode.
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x00 => Some(OpCode::False),
            0x01 => Some(OpCode::Push1),
            0x4c => Some(OpCode::PushData1),
            0x4d => Some(OpCode::PushData2),
            0x51 => Some(OpCode::True),
            0x76 => Some(OpCode::Dup),
            0xaa => Some(OpCode::Hash256),
            0xaf => Some(OpCode::HashBLAKE3),
            0xac => Some(OpCode::CheckSigPQC),
            0xad => Some(OpCode::CheckSigPQCVerify),
            _ => None,
        }
    }
}

/// Script interpreter with stack-based execution.
pub struct ScriptInterpreter {
    /// Execution stack.
    stack: Vec<Vec<u8>>,
    /// Operation count.
    op_count: usize,
    /// Crypto registry for signature verification.
    registry: CryptoRegistry,
}

impl ScriptInterpreter {
    /// Creates a new script interpreter.
    pub fn new(registry: CryptoRegistry) -> Self {
        Self {
            stack: Vec::new(),
            op_count: 0,
            registry,
        }
    }

    /// Executes a script and returns success/failure.
    pub fn execute(&mut self, script: &[u8], message: &[u8]) -> Result<bool, ScriptError> {
        // Check script size
        if script.len() > MAX_SCRIPT_SIZE {
            return Err(ScriptError::ScriptTooLarge(script.len()));
        }

        // Reset state
        self.stack.clear();
        self.op_count = 0;

        // Parse and execute
        let mut pc = 0; // Program counter

        while pc < script.len() {
            // Check operation limit
            self.op_count += 1;
            if self.op_count > MAX_OPS {
                return Err(ScriptError::TooManyOps(self.op_count));
            }

            let byte = script[pc];
            pc += 1;

            // Handle push data (0x01-0x4b directly push N bytes)
            if (0x01..=0x4b).contains(&byte) {
                let len = byte as usize;
                if pc + len > script.len() {
                    return Err(ScriptError::InvalidOpcode(byte));
                }
                let data = script[pc..pc + len].to_vec();
                self.push(data)?;
                pc += len;
                continue;
            }

            let opcode = OpCode::from_byte(byte).ok_or(ScriptError::InvalidOpcode(byte))?;

            match opcode {
                OpCode::False => self.push(vec![])?,
                OpCode::True => self.push(vec![1])?,

                OpCode::Push1 => {
                    if pc >= script.len() {
                        return Err(ScriptError::InvalidOpcode(byte));
                    }
                    let len = script[pc] as usize;
                    pc += 1;
                    if pc + len > script.len() {
                        return Err(ScriptError::InvalidOpcode(byte));
                    }
                    let data = script[pc..pc + len].to_vec();
                    self.push(data)?;
                    pc += len;
                }

                OpCode::PushData1 => {
                    if pc >= script.len() {
                        return Err(ScriptError::InvalidOpcode(byte));
                    }
                    let len = script[pc] as usize;
                    pc += 1;
                    if pc + len > script.len() {
                        return Err(ScriptError::InvalidOpcode(byte));
                    }
                    let data = script[pc..pc + len].to_vec();
                    self.push(data)?;
                    pc += len;
                }

                OpCode::PushData2 => {
                    if pc + 1 >= script.len() {
                        return Err(ScriptError::InvalidOpcode(byte));
                    }
                    let len = u16::from_le_bytes([script[pc], script[pc + 1]]) as usize;
                    pc += 2;
                    if pc + len > script.len() {
                        return Err(ScriptError::InvalidOpcode(byte));
                    }
                    let data = script[pc..pc + len].to_vec();
                    self.push(data)?;
                    pc += len;
                }

                OpCode::Dup => {
                    let top = self.peek()?.clone();
                    self.push(top)?;
                }

                OpCode::Hash256 => {
                    let data = self.pop()?;
                    let hash = sha256d(&data);
                    self.push(hash.to_vec())?;
                }

                OpCode::HashBLAKE3 => {
                    let data = self.pop()?;
                    let mut hasher = blake3::Hasher::new();
                    hasher.update(&data);
                    let hash = hasher.finalize().as_bytes().to_vec();
                    self.push(hash)?;
                }

                OpCode::CheckSigPQC | OpCode::CheckSigPQCVerify => {
                    let pubkey = self.pop()?;
                    let sig = self.pop()?;

                    // Verify signature
                    let result = self.verify_signature_pqc(&sig, &pubkey, message);

                    if opcode == OpCode::CheckSigPQC {
                        // Push result (1 or 0)
                        self.push(if result.is_ok() { vec![1] } else { vec![0] })?;
                    } else {
                        // CheckSigVerify: fail if verification failed
                        result?;
                    }
                }
            }
        }

        // Script succeeds if top of stack is true
        if self.stack.is_empty() {
            return Ok(false);
        }

        let top = self.peek()?;
        Ok(!is_false(top))
    }

    /// Pushes data onto stack.
    fn push(&mut self, data: Vec<u8>) -> Result<(), ScriptError> {
        if self.stack.len() >= MAX_STACK_SIZE {
            return Err(ScriptError::StackOverflow);
        }
        self.stack.push(data);
        Ok(())
    }

    /// Pops data from stack.
    fn pop(&mut self) -> Result<Vec<u8>, ScriptError> {
        self.stack.pop().ok_or(ScriptError::StackUnderflow)
    }

    /// Peeks at top of stack without removing.
    fn peek(&self) -> Result<&Vec<u8>, ScriptError> {
        self.stack.last().ok_or(ScriptError::StackUnderflow)
    }

    /// Verifies a PQC signature.
    fn verify_signature_pqc(
        &self,
        sig: &[u8],
        pubkey: &[u8],
        message: &[u8],
    ) -> Result<(), ScriptError> {
        // Create signature payload
        let payload = SignaturePayload {
            signer_index: 0,
            signature: sig.to_vec(),
            public_key: pubkey.to_vec(),
            aux: None,
        };

        // Get provider from registry
        let provider = self
            .registry
            .provider_for(bitquan_types::SigAlgorithm::Dilithium3)
            .ok_or(ScriptError::InvalidSignature)?;

        // Verify
        provider
            .verify(&payload, message)
            .map_err(|_| ScriptError::SigVerifyFailed)?;

        Ok(())
    }
}

/// Checks if a stack element is false (empty or all zeros).
fn is_false(data: &[u8]) -> bool {
    data.is_empty() || data.iter().all(|&b| b == 0)
}

/// Double SHA-256.
fn sha256d(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let first = Sha256::digest(data);
    let second = Sha256::digest(first);
    let mut out = [0u8; 32];
    out.copy_from_slice(&second);
    out
}

/// Verifies a transaction's script signature.
pub fn verify_script(
    script_sig: &[u8],
    script_pubkey: &[u8],
    message: &[u8],
    registry: CryptoRegistry,
) -> Result<bool, ScriptError> {
    let mut interpreter = ScriptInterpreter::new(registry);

    // Execute scriptSig first
    interpreter.execute(script_sig, message)?;

    // Then execute scriptPubKey
    interpreter.execute(script_pubkey, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_push_and_verify_true() {
        let registry = CryptoRegistry::default();
        let mut interp = ScriptInterpreter::new(registry);

        // Script: push 1, verify it's true
        let script = vec![OpCode::True as u8];
        let result = interp
            .execute(&script, &[])
            .expect("Failed to execute script");

        assert!(result);
    }

    #[test]
    fn script_push_false() {
        let registry = CryptoRegistry::default();
        let mut interp = ScriptInterpreter::new(registry);

        let script = vec![OpCode::False as u8];
        let result = interp
            .execute(&script, &[])
            .expect("Failed to execute script");

        assert!(!result);
    }

    #[test]
    fn script_dup() {
        let registry = CryptoRegistry::default();
        let mut interp = ScriptInterpreter::new(registry);

        // Push 1, duplicate, verify stack has 2 items
        let script = vec![OpCode::True as u8, OpCode::Dup as u8];
        interp
            .execute(&script, &[])
            .expect("Failed to execute script");

        assert_eq!(interp.stack.len(), 2);
        assert_eq!(interp.stack[0], vec![1]);
        assert_eq!(interp.stack[1], vec![1]);
    }

    #[test]
    fn script_hash256() {
        let registry = CryptoRegistry::default();
        let mut interp = ScriptInterpreter::new(registry);

        // Push data, hash it
        let script = vec![
            0x04, // Push 4 bytes
            0x01,
            0x02,
            0x03,
            0x04,
            OpCode::Hash256 as u8,
        ];
        interp
            .execute(&script, &[])
            .expect("Failed to execute script");

        assert_eq!(interp.stack.len(), 1);
        assert_eq!(interp.stack[0].len(), 32); // SHA-256 hash
    }

    #[test]
    fn script_hash_blake3() {
        let registry = CryptoRegistry::default();
        let mut interp = ScriptInterpreter::new(registry);

        // Push data, hash it with BLAKE3
        let script = vec![
            0x04, // Push 4 bytes
            0x01,
            0x02,
            0x03,
            0x04,
            OpCode::HashBLAKE3 as u8,
        ];
        interp
            .execute(&script, &[])
            .expect("Failed to execute script");

        assert_eq!(interp.stack.len(), 1);
        assert_eq!(interp.stack[0].len(), 32); // BLAKE3 hash
    }

    #[test]
    fn script_too_large() {
        let registry = CryptoRegistry::default();
        let mut interp = ScriptInterpreter::new(registry);

        let script = vec![0x51; MAX_SCRIPT_SIZE + 1];
        let result = interp.execute(&script, &[]);

        assert!(matches!(result, Err(ScriptError::ScriptTooLarge(_))));
    }

    #[test]
    fn script_too_many_ops() {
        let registry = CryptoRegistry::default();
        let mut interp = ScriptInterpreter::new(registry);

        // Script with > MAX_OPS operations
        let script = vec![OpCode::True as u8; MAX_OPS + 1];
        let result = interp.execute(&script, &[]);

        assert!(matches!(result, Err(ScriptError::TooManyOps(_))));
    }

    #[test]
    fn stack_overflow() {
        let registry = CryptoRegistry::default();
        let mut interp = ScriptInterpreter::new(registry);

        // Try to push more than MAX_STACK_SIZE items
        for _ in 0..MAX_STACK_SIZE {
            interp.push(vec![1]).expect("Failed to push to stack");
        }

        let result = interp.push(vec![1]);
        assert!(matches!(result, Err(ScriptError::StackOverflow)));
    }
}
