use async_trait::async_trait;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
    commitment_config::CommitmentConfig,
};
use solana_transaction_status::{
    UiTransactionEncoding, TransactionDetails,
    option_serializer::OptionSerializer,
};
use spl_token::id as token_program_id;
use std::str::FromStr;
use std::collections::HashMap;
use chrono::{DateTime, Utc, TimeZone};
use rust_decimal::Decimal;

use crate::{
    models::{
        Chain, ChainMetrics, DegenMetrics, TokenBalance, NFTBalance,
        TransactionSummary, DegenScoreError, Result,
        SolanaTransaction, SolanaInstruction,
        chain_data::ProtocolAddresses,
    },
    chains::{ChainClient, ProtocolMetrics, ChainClientConfig},
};

pub struct SolanaClient {
    rpc_client: RpcClient,
    chain: Chain,
}

impl SolanaClient {
    pub fn new(config: ChainClientConfig) -> Result<Self> {
        let rpc_client = RpcClient::new_with_timeout(
            config.rpc_url,
            std::time::Duration::from_secs(config.timeout_seconds),
        );
        
        Ok(Self {
            rpc_client,
            chain: Chain::Solana,
        })
    }
    
    async fn get_recent_signatures(&self, pubkey: &Pubkey, limit: usize) -> Result<Vec<Signature>> {
        let signatures = self.rpc_client
            .get_signatures_for_address_with_config(
                pubkey,
                solana_client::rpc_config::RpcGetConfirmedSignaturesForAddress2Config {
                    before: None,
                    until: None,
                    limit: Some(limit),
                    commitment: Some(CommitmentConfig::confirmed()),
                },
            )
            .map_err(|e| DegenScoreError::RpcError {
                chain: "solana".to_string(),
                message: format!("Failed to get signatures: {}", e),
            })?;
        
        Ok(signatures
            .into_iter()
            .filter_map(|sig_info| Signature::from_str(&sig_info.signature).ok())
            .collect())
    }
    
    async fn analyze_jupiter_activity(&self, pubkey: &Pubkey) -> Result<u32> {
        let jupiter_v4 = Pubkey::from_str(ProtocolAddresses::JUPITER_V4)
            .map_err(|_| DegenScoreError::ConfigError("Invalid Jupiter V4 address".to_string()))?;
        
        let jupiter_v6 = Pubkey::from_str(ProtocolAddresses::JUPITER_V6)
            .map_err(|_| DegenScoreError::ConfigError("Invalid Jupiter V6 address".to_string()))?;
        
        let signatures = self.get_recent_signatures(pubkey, 1000).await?;
        let mut jupiter_swaps = 0u32;
        
        // Check each transaction for Jupiter interactions
        for sig in signatures.iter().take(100) { // Limit to avoid rate limits
            if let Ok(transaction) = self.rpc_client.get_transaction(
                sig,
                UiTransactionEncoding::JsonParsed,
            ) {
                if let Some(tx) = transaction.transaction.transaction {
                    // Check if transaction includes Jupiter program
                    let has_jupiter = match &tx {
                        solana_transaction_status::EncodedTransaction::Json(ui_tx) => {
                            if let Some(msg) = &ui_tx.message {
                                msg.account_keys.iter().any(|key| {
                                    key.pubkey == jupiter_v4.to_string() || 
                                    key.pubkey == jupiter_v6.to_string()
                                })
                            } else {
                                false
                            }
                        }
                        _ => false,
                    };
                    
                    if has_jupiter {
                        jupiter_swaps += 1;
                    }
                }
            }
        }
        
        Ok(jupiter_swaps)
    }
    
    async fn get_spl_token_accounts(&self, pubkey: &Pubkey) -> Result<Vec<TokenBalance>> {
        let token_accounts = self.rpc_client
            .get_token_accounts_by_owner(
                pubkey,
                solana_client::rpc_request::TokenAccountsFilter::ProgramId(token_program_id()),
            )
            .map_err(|e| DegenScoreError::RpcError {
                chain: "solana".to_string(),
                message: format!("Failed to get token accounts: {}", e),
            })?;
        
        let mut balances = Vec::new();
        
        for (pubkey_str, account) in token_accounts {
            if let Ok(token_data) = spl_token::state::Account::unpack(&account.data) {
                let balance_u64 = token_data.amount;
                
                if balance_u64 > 0 {
                    // In production, we'd fetch token metadata
                    balances.push(TokenBalance {
                        token_address: token_data.mint.to_string(),
                        balance: ethers::types::U256::from(balance_u64),
                        decimals: 9, // Most SPL tokens use 9 decimals
                        symbol: "SPL".to_string(), // Would fetch actual symbol
                        name: "SPL Token".to_string(),
                    });
                }
            }
        }
        
        Ok(balances)
    }
    
    async fn check_nft_holdings(&self, pubkey: &Pubkey) -> Result<u32> {
        let token_accounts = self.get_spl_token_accounts(pubkey).await?;
        
        // NFTs on Solana typically have supply=1 and decimals=0
        let nft_count = token_accounts.iter()
            .filter(|token| {
                token.balance == ethers::types::U256::from(1) && 
                token.decimals == 0
            })
            .count() as u32;
        
        Ok(nft_count)
    }
}

#[async_trait]
impl ChainClient for SolanaClient {
    fn chain(&self) -> Chain {
        self.chain.clone()
    }
    
    async fn fetch_metrics(&self, address: &str) -> Result<ChainMetrics> {
        let pubkey = Pubkey::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        
        let mut metrics = DegenMetrics::default();
        
        // Get SOL balance
        let balance = self.rpc_client.get_balance(&pubkey)
            .map_err(|e| DegenScoreError::RpcError {
                chain: "solana".to_string(),
                message: format!("Failed to get balance: {}", e),
            })?;
        
        // Get transaction count (signatures)
        let signatures = self.get_recent_signatures(&pubkey, 10000).await?;
        metrics.total_tx_count = signatures.len() as u32;
        
        // Analyze Jupiter usage
        metrics.jupiter_swaps = self.analyze_jupiter_activity(&pubkey).await?;
        
        // Get SPL token holdings
        let token_accounts = self.get_spl_token_accounts(&pubkey).await?;
        metrics.distinct_tokens_traded = token_accounts.len() as u32;
        
        // Check NFT holdings
        metrics.nft_count = self.check_nft_holdings(&pubkey).await?;
        
        // Add Solana to active chains
        metrics.chains_active_on.push("solana".to_string());
        
        // Calculate wallet age from first transaction
        if let Some(oldest_sig) = signatures.last() {
            if let Ok(tx) = self.rpc_client.get_transaction(
                oldest_sig,
                UiTransactionEncoding::JsonParsed,
            ) {
                if let Some(block_time) = tx.block_time {
                    let first_tx_time = Utc.timestamp_opt(block_time, 0).single();
                    if let Some(first_tx) = first_tx_time {
                        metrics.first_transaction = Some(first_tx);
                        let age_days = (Utc::now() - first_tx).num_days() as u32;
                        metrics.wallet_age_days = age_days;
                    }
                }
            }
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
        let pubkey = Pubkey::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        
        let signatures = self.get_recent_signatures(&pubkey, 10000).await?;
        
        Ok(TransactionSummary {
            total_count: signatures.len() as u32,
            first_tx: None, // Would need to fetch block times
            last_tx: None,
            active_days: 0,
            average_tx_per_day: 0.0,
            gas_spent: Decimal::ZERO, // SOL uses lamports for fees
        })
    }
    
    async fn get_token_balances(&self, address: &str) -> Result<Vec<TokenBalance>> {
        let pubkey = Pubkey::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        
        let mut balances = self.get_spl_token_accounts(&pubkey).await?;
        
        // Add SOL balance
        let sol_balance = self.rpc_client.get_balance(&pubkey)
            .map_err(|e| DegenScoreError::RpcError {
                chain: "solana".to_string(),
                message: format!("Failed to get balance: {}", e),
            })?;
        
        if sol_balance > 0 {
            balances.push(TokenBalance {
                token_address: "11111111111111111111111111111111".to_string(),
                balance: ethers::types::U256::from(sol_balance),
                decimals: 9,
                symbol: "SOL".to_string(),
                name: "Solana".to_string(),
            });
        }
        
        Ok(balances)
    }
    
    async fn get_nft_balances(&self, address: &str) -> Result<Vec<NFTBalance>> {
        let pubkey = Pubkey::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        
        let token_accounts = self.get_spl_token_accounts(&pubkey).await?;
        
        let nfts: Vec<NFTBalance> = token_accounts
            .into_iter()
            .filter(|token| {
                token.balance == ethers::types::U256::from(1) && 
                token.decimals == 0
            })
            .map(|token| NFTBalance {
                contract_address: token.token_address.clone(),
                token_id: "1".to_string(), // Solana NFTs don't have token IDs like Ethereum
                token_uri: None,
                metadata: None,
            })
            .collect();
        
        Ok(nfts)
    }
    
    async fn has_used_protocol(&self, address: &str, protocol_address: &str) -> Result<bool> {
        let pubkey = Pubkey::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        
        let protocol_pubkey = Pubkey::from_str(protocol_address)
            .map_err(|_| DegenScoreError::InvalidAddress(protocol_address.to_string()))?;
        
        let signatures = self.get_recent_signatures(&pubkey, 1000).await?;
        
        // Check if any transaction includes the protocol
        for sig in signatures.iter().take(50) {
            if let Ok(transaction) = self.rpc_client.get_transaction(
                sig,
                UiTransactionEncoding::JsonParsed,
            ) {
                if let Some(tx) = transaction.transaction.transaction {
                    let has_protocol = match &tx {
                        solana_transaction_status::EncodedTransaction::Json(ui_tx) => {
                            if let Some(msg) = &ui_tx.message {
                                msg.account_keys.iter().any(|key| {
                                    key.pubkey == protocol_pubkey.to_string()
                                })
                            } else {
                                false
                            }
                        }
                        _ => false,
                    };
                    
                    if has_protocol {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    async fn get_protocol_metrics(
        &self,
        address: &str,
        protocol: &str
    ) -> Result<ProtocolMetrics> {
        let pubkey = Pubkey::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        
        match protocol.to_lowercase().as_str() {
            "jupiter" => {
                let swap_count = self.analyze_jupiter_activity(&pubkey).await?;
                Ok(ProtocolMetrics {
                    protocol_name: "Jupiter".to_string(),
                    interaction_count: swap_count,
                    volume_usd: Decimal::ZERO, // Would need to calculate from swap data
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
        Pubkey::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        Ok(())
    }
}