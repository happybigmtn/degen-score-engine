use crate::{
    models::{
        Result, DegenScoreError, Chain, UserProfile, VerifiedAddress, 
        VerificationMethod, AddressVerificationRequest
    },
    verification::{EvmSignatureVerifier, SolanaSignatureVerifier, DepositVerifier},
    chains::{ChainClient, EvmClient, SolanaClient},
};
use chrono::Utc;
use tracing::{info, warn, error};
use std::sync::Arc;

/// Main wallet verification coordinator
pub struct WalletVerifier {
    deposit_verifier: DepositVerifier,
}

impl WalletVerifier {
    pub fn new() -> Self {
        Self {
            deposit_verifier: DepositVerifier::new(Default::default()),
        }
    }
    
    /// Verify ownership of a wallet address using signature
    pub async fn verify_with_signature(
        &self,
        request: AddressVerificationRequest,
        signature: String,
    ) -> Result<VerifiedAddress> {
        info!("Verifying {} address {} with signature", 
            request.chain.as_str(), request.address);
        
        let message = match request.chain {
            Chain::Ethereum | Chain::Arbitrum | Chain::Optimism | Chain::Blast => {
                EvmSignatureVerifier::generate_message(&request.address, &request.nonce)
            },
            Chain::Solana => {
                SolanaSignatureVerifier::generate_message(&request.address, &request.nonce)
            },
        };
        
        let is_valid = match request.chain {
            Chain::Ethereum | Chain::Arbitrum | Chain::Optimism | Chain::Blast => {
                EvmSignatureVerifier::verify_signature(
                    &request.address,
                    &message,
                    &signature,
                )?
            },
            Chain::Solana => {
                SolanaSignatureVerifier::verify_signature(
                    &request.address,
                    &message,
                    &signature,
                )?
            },
        };
        
        if !is_valid {
            return Err(DegenScoreError::SignatureVerificationFailed(
                "Invalid signature".to_string()
            ));
        }
        
        Ok(VerifiedAddress {
            address: request.address,
            chain: request.chain,
            verification_method: VerificationMethod::Signature {
                message: message.clone(),
                signature,
            },
            verified_at: Utc::now(),
            nonce: request.nonce,
        })
    }
    
    /// Verify ownership of a wallet address using micro-deposit
    pub async fn verify_with_deposit(
        &self,
        request: AddressVerificationRequest,
        chain_client: Arc<dyn ChainClient>,
    ) -> Result<VerifiedAddress> {
        info!("Setting up deposit verification for {} address {}", 
            request.chain.as_str(), request.address);
        
        let deposit_address = DepositVerifier::generate_deposit_address(&request.chain)?;
        let reference = DepositVerifier::generate_reference("user", &request.address);
        
        info!("Please send a small amount to {} with reference: {}", 
            deposit_address, reference);
        
        let start_time = Utc::now();
        
        // Wait for deposit based on chain type
        let tx_hash = match &request.chain {
            Chain::Ethereum | Chain::Arbitrum | Chain::Optimism | Chain::Blast => {
                // This would need the actual EVM client instance
                // For now, returning error as full implementation would require
                // transaction monitoring
                return Err(DegenScoreError::SignatureVerificationFailed(
                    "Deposit verification not fully implemented".to_string()
                ));
            },
            Chain::Solana => {
                // Similar for Solana
                return Err(DegenScoreError::SignatureVerificationFailed(
                    "Deposit verification not fully implemented".to_string()
                ));
            },
        };
        
        // If we get here, deposit was verified
        Ok(VerifiedAddress {
            address: request.address,
            chain: request.chain,
            verification_method: VerificationMethod::MicroDeposit {
                tx_hash,
                amount: self.deposit_verifier.config.min_amount.to_string(),
            },
            verified_at: Utc::now(),
            nonce: request.nonce,
        })
    }
    
    /// Add a verified address to a user profile
    pub fn add_verified_address_to_profile(
        &self,
        user: &mut UserProfile,
        verified_address: VerifiedAddress,
    ) -> Result<()> {
        // Check if address is already verified
        if user.verified_addresses.iter().any(|a| {
            a.address == verified_address.address && a.chain == verified_address.chain
        }) {
            return Err(DegenScoreError::ConfigError(
                "Address already verified for this user".to_string()
            ));
        }
        
        user.add_verified_address(verified_address);
        info!("Added verified address to user {}", user.id);
        
        Ok(())
    }
    
    /// Generate a new verification request
    pub fn create_verification_request(
        chain: Chain,
        address: String,
    ) -> AddressVerificationRequest {
        AddressVerificationRequest {
            address,
            chain,
            nonce: Self::generate_nonce(),
            timestamp: Utc::now(),
        }
    }
    
    /// Generate a random nonce for verification
    fn generate_nonce() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let nonce: u64 = rng.gen();
        nonce.to_string()
    }
    
    /// Validate that an address format is correct for the chain
    pub fn validate_address_format(chain: &Chain, address: &str) -> Result<()> {
        match chain {
            Chain::Ethereum | Chain::Arbitrum | Chain::Optimism | Chain::Blast => {
                if !address.starts_with("0x") || address.len() != 42 {
                    return Err(DegenScoreError::InvalidAddress(
                        format!("Invalid EVM address format: {}", address)
                    ));
                }
                
                // Try to parse as hex
                let without_prefix = &address[2..];
                hex::decode(without_prefix)
                    .map_err(|_| DegenScoreError::InvalidAddress(
                        format!("Invalid hex in address: {}", address)
                    ))?;
            },
            Chain::Solana => {
                // Solana addresses are base58 encoded and 32-44 characters
                if address.len() < 32 || address.len() > 44 {
                    return Err(DegenScoreError::InvalidAddress(
                        format!("Invalid Solana address length: {}", address)
                    ));
                }
                
                // Try to decode base58
                bs58::decode(address)
                    .into_vec()
                    .map_err(|_| DegenScoreError::InvalidAddress(
                        format!("Invalid base58 in address: {}", address)
                    ))?;
            },
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_evm_address() {
        let verifier = WalletVerifier::new();
        
        // Valid address
        assert!(verifier.validate_address_format(
            &Chain::Ethereum,
            "0x742d35Cc6634C0532925a3b844Bc9e7595f6e842"
        ).is_ok());
        
        // Invalid - no 0x prefix
        assert!(verifier.validate_address_format(
            &Chain::Ethereum,
            "742d35Cc6634C0532925a3b844Bc9e7595f6e842"
        ).is_err());
        
        // Invalid - wrong length
        assert!(verifier.validate_address_format(
            &Chain::Ethereum,
            "0x742d35Cc6634C0532925a3b844Bc9e7595f6e8"
        ).is_err());
        
        // Invalid - not hex
        assert!(verifier.validate_address_format(
            &Chain::Ethereum,
            "0xGGGG35Cc6634C0532925a3b844Bc9e7595f6e842"
        ).is_err());
    }
    
    #[test]
    fn test_validate_solana_address() {
        let verifier = WalletVerifier::new();
        
        // Valid address
        assert!(verifier.validate_address_format(
            &Chain::Solana,
            "7VXNK6XaXQPZnqVwGHXBuCfLj9jfJzRy3aqf9PCYizv"
        ).is_ok());
        
        // Invalid - too short
        assert!(verifier.validate_address_format(
            &Chain::Solana,
            "7VXNK6XaXQPZ"
        ).is_err());
        
        // Invalid - too long
        assert!(verifier.validate_address_format(
            &Chain::Solana,
            "7VXNK6XaXQPZnqVwGHXBuCfLj9jfJzRy3aqf9PCYizvTooLongAddress"
        ).is_err());
    }
    
    #[test]
    fn test_nonce_generation() {
        let nonce1 = WalletVerifier::generate_nonce();
        let nonce2 = WalletVerifier::generate_nonce();
        
        // Nonces should be different
        assert_ne!(nonce1, nonce2);
        
        // Nonces should be numeric strings
        assert!(nonce1.parse::<u64>().is_ok());
        assert!(nonce2.parse::<u64>().is_ok());
    }
}