use async_trait::async_trait;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use reqwest;
use std::str::FromStr;

use crate::{
    models::{
        Chain, ChainMetrics, DegenMetrics, TokenBalance, NFTBalance,
        TransactionSummary, DegenScoreError, Result,
    },
    chains::{ChainClient, client::{ProtocolMetrics, ChainClientConfig}},
};

/// Solana RPC client using direct JSON-RPC calls to avoid dependency conflicts
pub struct SolanaRpcClient {
    http_client: reqwest::Client,
    rpc_url: String,
    chain: Chain,
}

#[derive(Serialize)]
struct RpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Deserialize)]
struct RpcResponse<T> {
    jsonrpc: String,
    result: Option<T>,
    error: Option<RpcError>,
    id: u64,
}

#[derive(Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Deserialize)]
struct GetSignaturesForAddressResult {
    signature: String,
    slot: u64,
    #[serde(rename = "blockTime")]
    block_time: Option<i64>,
    err: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct GetAccountInfoResult {
    value: Option<AccountInfo>,
}

#[derive(Deserialize)]
struct AccountInfo {
    lamports: u64,
    owner: String,
    data: serde_json::Value,
    executable: bool,
    #[serde(rename = "rentEpoch")]
    rent_epoch: u64,
}

#[derive(Deserialize)]
struct GetTokenAccountsByOwnerResult {
    value: Vec<TokenAccount>,
}

#[derive(Deserialize)]
struct TokenAccount {
    pubkey: String,
    account: AccountData,
}

#[derive(Deserialize)]
struct AccountData {
    data: ParsedAccountData,
    lamports: u64,
    owner: String,
}

#[derive(Deserialize)]
struct ParsedAccountData {
    parsed: TokenAccountInfo,
}

#[derive(Deserialize)]
struct TokenAccountInfo {
    info: TokenInfo,
}

#[derive(Deserialize)]
struct TokenInfo {
    mint: String,
    owner: String,
    #[serde(rename = "tokenAmount")]
    token_amount: TokenAmount,
}

#[derive(Deserialize)]
struct TokenAmount {
    amount: String,
    decimals: u8,
    #[serde(rename = "uiAmount")]
    ui_amount: Option<f64>,
    #[serde(rename = "uiAmountString")]
    ui_amount_string: String,
}

impl SolanaRpcClient {
    pub fn new(config: ChainClientConfig) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| DegenScoreError::ConfigError(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self {
            http_client,
            rpc_url: config.rpc_url,
            chain: Chain::Solana,
        })
    }
    
    async fn make_rpc_request<T: for<'de> Deserialize<'de>>(&self, method: &str, params: serde_json::Value) -> Result<T> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: method.to_string(),
            params,
        };
        
        let response = self.http_client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;
        
        let rpc_response: RpcResponse<T> = response.json().await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;
        
        if let Some(error) = rpc_response.error {
            return Err(DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("RPC error {}: {}", error.code, error.message),
            });
        }
        
        rpc_response.result.ok_or_else(|| DegenScoreError::RpcError {
            chain: self.chain.as_str().to_string(),
            message: "Empty result from RPC".to_string(),
        })
    }
    
    async fn get_signatures_for_address(&self, address: &str, limit: usize) -> Result<Vec<GetSignaturesForAddressResult>> {
        let params = json!([
            address,
            {
                "limit": limit,
                "commitment": "confirmed"
            }
        ]);
        
        self.make_rpc_request("getSignaturesForAddress", params).await
    }
    
    async fn get_account_info(&self, address: &str) -> Result<GetAccountInfoResult> {
        let params = json!([
            address,
            {
                "encoding": "jsonParsed",
                "commitment": "confirmed"
            }
        ]);
        
        self.make_rpc_request("getAccountInfo", params).await
    }
    
    async fn get_token_accounts_by_owner(&self, address: &str) -> Result<GetTokenAccountsByOwnerResult> {
        let params = json!([
            address,
            {
                "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            },
            {
                "encoding": "jsonParsed"
            }
        ]);
        
        self.make_rpc_request("getTokenAccountsByOwner", params).await
    }
    
    fn validate_solana_address(address: &str) -> Result<()> {
        // Basic validation: Solana addresses are base58 encoded and 32-44 characters
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

#[async_trait]
impl ChainClient for SolanaRpcClient {
    fn chain(&self) -> Chain {
        self.chain.clone()
    }
    
    async fn fetch_metrics(&self, address: &str) -> Result<ChainMetrics> {
        // Validate address
        Self::validate_solana_address(address)?;
        
        let mut metrics = DegenMetrics::default();
        
        // Get account info
        let account_info = self.get_account_info(address).await?;
        if let Some(info) = account_info.value {
            println!("Solana address {} has {} SOL", address, info.lamports as f64 / 1e9);
        }
        
        // Get transaction signatures
        let signatures = self.get_signatures_for_address(address, 1000).await?;
        metrics.total_tx_count = signatures.len() as u32;
        println!("Total transactions: {}", metrics.total_tx_count);
        
        // Calculate wallet age from oldest transaction
        if let Some(oldest) = signatures.last() {
            if let Some(block_time) = oldest.block_time {
                let tx_time = DateTime::from_timestamp(block_time, 0)
                    .unwrap_or_else(|| Utc::now());
                let age = Utc::now().signed_duration_since(tx_time);
                metrics.wallet_age_days = age.num_days().max(0) as u32;
                println!("Wallet age: {} days", metrics.wallet_age_days);
            }
        }
        
        // Count active days
        let mut active_days = std::collections::HashSet::new();
        for sig in &signatures {
            if let Some(block_time) = sig.block_time {
                let day = block_time / 86400; // Convert to days since epoch
                active_days.insert(day);
            }
        }
        metrics.active_days = active_days.len() as u32;
        
        // Check for Jupiter interactions (simplified - just count recent txs as potential swaps)
        // In production, we would parse transaction details
        let jupiter_program_v6 = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
        let mut jupiter_count = 0;
        
        // For now, estimate based on transaction count (simplified)
        if metrics.total_tx_count > 50 {
            jupiter_count = (metrics.total_tx_count / 20).min(50); // Rough estimate
            metrics.jupiter_swaps = jupiter_count;
            metrics.defi_protocols_used += 1;
        }
        
        // Get SPL token accounts
        match self.get_token_accounts_by_owner(address).await {
            Ok(token_accounts) => {
                let token_count = token_accounts.value.len();
                metrics.distinct_tokens_traded = token_count as u32;
                println!("Found {} SPL token accounts", token_count);
                
                // Check for NFTs (tokens with amount = 1 and decimals = 0)
                let mut nft_count = 0;
                for token_account in &token_accounts.value {
                    let token_info = &token_account.account.data.parsed.info;
                    let amount = &token_info.token_amount;
                    
                    if amount.amount == "1" && amount.decimals == 0 {
                        nft_count += 1;
                    }
                    
                    // Check for casino tokens (simplified check)
                    if token_info.mint.contains("dice") || token_info.mint.contains("DICE") {
                        if let Ok(balance) = Decimal::from_str(&amount.ui_amount_string) {
                            metrics.casino_tokens_held.insert("DICE".to_string(), balance);
                            metrics.casinos_used = 1;
                        }
                    }
                }
                
                metrics.nft_count = nft_count;
                println!("Found {} NFTs", nft_count);
            }
            Err(e) => {
                println!("Failed to get token accounts: {}", e);
            }
        }
        
        // Add Solana to active chains
        metrics.chains_active_on.push("solana".to_string());
        
        println!("Solana metrics fetched successfully");
        println!("- Wallet age: {} days", metrics.wallet_age_days);
        println!("- Active days: {}", metrics.active_days);
        println!("- Jupiter swaps (estimated): {}", metrics.jupiter_swaps);
        println!("- Distinct tokens: {}", metrics.distinct_tokens_traded);
        println!("- NFT count: {}", metrics.nft_count);
        
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
        Self::validate_solana_address(address)?;
        
        let signatures = self.get_signatures_for_address(address, 1000).await?;
        let total_count = signatures.len() as u32;
        
        let mut first_tx = None;
        let mut last_tx = None;
        
        if let Some(newest) = signatures.first() {
            if let Some(block_time) = newest.block_time {
                last_tx = DateTime::from_timestamp(block_time, 0);
            }
        }
        
        if let Some(oldest) = signatures.last() {
            if let Some(block_time) = oldest.block_time {
                first_tx = DateTime::from_timestamp(block_time, 0);
            }
        }
        
        let active_days = if let (Some(first), Some(last)) = (first_tx, last_tx) {
            (last.signed_duration_since(first).num_days() as u32).max(1)
        } else {
            0
        };
        
        let average_tx_per_day = if active_days > 0 {
            total_count as f64 / active_days as f64
        } else {
            0.0
        };
        
        Ok(TransactionSummary {
            total_count,
            first_tx,
            last_tx,
            active_days,
            average_tx_per_day,
            gas_spent: Decimal::ZERO,
        })
    }
    
    async fn get_token_balances(&self, address: &str) -> Result<Vec<TokenBalance>> {
        Self::validate_solana_address(address)?;
        
        let mut balances = Vec::new();
        
        // Get SOL balance
        let account_info = self.get_account_info(address).await?;
        if let Some(info) = account_info.value {
            balances.push(TokenBalance {
                token_address: "11111111111111111111111111111111".to_string(),
                balance: ethers::types::U256::from(info.lamports),
                decimals: 9,
                symbol: "SOL".to_string(),
                name: "Solana".to_string(),
            });
        }
        
        // Get SPL token balances
        if let Ok(token_accounts) = self.get_token_accounts_by_owner(address).await {
            for token_account in token_accounts.value {
                let token_info = &token_account.account.data.parsed.info;
                let amount = &token_info.token_amount;
                
                if let Ok(balance) = ethers::types::U256::from_dec_str(&amount.amount) {
                    balances.push(TokenBalance {
                        token_address: token_info.mint.clone(),
                        balance,
                        decimals: amount.decimals,
                        symbol: token_info.mint[..4].to_string(), // Simplified
                        name: "SPL Token".to_string(),
                    });
                }
            }
        }
        
        Ok(balances)
    }
    
    async fn get_nft_balances(&self, _address: &str) -> Result<Vec<NFTBalance>> {
        // Simplified - would need Metaplex integration for full NFT metadata
        Ok(vec![])
    }
    
    async fn has_used_protocol(&self, address: &str, protocol_address: &str) -> Result<bool> {
        Self::validate_solana_address(address)?;
        
        // Simplified check - just see if address has many transactions
        if protocol_address.contains("JUP") {
            let signatures = self.get_signatures_for_address(address, 100).await?;
            return Ok(signatures.len() > 10); // Simple heuristic
        }
        
        Ok(false)
    }
    
    async fn get_protocol_metrics(
        &self,
        address: &str,
        protocol: &str
    ) -> Result<ProtocolMetrics> {
        Self::validate_solana_address(address)?;
        
        match protocol.to_lowercase().as_str() {
            "jupiter" => {
                // Simplified metrics based on transaction count
                let signatures = self.get_signatures_for_address(address, 1000).await?;
                let swap_count = (signatures.len() as u32 / 20).min(50);
                
                Ok(ProtocolMetrics {
                    protocol_name: "Jupiter".to_string(),
                    interaction_count: swap_count,
                    volume_usd: Decimal::from(swap_count) * Decimal::from(1000), // Estimate $1000 per swap
                    first_interaction: None,
                    last_interaction: None,
                    custom_metrics: HashMap::new(),
                })
            },
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
        Self::validate_solana_address(address)
    }
}