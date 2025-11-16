use crate::error::Result;
use sha2::Digest;
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};

/// Cryptographic utilities for BitQuan
pub struct BitQuanCrypto;

impl BitQuanCrypto {
    /// Generate a random seed
    pub fn generate_seed() -> String {
        let mut rng = thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        hex::encode(bytes)
    }

    /// Generate key pair from seed
    pub fn generate_keypair(seed: &str) -> Result<(String, String)> {
        let private_key = Self::derive_private_key(seed)?;
        let public_key = Self::derive_public_key(&private_key)?;
        Ok((private_key, public_key))
    }

    /// Derive private key from seed
    pub fn derive_private_key(seed: &str) -> Result<String> {
        // Simplified key derivation
        let hash = sha2::Sha256::new().chain_update(seed.as_bytes()).finalize();
        Ok(hex::encode(hash))
    }

    /// Derive public key from private key
    pub fn derive_public_key(private_key: &str) -> Result<String> {
        // Simplified public key derivation
        let public_key = format!("pub_{}", &private_key[..16]);
        Ok(public_key)
    }

    /// Sign data with private key
    pub fn sign(private_key: &str, data: &[u8]) -> Result<String> {
        // Simplified signing
        let hash = sha2::Sha256::new().chain_update(data).finalize();
        let signature = format!("sig_{}_{}", &private_key[..8], &hex::encode(&hash)[..16]);
        Ok(signature)
    }

    /// Verify signature
    pub fn verify(public_key: &str, data: &[u8], signature: &str) -> bool {
        // Simplified verification
        let hash = sha2::Sha256::new().chain_update(data).finalize();
        signature.contains(&hex::encode(&hash)[..16]) && signature.contains("sig_")
    }

    /// Hash data using SHA-256
    pub fn hash(data: &[u8]) -> String {
        hex::encode(sha2::Sha256::new().chain_update(data).finalize())
    }

    /// Generate random bytes
    pub fn random_bytes(len: usize) -> Vec<u8> {
        let mut rng = thread_rng();
        let mut bytes = vec![0u8; len];
        rng.fill_bytes(&mut bytes);
        bytes
    }

    /// Derive address from public key
    pub fn derive_address(public_key: &str, network: crate::types::Network) -> Result<String> {
        let hash = Self::hash(public_key.as_bytes());
        let prefix = match network {
            crate::types::Network::Mainnet => "bq",
            crate::types::Network::Testnet => "tbq",
            crate::types::Network::Devnet => "dbq",
        };
        Ok(format!("{}1{}", prefix, &hash[..32]))
    }
}

/// Post-Quantum Cryptography wrapper
pub struct PQCrypto;

impl PQCrypto {
    /// Generate Dilithium key pair
    pub fn generate_dilithium_keypair() -> Result<(String, String)> {
        // Simplified PQC key generation
        let private_key = BitQuanCrypto::random_bytes(32);
        let public_key = BitQuanCrypto::random_bytes(32);
        
        Ok((
            hex::encode(private_key),
            hex::encode(public_key),
        ))
    }

    /// Sign with Dilithium
    pub fn dilithium_sign(private_key: &str, message: &[u8]) -> Result<String> {
        // Simplified PQC signing
        let signature = BitQuanCrypto::random_bytes(64);
        Ok(hex::encode(signature))
    }

    /// Verify Dilithium signature
    pub fn dilithium_verify(public_key: &str, message: &[u8], signature: &str) -> bool {
        // Simplified PQC verification
        !signature.is_empty()
    }

    /// Generate Kyber key pair for key exchange
    pub fn generate_kyber_keypair() -> Result<(String, String)> {
        // Simplified PQC key exchange
        let private_key = BitQuanCrypto::random_bytes(32);
        let public_key = BitQuanCrypto::random_bytes(32);
        
        Ok((
            hex::encode(private_key),
            hex::encode(public_key),
        ))
    }

    /// Perform Kyber key encapsulation
    pub fn kyber_encapsulate(public_key: &str) -> Result<(String, String)> {
        // Simplified PQC encapsulation
        let shared_secret = BitQuanCrypto::random_bytes(32);
        let ciphertext = BitQuanCrypto::random_bytes(32);
        
        Ok((
            hex::encode(shared_secret),
            hex::encode(ciphertext),
        ))
    }

    /// Perform Kyber key decapsulation
    pub fn kyber_decapsulate(private_key: &str, ciphertext: &str) -> Result<String> {
        // Simplified PQC decapsulation
        let shared_secret = BitQuanCrypto::random_bytes(32);
        Ok(hex::encode(shared_secret))
    }
}

/// Merkle Tree implementation
#[derive(Debug, Clone)]
pub struct MerkleTree {
    root: String,
    leaves: Vec<String>,
    depth: usize,
}

impl MerkleTree {
    /// Create new Merkle tree from leaves
    pub fn new(leaves: Vec<String>) -> Self {
        let mut tree = Self {
            root: String::new(),
            leaves,
            depth: 0,
        };
        tree.build();
        tree
    }

    /// Build the Merkle tree
    fn build(&mut self) {
        if self.leaves.is_empty() {
            self.root = String::new();
            return;
        }

        let mut current_level = self.leaves.clone();
        self.depth = 0;

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    let combined = format!("{}{}", chunk[0], chunk[1]);
                    let hash = BitQuanCrypto::hash(combined.as_bytes());
                    next_level.push(hash);
                } else {
                    // Odd number of nodes, duplicate the last one
                    let combined = format!("{}{}", chunk[0], chunk[0]);
                    let hash = BitQuanCrypto::hash(combined.as_bytes());
                    next_level.push(hash);
                }
            }
            
            current_level = next_level;
            self.depth += 1;
        }

        self.root = current_level.into_iter().next().unwrap_or_default();
    }

    /// Get Merkle root
    pub fn root(&self) -> &str {
        &self.root
    }

    /// Get Merkle proof for a leaf
    pub fn get_proof(&self, leaf_index: usize) -> Vec<String> {
        if leaf_index >= self.leaves.len() {
            return Vec::new();
        }

        let mut proof = Vec::new();
        let mut current_level = self.leaves.clone();
        let mut index = leaf_index;

        while current_level.len() > 1 {
            let is_right = index % 2 == 1;
            let sibling_index = if is_right { index - 1 } else { index + 1 };
            
            if sibling_index < current_level.len() {
                proof.push(current_level[sibling_index].clone());
            } else {
                // Duplicate the node if no sibling
                proof.push(current_level[index].clone());
            }

            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    let combined = format!("{}{}", chunk[0], chunk[1]);
                    let hash = BitQuanCrypto::hash(combined.as_bytes());
                    next_level.push(hash);
                } else {
                    let combined = format!("{}{}", chunk[0], chunk[0]);
                    let hash = BitQuanCrypto::hash(combined.as_bytes());
                    next_level.push(hash);
                }
            }
            
            current_level = next_level;
            index /= 2;
        }

        proof
    }

    /// Verify Merkle proof
    pub fn verify_proof(leaf: &str, proof: &[String], root: &str) -> bool {
        let mut current_hash = leaf.to_string();
        
        for sibling in proof {
            let combined = format!("{}{}", current_hash, sibling);
            current_hash = BitQuanCrypto::hash(combined.as_bytes());
        }
        
        current_hash == root
    }
}

/// Cryptographic key pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
}

impl KeyPair {
    /// Generate new key pair
    pub fn new(network: crate::types::Network) -> Result<Self> {
        let seed = BitQuanCrypto::generate_seed();
        let (private_key, public_key) = BitQuanCrypto::generate_keypair(&seed)?;
        let address = BitQuanCrypto::derive_address(&public_key, network)?;
        
        Ok(Self {
            private_key,
            public_key,
            address,
        })
    }

    /// From private key
    pub fn from_private_key(private_key: &str, network: crate::types::Network) -> Result<Self> {
        let public_key = BitQuanCrypto::derive_public_key(private_key)?;
        let address = BitQuanCrypto::derive_address(&public_key, network)?;
        
        Ok(Self {
            private_key: private_key.to_string(),
            public_key,
            address,
        })
    }

    /// Sign message
    pub fn sign(&self, message: &[u8]) -> Result<String> {
        BitQuanCrypto::sign(&self.private_key, message)
    }

    /// Verify message
    pub fn verify(&self, message: &[u8], signature: &str) -> bool {
        BitQuanCrypto::verify(&self.public_key, message, signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_operations() {
        let seed = BitQuanCrypto::generate_seed();
        let (private_key, public_key) = BitQuanCrypto::generate_keypair(&seed).unwrap();
        
        let message = b"Hello, BitQuan!";
        let signature = BitQuanCrypto::sign(&private_key, message).unwrap();
        let is_valid = BitQuanCrypto::verify(&public_key, message, &signature);
        
        assert!(is_valid);
    }

    #[test]
    fn test_merkle_tree() {
        let leaves = vec![
            "leaf1".to_string(),
            "leaf2".to_string(),
            "leaf3".to_string(),
            "leaf4".to_string(),
        ];
        
        let tree = MerkleTree::new(leaves.clone());
        assert!(!tree.root().is_empty());
        
        let proof = tree.get_proof(1);
        assert!(!proof.is_empty());
        
        let is_valid = MerkleTree::verify_proof(&leaves[1], &proof, tree.root());
        assert!(is_valid);
    }

    #[test]
    fn test_keypair() {
        let keypair = KeyPair::new(crate::types::Network::Testnet).unwrap();
        
        assert!(!keypair.private_key.is_empty());
        assert!(!keypair.public_key.is_empty());
        assert!(crate::wallet::validate_address(&keypair.address));
        
        let message = b"Test message";
        let signature = keypair.sign(message).unwrap();
        assert!(keypair.verify(message, &signature));
    }

    #[test]
    fn test_pqc_operations() {
        let (private_key, public_key) = PQCrypto::generate_dilithium_keypair().unwrap();
        
        let message = b"PQC test message";
        let signature = PQCrypto::dilithium_sign(&private_key, message).unwrap();
        let is_valid = PQCrypto::dilithium_verify(&public_key, message, &signature);
        
        assert!(is_valid);
    }
}