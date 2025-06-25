use async_trait::async_trait;
use crate::models::{
    Chain, DegenMetrics, ChainMetrics, TokenBalance, NFTBalance, 
    TransactionSummary, Result
};
use chrono::{DateTime, Utc};

#[async_trait]
pub trait ChainClient: Send + Sync {
    /// Get the chain this client is for
    fn chain(&self) -> Chain;
    
    /// Fetch all metrics for a given address
    async fn fetch_metrics(&self, address: &str) -> Result<ChainMetrics>;
    
    /// Get transaction summary for an address
    async fn get_transaction_summary(
        &self, 
        address: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>
    ) -> Result<TransactionSummary>;
    
    /// Get token balances for an address
    async fn get_token_balances(&self, address: &str) -> Result<Vec<TokenBalance>>;
    
    /// Get NFT balances for an address
    async fn get_nft_balances(&self, address: &str) -> Result<Vec<NFTBalance>>;
    
    /// Check if an address has interacted with a specific protocol
    async fn has_used_protocol(&self, address: &str, protocol_address: &str) -> Result<bool>;
    
    /// Get protocol-specific metrics (e.g., GMX trading volume)
    async fn get_protocol_metrics(
        &self, 
        address: &str, 
        protocol: &str
    ) -> Result<ProtocolMetrics>;
    
    /// Validate if an address is valid for this chain
    fn validate_address(&self, address: &str) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct ProtocolMetrics {
    pub protocol_name: String,
    pub interaction_count: u32,
    pub volume_usd: rust_decimal::Decimal,
    pub first_interaction: Option<DateTime<Utc>>,
    pub last_interaction: Option<DateTime<Utc>>,
    pub custom_metrics: std::collections::HashMap<String, serde_json::Value>,
}

/// Configuration for chain clients
#[derive(Debug, Clone)]
pub struct ChainClientConfig {
    pub rpc_url: String,
    pub chain_id: Option<u64>,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub rate_limit_per_second: f64,
}