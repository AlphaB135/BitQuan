#![no_main]

use bitquan_types::{SigAlgorithm, SignaturePayload};
use bq_crypto::CryptoRegistry;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz Dilithium signature verification
    if !data.is_empty() && data.len() <= 1_000_000 {
        let registry = CryptoRegistry::new();

        // Split data into signature, message, and public key
        if data.len() >= 100 {
            let sig_len = 4595; // Dilithium5 signature size
            let pk_len = 2592; // Dilithium5 public key size

            let mut signature = [0u8; 4595];
            let mut public_key = [0u8; 2592];
            let mut message = Vec::new();

            // Fill with fuzz data
            let sig_end = sig_len.min(data.len());
            signature[..sig_end].copy_from_slice(&data[..sig_end]);

            if data.len() > sig_len {
                let pk_end = sig_len + pk_len.min(data.len() - sig_len);
                let pk_actual_len = pk_end - sig_len;
                public_key[..pk_actual_len].copy_from_slice(&data[sig_len..pk_end]);

                if data.len() > pk_end {
                    message = data[pk_end..].to_vec();
                }
            }

            // Test verification doesn't panic
            if let Some(provider) = registry.provider_for(SigAlgorithm::Dilithium5) {
                let payload = SignaturePayload {
                    signer_index: 0,
                    signature: signature.to_vec(),
                    public_key: public_key.to_vec(),
                    aux: None,
                };
                let _ = provider.verify(&payload, &message);
            }
        }
    }

    // Fuzz malformed signature sizes
    if data.len() >= 4 {
        let mut malformed_sig = [0u8; 4000];
        let sig_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let actual_len = sig_len.min(malformed_sig.len()).min(data.len() - 4);

        if data.len() > 4 {
            malformed_sig[..actual_len].copy_from_slice(&data[4..4 + actual_len]);
        }

        let registry = CryptoRegistry::new();
        let public_key = [0u8; 2592];
        let message = b"test message";

        // Should handle malformed signatures gracefully
        if let Some(provider) = registry.provider_for(SigAlgorithm::Dilithium5) {
            let payload = SignaturePayload {
                signer_index: 0,
                signature: malformed_sig[..actual_len].to_vec(),
                public_key: public_key.to_vec(),
                aux: None,
            };
            let _ = provider.verify(&payload, message);
        }
    }

    // Fuzz oversized messages
    if data.len() > 1_000_000 {
        let registry = CryptoRegistry::new();
        let signature = [0u8; 4595];
        let public_key = [0u8; 2592];

        // Should handle oversized messages
        if let Some(provider) = registry.provider_for(SigAlgorithm::Dilithium5) {
            let payload = SignaturePayload {
                signer_index: 0,
                signature: signature.to_vec(),
                public_key: public_key.to_vec(),
                aux: None,
            };
            let _ = provider.verify(&payload, &data[..1_000_000]); // Truncate to 1MB
        }
    }
});
