use async_trait::async_trait;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::{
    models::{
        Chain, ChainMetrics, DegenMetrics, TokenBalance, NFTBalance,
        TransactionSummary, DegenScoreError, Result,
    },
    chains::{ChainClient, client::{ProtocolMetrics, ChainClientConfig}},
};

/// Mock Solana client for demo purposes
/// In production, this would use the actual solana-client crate
pub struct SolanaClient {
    chain: Chain,
}

impl SolanaClient {
    pub fn new(_config: ChainClientConfig) -> Result<Self> {
        Ok(Self {
            chain: Chain::Solana,
        })
    }
}

#[async_trait]
impl ChainClient for SolanaClient {
    fn chain(&self) -> Chain {
        self.chain.clone()
    }
    
    async fn fetch_metrics(&self, address: &str) -> Result<ChainMetrics> {
        // Validate Solana address format
        if address.len() < 32 || address.len() > 44 {
            return Err(DegenScoreError::InvalidAddress(address.to_string()));
        }
        
        // For demo: return mock data
        let mut metrics = DegenMetrics::default();
        
        // Simulate some activity for known addresses
        if address.contains("7VXN") || address.contains("demo") {
            metrics.jupiter_swaps = 25;
            metrics.distinct_tokens_traded = 15;
            metrics.nft_count = 5;
            metrics.total_tx_count = 150;
            metrics.wallet_age_days = 400;
            metrics.active_days = 120;
            metrics.chains_active_on.push("solana".to_string());
            
            // Add some mock casino token holdings
            metrics.casino_tokens_held.insert("DICE".to_string(), Decimal::from(1000));
            metrics.casinos_used = 1;
        }
        
        Ok(ChainMetrics {
            chain: "solana".to_string(),
            address: address.to_string(),
            metrics,
            last_updated: Utc::now(),
        })
    }
    
    async fn get_transaction_summary(
        &self,
        address: &str,
        _start_time: Option<DateTime<Utc>>,
        _end_time: Option<DateTime<Utc>>
    ) -> Result<TransactionSummary> {
        Ok(TransactionSummary {
            total_count: 150,
            first_tx: None,
            last_tx: None,
            active_days: 120,
            average_tx_per_day: 0.375,
            gas_spent: Decimal::ZERO,
        })
    }
    
    async fn get_token_balances(&self, address: &str) -> Result<Vec<TokenBalance>> {
        let mut balances = Vec::new();
        
        // Mock SOL balance
        balances.push(TokenBalance {
            token_address: "11111111111111111111111111111111".to_string(),
            balance: ethers::types::U256::from(5_000_000_000u64), // 5 SOL
            decimals: 9,
            symbol: "SOL".to_string(),
            name: "Solana".to_string(),
        });
        
        Ok(balances)
    }
    
    async fn get_nft_balances(&self, _address: &str) -> Result<Vec<NFTBalance>> {
        Ok(vec![])
    }
    
    async fn has_used_protocol(&self, _address: &str, protocol_address: &str) -> Result<bool> {
        // Mock Jupiter usage
        Ok(protocol_address.contains("JUP"))
    }
    
    async fn get_protocol_metrics(
        &self,
        _address: &str,
        protocol: &str
    ) -> Result<ProtocolMetrics> {
        match protocol.to_lowercase().as_str() {
            "jupiter" => Ok(ProtocolMetrics {
                protocol_name: "Jupiter".to_string(),
                interaction_count: 25,
                volume_usd: Decimal::from(50000),
                first_interaction: None,
                last_interaction: None,
                custom_metrics: HashMap::new(),
            }),
            _ => Ok(ProtocolMetrics {
                protocol_name: protocol.to_string(),
                interaction_count: 0,
                volume_usd: Decimal::ZERO,
                first_interaction: None,
                last_interaction: None,
                custom_metrics: HashMap::new(),
            }),
        }
    }
    
    fn validate_address(&self, address: &str) -> Result<()> {
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
        
        Ok(())
    }
}