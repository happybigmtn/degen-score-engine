use crate::models::{Result, DegenScoreError};
use ethers::core::types::{Address as EthAddress, Signature, RecoveryMessage};
use ethers::utils::hash_message;
use ring::signature::{self, UnparsedPublicKey};
use hex;
use std::str::FromStr;

/// Verifies Ethereum/EVM wallet signatures
pub struct EvmSignatureVerifier;

impl EvmSignatureVerifier {
    /// Verify an EIP-191 personal signature
    pub fn verify_signature(
        address: &str,
        message: &str,
        signature: &str,
    ) -> Result<bool> {
        // Parse the expected address
        let expected_address = EthAddress::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        
        // Parse the signature (remove 0x prefix if present)
        let sig_str = signature.strip_prefix("0x").unwrap_or(signature);
        let sig_bytes = hex::decode(sig_str)
            .map_err(|e| DegenScoreError::SignatureVerificationFailed(
                format!("Invalid signature hex: {}", e)
            ))?;
        
        if sig_bytes.len() != 65 {
            return Err(DegenScoreError::SignatureVerificationFailed(
                "Signature must be 65 bytes".to_string()
            ));
        }
        
        // Create signature from bytes
        let signature = Signature::try_from(sig_bytes.as_slice())
            .map_err(|e| DegenScoreError::SignatureVerificationFailed(
                format!("Invalid signature format: {}", e)
            ))?;
        
        // Hash the message according to EIP-191
        let message_hash = hash_message(message);
        
        // Recover the signer address
        let recovered = signature.recover(RecoveryMessage::Hash(message_hash))
            .map_err(|e| DegenScoreError::SignatureVerificationFailed(
                format!("Failed to recover address: {}", e)
            ))?;
        
        // Compare addresses
        Ok(recovered == expected_address)
    }
    
    /// Generate the message to be signed for verification
    pub fn generate_message(address: &str, nonce: &str) -> String {
        format!(
            "I verify that I own wallet {} for Craps Anchor Degen Score (nonce: {}). \
            This signature does NOT grant any permissions or approvals.",
            address, nonce
        )
    }
}

/// Verifies Solana wallet signatures
pub struct SolanaSignatureVerifier;

impl SolanaSignatureVerifier {
    /// Verify an Ed25519 signature from a Solana wallet
    pub fn verify_signature(
        address: &str,
        message: &str,
        signature: &str,
    ) -> Result<bool> {
        // Decode the public key from base58
        let pubkey_bytes = bs58::decode(address)
            .into_vec()
            .map_err(|e| DegenScoreError::InvalidAddress(
                format!("Invalid Solana address: {}", e)
            ))?;
        
        if pubkey_bytes.len() != 32 {
            return Err(DegenScoreError::InvalidAddress(
                "Solana address must be 32 bytes".to_string()
            ));
        }
        
        // Decode the signature from base58
        let sig_bytes = bs58::decode(signature)
            .into_vec()
            .map_err(|e| DegenScoreError::SignatureVerificationFailed(
                format!("Invalid signature encoding: {}", e)
            ))?;
        
        if sig_bytes.len() != 64 {
            return Err(DegenScoreError::SignatureVerificationFailed(
                "Ed25519 signature must be 64 bytes".to_string()
            ));
        }
        
        // Verify using ring
        let public_key = UnparsedPublicKey::new(&signature::ED25519, &pubkey_bytes);
        
        match public_key.verify(message.as_bytes(), &sig_bytes) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Generate the message to be signed for verification
    pub fn generate_message(address: &str, nonce: &str) -> String {
        // Same format as EVM for consistency
        EvmSignatureVerifier::generate_message(address, nonce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_evm_message_generation() {
        let msg = EvmSignatureVerifier::generate_message(
            "0x742d35Cc6634C0532925a3b844Bc9e7595f6e842",
            "12345"
        );
        
        assert!(msg.contains("0x742d35Cc6634C0532925a3b844Bc9e7595f6e842"));
        assert!(msg.contains("12345"));
        assert!(msg.contains("does NOT grant any permissions"));
    }
    
    #[test]
    fn test_invalid_evm_address() {
        let result = EvmSignatureVerifier::verify_signature(
            "invalid_address",
            "test message",
            "0x1234"
        );
        
        assert!(result.is_err());
        match result {
            Err(DegenScoreError::InvalidAddress(_)) => {},
            _ => panic!("Expected InvalidAddress error"),
        }
    }
    
    #[test]
    fn test_invalid_signature_length() {
        let result = EvmSignatureVerifier::verify_signature(
            "0x742d35Cc6634C0532925a3b844Bc9e7595f6e842",
            "test message",
            "0x1234" // Too short
        );
        
        assert!(result.is_err());
        match result {
            Err(DegenScoreError::SignatureVerificationFailed(_)) => {},
            _ => panic!("Expected SignatureVerificationFailed error"),
        }
    }
}