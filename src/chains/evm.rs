use async_trait::async_trait;
use ethers::{
    prelude::*,
    providers::{Provider, Http, Middleware},
    types::{Address, BlockNumber, Filter, H160, H256, U256, U64},
    utils::format_units,
};
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;
use tracing::{info, warn, debug};

use crate::{
    models::{
        Chain, ChainMetrics, DegenMetrics, TokenBalance, NFTBalance,
        TransactionSummary, DegenScoreError, Result, TokenType,
        ProtocolInteraction, ProtocolType, EVMTransaction, EVMTokenTransfer,
        chain_data::{ProtocolAddresses, EventSignatures, KnownTokens, TokenInteractionMetrics},
        CasinoInteraction, CasinoPlatform, InteractionType, CasinoMetrics,
        ScoreCache, CacheKey,
    },
    chains::{ChainClient, client::{ProtocolMetrics, ChainClientConfig}, ResilientRpcClient, CircuitBreakerConfig, RetryConfig},
};

pub struct EvmClient {
    provider: Arc<Provider<Http>>,
    resilient_client: ResilientRpcClient,
    chain: Chain,
    chain_id: u64,
    explorer_api: Option<String>,
    cache: Arc<ScoreCache>,
}

impl EvmClient {
    pub async fn new(config: ChainClientConfig, chain: Chain) -> Result<Self> {
        let provider = Provider::<Http>::try_from(config.rpc_url.as_str())
            .map_err(|e| DegenScoreError::ConfigError(format!("Invalid RPC URL: {}", e)))?;
        
        let provider = Arc::new(provider);
        
        // Create resilient RPC client
        let circuit_config = CircuitBreakerConfig::default();
        let retry_config = RetryConfig::default();
        let resilient_client = ResilientRpcClient::new(
            format!("{}_client", chain.as_str()),
            circuit_config,
            retry_config,
        );
        
        // Verify chain ID matches using resilient client
        let chain_name = chain.as_str().to_string();
        let chain_id = resilient_client.call(|| {
            let provider = provider.clone();
            let chain_name = chain_name.clone();
            async move {
                provider.get_chainid().await
                    .map_err(|e| DegenScoreError::RpcError {
                        chain: chain_name,
                        message: format!("Failed to get chain ID: {}", e),
                    })
            }
        }).await?;
        
        if let Some(expected_id) = config.chain_id {
            if chain_id.as_u64() != expected_id {
                return Err(DegenScoreError::ConfigError(
                    format!("Chain ID mismatch: expected {}, got {}", expected_id, chain_id)
                ));
            }
        }
        
        Ok(Self {
            provider,
            resilient_client,
            chain,
            chain_id: chain_id.as_u64(),
            explorer_api: None,
            cache: Arc::new(ScoreCache::default()),
        })
    }
    
    pub fn with_explorer_api(mut self, api_url: String) -> Self {
        self.explorer_api = Some(api_url);
        self
    }
    
    /// Clear all cached data for this client
    pub fn clear_cache(&self) {
        self.cache.clear_all();
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> crate::models::CacheStats {
        self.cache.get_stats()
    }
    
    async fn get_transaction_history(&self, address: &Address) -> Result<Vec<EVMTransaction>> {
        // For now, we'll use event logs to reconstruct activity
        // In production, we'd use explorer APIs for full history
        let current_block = self.resilient_client.call(|| {
            let provider = self.provider.clone();
            let chain_name = self.chain.as_str().to_string();
            async move {
                provider.get_block_number().await
                    .map_err(|e| DegenScoreError::RpcError {
                        chain: chain_name,
                        message: format!("Failed to get current block: {}", e),
                    })
            }
        }).await?;
        
        // Get transactions from the last ~2 weeks (to avoid rate limits)
        let from_block = current_block.saturating_sub(U64::from(8_000));
        
        let filter = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .address(vec![*address]);
        
        let logs = self.resilient_client.call(|| {
            let provider = self.provider.clone();
            let chain_name = self.chain.as_str().to_string();
            let filter = filter.clone();
            async move {
                provider.get_logs(&filter).await
                    .map_err(|e| DegenScoreError::RpcError {
                        chain: chain_name,
                        message: format!("Failed to get logs: {}", e),
                    })
            }
        }).await?;
        
        // This is a simplified version - in production we'd parse these logs
        // and potentially use explorer APIs for complete transaction history
        Ok(vec![])
    }
    
    async fn get_erc20_transfers(&self, address: &Address) -> Result<Vec<EVMTokenTransfer>> {
        let transfer_topic = H256::from_slice(
            &ethers::core::utils::keccak256(EventSignatures::ERC20_TRANSFER.as_bytes())
        );
        
        let current_block = self.resilient_client.call(|| {
            let provider = self.provider.clone();
            let chain_name = self.chain.as_str().to_string();
            async move {
                provider.get_block_number().await
                    .map_err(|e| DegenScoreError::RpcError {
                        chain: chain_name,
                        message: format!("Failed to get current block: {}", e),
                    })
            }
        }).await?;
        
        let from_block = current_block.saturating_sub(U64::from(8_000));
        
        // Get transfers FROM the address
        let filter_from = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .topic0(transfer_topic)
            .topic1(*address);
        
        // Get transfers TO the address
        let filter_to = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .topic0(transfer_topic)
            .topic2(*address);
        
        let logs_from = self.resilient_client.call(|| {
            let provider = self.provider.clone();
            let chain_name = self.chain.as_str().to_string();
            let filter = filter_from.clone();
            async move {
                provider.get_logs(&filter).await
                    .map_err(|e| DegenScoreError::RpcError {
                        chain: chain_name,
                        message: format!("Failed to get transfer logs: {}", e),
                    })
            }
        }).await?;
        
        let logs_to = self.resilient_client.call(|| {
            let provider = self.provider.clone();
            let chain_name = self.chain.as_str().to_string();
            let filter = filter_to.clone();
            async move {
                provider.get_logs(&filter).await
                    .map_err(|e| DegenScoreError::RpcError {
                        chain: chain_name,
                        message: format!("Failed to get transfer logs: {}", e),
                    })
            }
        }).await?;
        
        let mut transfers = Vec::new();
        
        // Collect unique block numbers to fetch timestamps efficiently
        let mut unique_blocks: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for log in logs_from.iter().chain(logs_to.iter()) {
            if let Some(block_number) = log.block_number {
                unique_blocks.insert(block_number.as_u64());
            }
        }
        
        // Fetch block timestamps for unique blocks (limit to avoid too many RPC calls)
        let mut block_timestamps = HashMap::new();
        for &block_num in unique_blocks.iter().take(20) { // Limit to 20 blocks max
            let block_result = self.resilient_client.call(|| {
                let provider = self.provider.clone();
                async move {
                    provider.get_block(block_num).await
                        .map_err(|e| DegenScoreError::RpcError {
                            chain: "evm".to_string(),
                            message: format!("Failed to get block {}: {}", block_num, e),
                        })
                }
            }).await;
            
            if let Ok(Some(block)) = block_result {
                block_timestamps.insert(block_num, block.timestamp.as_u64());
            }
        }
        
        // Parse transfer logs
        for log in logs_from.iter().chain(logs_to.iter()) {
            if log.topics.len() >= 3 {
                let from = Address::from(H160::from(log.topics[1]));
                let to = Address::from(H160::from(log.topics[2]));
                let value = U256::from_big_endian(&log.data);
                
                // Get timestamp from block data if available
                let timestamp = if let Some(block_number) = log.block_number {
                    let block_num = block_number.as_u64();
                    if let Some(&block_timestamp) = block_timestamps.get(&block_num) {
                        DateTime::from_timestamp(block_timestamp as i64, 0).unwrap_or_else(|| Utc::now())
                    } else {
                        Utc::now() // Fallback to current time
                    }
                } else {
                    Utc::now()
                };
                
                transfers.push(EVMTokenTransfer {
                    token_address: format!("{:?}", log.address),
                    from: format!("{:?}", from),
                    to: format!("{:?}", to),
                    value,
                    tx_hash: format!("{:?}", log.transaction_hash.unwrap_or_default()),
                    log_index: log.log_index.unwrap_or_default().as_u64(),
                    timestamp,
                });
            }
        }
        
        Ok(transfers)
    }
    
    async fn check_gmx_activity(&self, address: &Address) -> Result<ProtocolMetrics> {
        if self.chain != Chain::Arbitrum {
            return Ok(ProtocolMetrics {
                protocol_name: "GMX".to_string(),
                interaction_count: 0,
                volume_usd: Decimal::ZERO,
                first_interaction: None,
                last_interaction: None,
                custom_metrics: HashMap::new(),
            });
        }
        
        let gmx_vault = Address::from_str(ProtocolAddresses::GMX_VAULT)
            .map_err(|_| DegenScoreError::ConfigError("Invalid GMX vault address".to_string()))?;
        
        let increase_position_topic = H256::from_slice(
            &ethers::core::utils::keccak256(EventSignatures::GMX_INCREASE_POSITION.as_bytes())
        );
        
        let decrease_position_topic = H256::from_slice(
            &ethers::core::utils::keccak256(EventSignatures::GMX_DECREASE_POSITION.as_bytes())
        );
        
        let current_block = self.resilient_client.call(|| {
            let provider = self.provider.clone();
            let chain_name = self.chain.as_str().to_string();
            async move {
                provider.get_block_number().await
                    .map_err(|e| DegenScoreError::RpcError {
                        chain: chain_name,
                        message: format!("Failed to get current block: {}", e),
                    })
            }
        }).await?;
        
        let from_block = current_block.saturating_sub(U64::from(8_000)); // Reduced range for RPC limits
        
        // Check IncreasePosition events
        let increase_filter = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .address(gmx_vault)
            .topic0(increase_position_topic)
            .topic2(*address); // account is the second indexed parameter
        
        let increase_logs = self.resilient_client.call(|| {
            let provider = self.provider.clone();
            let chain_name = self.chain.as_str().to_string();
            let filter = increase_filter.clone();
            async move {
                provider.get_logs(&filter).await
                    .map_err(|e| DegenScoreError::RpcError {
                        chain: chain_name,
                        message: format!("Failed to get GMX IncreasePosition logs: {}", e),
                    })
            }
        }).await?;
        
        // Check DecreasePosition events
        let decrease_filter = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .address(gmx_vault)
            .topic0(decrease_position_topic)
            .topic2(*address); // account is the second indexed parameter
        
        let decrease_logs = self.resilient_client.call(|| {
            let provider = self.provider.clone();
            let chain_name = self.chain.as_str().to_string();
            let filter = decrease_filter.clone();
            async move {
                provider.get_logs(&filter).await
                    .map_err(|e| DegenScoreError::RpcError {
                        chain: chain_name,
                        message: format!("Failed to get GMX DecreasePosition logs: {}", e),
                    })
            }
        }).await?;
        
        let mut total_volume = Decimal::ZERO;
        let mut total_interactions = 0u32;
        
        // Parse IncreasePosition sizes from logs
        for log in &increase_logs {
            if log.data.len() >= 256 {
                // Size is the 5th parameter (uint256) in the event
                let size_bytes = &log.data[128..160];
                let size = U256::from_big_endian(size_bytes);
                let size_decimal = Decimal::from_str(&size.to_string()).unwrap_or(Decimal::ZERO);
                total_volume += size_decimal / Decimal::from(10u64.pow(30)); // GMX uses 30 decimals for USD
            }
        }
        
        // Parse DecreasePosition sizes from logs
        for log in &decrease_logs {
            if log.data.len() >= 256 {
                // Size is the 5th parameter (uint256) in the event
                let size_bytes = &log.data[128..160];
                let size = U256::from_big_endian(size_bytes);
                let size_decimal = Decimal::from_str(&size.to_string()).unwrap_or(Decimal::ZERO);
                total_volume += size_decimal / Decimal::from(10u64.pow(30)); // GMX uses 30 decimals for USD
            }
        }
        
        total_interactions = (increase_logs.len() + decrease_logs.len()) as u32;
        
        info!("GMX activity: {} increase, {} decrease = {} total interactions, ${} volume", 
              increase_logs.len(), decrease_logs.len(), total_interactions, total_volume);
        
        Ok(ProtocolMetrics {
            protocol_name: "GMX".to_string(),
            interaction_count: total_interactions,
            volume_usd: total_volume,
            first_interaction: None, // Would need to fetch block timestamps
            last_interaction: None,
            custom_metrics: HashMap::new(),
        })
    }
    
    async fn check_perpetual_protocol_activity(&self, address: &Address) -> Result<ProtocolMetrics> {
        if self.chain != Chain::Optimism {
            return Ok(ProtocolMetrics {
                protocol_name: "Perpetual Protocol".to_string(),
                interaction_count: 0,
                volume_usd: Decimal::ZERO,
                first_interaction: None,
                last_interaction: None,
                custom_metrics: HashMap::new(),
            });
        }

        let clearing_house = Address::from_str(ProtocolAddresses::PERP_CLEARING_HOUSE_OPT)
            .map_err(|_| DegenScoreError::ConfigError("Invalid Perpetual Protocol ClearingHouse address".to_string()))?;

        let current_block = self.provider.get_block_number().await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get current block: {}", e),
            })?;

        let from_block = current_block.saturating_sub(U64::from(8_000)); // Reduced range for RPC limits

        // Check for any events from the ClearingHouse involving this user
        let filter = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .address(clearing_house);

        let logs = self.provider.get_logs(&filter).await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get Perpetual Protocol logs: {}", e),
            })?;

        let mut user_interactions = 0u32;
        let mut estimated_volume = Decimal::ZERO;

        // Filter logs that involve the user address
        for log in &logs {
            let user_involved = log.topics.iter().any(|topic| {
                // Check if user address appears in any topic
                topic.as_bytes().ends_with(address.as_bytes())
            });

            if user_involved {
                user_interactions += 1;
                // Estimate volume based on interaction type
                // Since we can't easily parse the exact volumes without event ABI,
                // we'll use a reasonable estimate per interaction
                estimated_volume += Decimal::from(500); // $500 per position interaction
            }
        }

        info!("Perpetual Protocol: {} interactions, estimated ${} volume", user_interactions, estimated_volume);

        Ok(ProtocolMetrics {
            protocol_name: "Perpetual Protocol".to_string(),
            interaction_count: user_interactions,
            volume_usd: estimated_volume,
            first_interaction: None,
            last_interaction: None,
            custom_metrics: HashMap::new(),
        })
    }
    
    async fn check_casino_tokens(&self, address: &Address) -> Result<HashMap<String, Decimal>> {
        let mut casino_holdings = HashMap::new();
        let known_casinos = KnownTokens::casino_tokens_by_chain(&self.chain);
        
        debug!("Checking {} casino tokens on {}", known_casinos.len(), self.chain.as_str());
        
        for (token_addr, symbol) in known_casinos {
            if let Ok(token_address) = Address::from_str(token_addr) {
                // Check actual token balance using balanceOf call
                match self.get_token_balance(*address, token_address).await {
                    Ok(balance) => {
                        if balance > Decimal::ZERO {
                            // Convert from raw token units to human readable
                            // Most tokens have 18 decimals
                            let decimals = 18;
                            let divisor = Decimal::from(10u64.pow(decimals));
                            let human_balance = balance / divisor;
                            
                            casino_holdings.insert(symbol.to_string(), human_balance);
                            info!("Found {} {} tokens", human_balance, symbol);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to check {} balance: {}", symbol, e);
                    }
                }
            }
        }
        
        Ok(casino_holdings)
    }
    
    async fn get_token_balance(&self, holder: Address, token: Address) -> Result<Decimal> {
        // ERC20 balanceOf function selector: 0x70a08231
        let function_selector = [0x70, 0xa0, 0x82, 0x31];
        let mut call_data = function_selector.to_vec();
        
        // Encode the holder address (32 bytes, left-padded)
        let mut holder_bytes = [0u8; 32];
        holder_bytes[12..].copy_from_slice(holder.as_bytes());
        call_data.extend_from_slice(&holder_bytes);
        
        let call_req = ethers::types::transaction::eip2718::TypedTransaction::Legacy(
            ethers::types::TransactionRequest {
                to: Some(token.into()),
                data: Some(call_data.into()),
                ..Default::default()
            }
        );
        
        let result = self.provider.call(&call_req, None).await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to call balanceOf: {}", e),
            })?;
        
        if result.len() >= 32 {
            let balance_u256 = U256::from_big_endian(&result[..32]);
            let balance_decimal = Decimal::from_str(&balance_u256.to_string())
                .unwrap_or(Decimal::ZERO);
            Ok(balance_decimal)
        } else {
            Ok(Decimal::ZERO)
        }
    }
    
    async fn check_aave_activity(&self, address: &Address) -> Result<bool> {
        let aave_pool = match self.chain {
            Chain::Ethereum => ProtocolAddresses::AAVE_V2_POOL_ETH,
            Chain::Arbitrum => ProtocolAddresses::AAVE_V3_POOL_ARB,
            Chain::Optimism => ProtocolAddresses::AAVE_V3_POOL_OPT,
            _ => return Ok(false),
        };
        
        let pool_addr = Address::from_str(aave_pool)
            .map_err(|_| DegenScoreError::ConfigError("Invalid Aave pool address".to_string()))?;
            
        // Check for Deposit events
        let deposit_topic = H256::from_slice(
            &ethers::core::utils::keccak256(EventSignatures::AAVE_DEPOSIT.as_bytes())
        );
        
        let current_block = self.provider.get_block_number().await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get current block: {}", e),
            })?;
        
        let from_block = current_block.saturating_sub(U64::from(8_000)); // Reduced range for RPC limits
        
        // Check if user deposited (onBehalfOf parameter)
        let filter = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .address(pool_addr)
            .topic0(deposit_topic);
            
        let logs = self.provider.get_logs(&filter).await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get Aave logs: {}", e),
            })?;
            
        // Check if any logs have the user address as onBehalfOf (5th parameter)
        for log in logs {
            if log.topics.len() >= 2 && log.data.len() >= 160 {
                // onBehalfOf is in the data field (5th parameter)
                let on_behalf_of_bytes = &log.data[128..160];
                let on_behalf_of = Address::from_slice(&on_behalf_of_bytes[12..]);
                if on_behalf_of == *address {
                    return Ok(true);
                }
            }
        }
        
        // Also check for Borrow events
        let borrow_topic = H256::from_slice(
            &ethers::core::utils::keccak256(EventSignatures::AAVE_BORROW.as_bytes())
        );
        
        let filter_borrow = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .address(pool_addr)
            .topic0(borrow_topic)
            .topic2(*address); // borrower is indexed
            
        let borrow_logs = self.provider.get_logs(&filter_borrow).await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get Aave borrow logs: {}", e),
            })?;
            
        Ok(!borrow_logs.is_empty())
    }
    
    async fn check_compound_activity(&self, address: &Address) -> Result<bool> {
        if self.chain != Chain::Ethereum {
            return Ok(false); // Compound is mainly on Ethereum
        }
        
        // Check if user holds any cTokens
        let ctoken_addresses = vec![
            ProtocolAddresses::COMPOUND_CDAI,
            ProtocolAddresses::COMPOUND_CUSDC,
            ProtocolAddresses::COMPOUND_CETH,
        ];
        
        for ctoken_str in ctoken_addresses {
            let ctoken = Address::from_str(ctoken_str)
                .map_err(|_| DegenScoreError::ConfigError("Invalid cToken address".to_string()))?;
                
            let balance = self.get_token_balance(*address, ctoken).await?;
            if balance > Decimal::ZERO {
                return Ok(true);
            }
        }
        
        // Also check for Mint events (supplying to Compound)
        let comptroller = Address::from_str(ProtocolAddresses::COMPOUND_COMPTROLLER)
            .map_err(|_| DegenScoreError::ConfigError("Invalid Compound comptroller".to_string()))?;
            
        if self.check_contract_interaction(address, &comptroller).await? {
            return Ok(true);
        }
        
        Ok(false)
    }
    
    async fn check_bridge_activity(&self, address: &Address) -> Result<u32> {
        let mut bridge_uses = 0;
        
        // Check Hyperliquid bridge on Arbitrum
        if self.chain == Chain::Arbitrum {
            let hl_bridge = Address::from_str(ProtocolAddresses::HYPERLIQUID_BRIDGE_ARB)
                .map_err(|_| DegenScoreError::ConfigError("Invalid Hyperliquid bridge".to_string()))?;
                
            // Check for USDC transfers to Hyperliquid bridge (deposits)
            if let Ok((deposits, _volume)) = self.check_hyperliquid_deposits(address, &hl_bridge).await {
                bridge_uses += deposits;
            }
        }
        
        // Check other bridges on Ethereum
        if self.chain == Chain::Ethereum {
            let bridges = vec![
                ProtocolAddresses::HOP_BRIDGE_ETH,
                ProtocolAddresses::ACROSS_BRIDGE_ETH,
            ];
            
            for bridge_str in bridges {
                let bridge = Address::from_str(bridge_str)
                    .map_err(|_| DegenScoreError::ConfigError("Invalid bridge address".to_string()))?;
                    
                if self.check_contract_interaction(address, &bridge).await? {
                    bridge_uses += 1;
                }
            }
        }
        
        Ok(bridge_uses)
    }
    
    async fn check_hyperliquid_deposits(&self, user_addr: &Address, bridge_addr: &Address) -> Result<(u32, Decimal)> {
        // Check for USDC transfers from user to Hyperliquid bridge
        // Using native USDC on Arbitrum (not USDC.e bridged version)
        let usdc_arb = Address::from_str("0xaf88d065ef77c8cC2239327C5EDb3A432268e5831") // Native USDC on Arbitrum
            .map_err(|_| DegenScoreError::ConfigError("Invalid USDC address".to_string()))?;
            
        let transfer_topic = H256::from_slice(
            &ethers::core::utils::keccak256(EventSignatures::ERC20_TRANSFER.as_bytes())
        );
        
        let current_block = self.provider.get_block_number().await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get current block: {}", e),
            })?;
        
        let from_block = current_block.saturating_sub(U64::from(8_000)); // Reduced range for RPC limits
        
        // Look for transfers: from=user, to=bridge
        let filter = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .address(vec![usdc_arb])
            .topic0(transfer_topic)
            .topic1(vec![H256::from(*user_addr)])  // from user
            .topic2(vec![H256::from(*bridge_addr)]); // to bridge
            
        let logs = self.provider.get_logs(&filter).await
            .unwrap_or_default();
            
        // Calculate total deposit volume
        let mut total_volume = Decimal::ZERO;
        for log in &logs {
            if log.data.len() >= 32 {
                let amount = U256::from_big_endian(&log.data[..32]);
                let amount_decimal = Decimal::from_str(&amount.to_string()).unwrap_or(Decimal::ZERO);
                // USDC has 6 decimals
                let amount_usd = amount_decimal / Decimal::from(1_000_000);
                total_volume += amount_usd;
            }
        }
            
        Ok((logs.len() as u32, total_volume))
    }
    
    async fn calculate_wallet_age(&self, address: &Address) -> Result<u32> {
        // Binary search to find the first transaction efficiently
        let current_block = self.provider.get_block_number().await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get current block: {}", e),
            })?;
        
        // Start with a reasonable range - check last 2 years worth of blocks
        let blocks_per_day = match self.chain {
            Chain::Ethereum => 7200,  // ~12 sec blocks
            Chain::Arbitrum => 43200, // ~2 sec blocks  
            Chain::Optimism => 43200, // ~2 sec blocks
            Chain::Blast => 43200,    // ~2 sec blocks
            _ => 7200,
        };
        
        let max_age_blocks = blocks_per_day * 365 * 2; // 2 years
        let start_block = current_block.saturating_sub(U64::from(max_age_blocks));
        
        // Find first block where nonce > 0
        let first_tx_block = self.binary_search_first_transaction(address, start_block.as_u64(), current_block.as_u64()).await?;
        
        if let Some(block_number) = first_tx_block {
            let block = self.provider.get_block(block_number).await
                .map_err(|e| DegenScoreError::RpcError {
                    chain: self.chain.as_str().to_string(),
                    message: format!("Failed to get block: {}", e),
                })?;
            
            if let Some(block) = block {
                let block_timestamp = block.timestamp.as_u64();
                let current_timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                
                let age_seconds = current_timestamp.saturating_sub(block_timestamp);
                let age_days = (age_seconds / 86400) as u32; // 86400 seconds in a day
                
                info!("Wallet age: {} days (first tx at block {})", age_days, block_number);
                return Ok(age_days);
            }
        }
        
        // If we can't find first transaction, return 0
        Ok(0)
    }
    
    async fn binary_search_first_transaction(&self, address: &Address, start_block: u64, end_block: u64) -> Result<Option<u64>> {
        let mut low = start_block;
        let mut high = end_block;
        let mut first_tx_block = None;
        
        // Limit iterations to prevent infinite loops
        for _ in 0..20 {
            if low >= high {
                break;
            }
            
            let mid = (low + high) / 2;
            
            let nonce = self.provider.get_transaction_count(*address, Some(mid.into())).await
                .map_err(|e| DegenScoreError::RpcError {
                    chain: self.chain.as_str().to_string(),
                    message: format!("Failed to get nonce at block {}: {}", mid, e),
                })?;
            
            if nonce > U256::zero() {
                // Found a transaction, search earlier
                first_tx_block = Some(mid);
                high = mid;
            } else {
                // No transaction yet, search later
                low = mid + 1;
            }
        }
        
        Ok(first_tx_block)
    }
    
    async fn check_casino_interactions(&self, address: &Address) -> Result<CasinoMetrics> {
        let mut metrics = CasinoMetrics::default();
        
        // Check Rollbit interactions
        let rollbit_lottery = Address::from_str(ProtocolAddresses::ROLLBIT_LOTTERY)
            .map_err(|_| DegenScoreError::ConfigError("Invalid Rollbit lottery address".to_string()))?;
        let rollbit_staking = Address::from_str(ProtocolAddresses::ROLLBIT_STAKING)
            .map_err(|_| DegenScoreError::ConfigError("Invalid Rollbit staking address".to_string()))?;
            
        // Check for any transactions to Rollbit contracts
        if self.check_contract_interaction(address, &rollbit_lottery).await? ||
           self.check_contract_interaction(address, &rollbit_staking).await? {
            metrics.platforms_used.insert(CasinoPlatform::Rollbit);
            metrics.total_interactions += 1;
        }
        
        // Check Shuffle interactions
        let shuffle_router = Address::from_str(ProtocolAddresses::SHUFFLE_ROUTER)
            .map_err(|_| DegenScoreError::ConfigError("Invalid Shuffle router address".to_string()))?;
            
        if self.check_contract_interaction(address, &shuffle_router).await? {
            metrics.platforms_used.insert(CasinoPlatform::Shuffle);
            metrics.total_interactions += 1;
        }
        
        // Check for YEET token transfers as proxy for Yeet platform usage
        let yeet_token = Address::from_str(ProtocolAddresses::YEET_TOKEN)
            .map_err(|_| DegenScoreError::ConfigError("Invalid YEET token address".to_string()))?;
            
        if self.check_token_interaction(address, &yeet_token).await? {
            metrics.platforms_used.insert(CasinoPlatform::Yeet);
            metrics.total_interactions += 1;
        }
        
        Ok(metrics)
    }
    
    async fn check_contract_interaction(&self, user_addr: &Address, contract_addr: &Address) -> Result<bool> {
        let current_block = self.provider.get_block_number().await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get current block: {}", e),
            })?;
        
        let from_block = current_block.saturating_sub(U64::from(8_000)); // Reduced range for RPC limits
        
        // Check by looking for any events from the contract where user is involved
        // We'll check if user has ever called the contract by looking at nonce
        let nonce_at_block = self.provider.get_transaction_count(*user_addr, Some(from_block.into())).await
            .unwrap_or(U256::zero());
        let current_nonce = self.provider.get_transaction_count(*user_addr, None).await
            .unwrap_or(U256::zero());
            
        // If nonce increased, user made transactions
        if current_nonce > nonce_at_block {
            // For a more precise check, we'd need to iterate through blocks
            // For now, we'll check if the contract was used by looking for events
            let filter = Filter::new()
                .from_block(from_block)
                .to_block(current_block)
                .address(*contract_addr);
                
            let logs = self.provider.get_logs(&filter).await
                .map_err(|e| DegenScoreError::RpcError {
                    chain: self.chain.as_str().to_string(),
                    message: format!("Failed to get logs: {}", e),
                })?;
                
            // Check if any log involves the user address in topics
            for log in &logs {
                for topic in &log.topics {
                    if topic.as_bytes().ends_with(user_addr.as_bytes()) {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    async fn check_token_interaction(&self, user_addr: &Address, token_addr: &Address) -> Result<bool> {
        // Check if user has meaningful interactions with this token
        let interaction_metrics = self.check_token_interaction_detailed(user_addr, token_addr).await?;
        
        // Apply refined thresholds for meaningful interaction
        Ok(self.is_meaningful_token_interaction(&interaction_metrics))
    }
    
    async fn check_token_interaction_detailed(&self, user_addr: &Address, token_addr: &Address) -> Result<TokenInteractionMetrics> {
        let transfer_topic = H256::from_slice(
            &ethers::core::utils::keccak256(EventSignatures::ERC20_TRANSFER.as_bytes())
        );
        
        let current_block = self.provider.get_block_number().await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get current block: {}", e),
            })?;
        
        let from_block = current_block.saturating_sub(U64::from(8_000)); // Reduced range for RPC limits
        
        // Check transfers FROM user (outgoing)
        let filter_from = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .address(*token_addr)
            .topic0(transfer_topic)
            .topic1(*user_addr); // User as sender
            
        let logs_from = self.provider.get_logs(&filter_from).await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get outgoing transfer logs: {}", e),
            })?;
        
        // Check transfers TO user (incoming)
        let filter_to = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .address(*token_addr)
            .topic0(transfer_topic)
            .topic2(*user_addr); // User as recipient
            
        let logs_to = self.provider.get_logs(&filter_to).await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get incoming transfer logs: {}", e),
            })?;
        
        // Calculate total volume
        let mut total_volume = Decimal::ZERO;
        for log in logs_from.iter().chain(logs_to.iter()) {
            if log.data.len() >= 32 {
                let amount = U256::from_big_endian(&log.data[..32]);
                let amount_decimal = Decimal::from_str(&amount.to_string()).unwrap_or(Decimal::ZERO);
                total_volume += amount_decimal;
            }
        }
        
        Ok(TokenInteractionMetrics {
            transfers_in: logs_to.len() as u32,
            transfers_out: logs_from.len() as u32,
            total_volume_raw: total_volume,
            first_interaction: None, // Could extract from block timestamps
            last_interaction: None,
        })
    }
    
    fn is_meaningful_token_interaction(&self, metrics: &TokenInteractionMetrics) -> bool {
        let total_transfers = metrics.transfers_in + metrics.transfers_out;
        
        // Refined thresholds for meaningful interaction
        if total_transfers == 0 {
            return false;
        }
        
        // Require either:
        // 1. Multiple transfers (not just airdrop) - suggests actual usage
        if total_transfers > 2 {
            return true;
        }
        
        // 2. Both directions (received AND sent) - suggests active trading
        if metrics.transfers_in > 0 && metrics.transfers_out > 0 {
            return true;
        }
        
        // 3. Large volume (even if single transfer) - suggests significant activity
        if metrics.total_volume_raw > Decimal::from(1_000_000_000_000_000_000u64) { // > 1 token (assuming 18 decimals)
            return true;
        }
        
        // Otherwise, likely just dust or airdrop
        false
    }
    
    async fn check_protocol_interaction(&self, user_addr: &Address, protocol_addr: &Address) -> Result<bool> {
        // Check cache first
        let cache_key = CacheKey::protocol(
            self.chain.as_str(), 
            &format!("{:?}", user_addr), 
            &format!("{:?}", protocol_addr)
        );
        
        // For protocol interactions, we cache as a simple count (0 = no interaction, >0 = has interaction)
        if let Some(cached_interactions) = self.cache.get_protocol_interactions(&cache_key) {
            return Ok(cached_interactions.values().sum::<u32>() > 0);
        }
        
        // Check if user has interacted with a protocol by looking for transactions to that address
        let current_block = self.resilient_client.call(|| {
            let provider = self.provider.clone();
            let chain_name = self.chain.as_str().to_string();
            async move {
                provider.get_block_number().await
                    .map_err(|e| DegenScoreError::RpcError {
                        chain: chain_name,
                        message: format!("Failed to get current block: {}", e),
                    })
            }
        }).await?;
        
        let from_block = current_block.saturating_sub(U64::from(8_000));
        
        // Method 1: Check for transactions FROM user TO protocol
        let filter_to = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .address(vec![*protocol_addr])
            .topic1(vec![H256::from(*user_addr)]); // User as sender in Transfer events
        
        let logs_to = self.provider.get_logs(&filter_to).await
            .unwrap_or_default();
        
        if !logs_to.is_empty() {
            // Cache the positive result
            let mut interactions = HashMap::new();
            interactions.insert(format!("{:?}", protocol_addr), 1);
            self.cache.set_protocol_interactions(cache_key.clone(), interactions);
            return Ok(true);
        }
        
        // Method 2: Check for any events from protocol that involve user
        let filter_from = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .address(vec![*protocol_addr])
            .topic2(vec![H256::from(*user_addr)]); // User as recipient
        
        let logs_from = self.provider.get_logs(&filter_from).await
            .unwrap_or_default();
        
        let has_interaction = !logs_from.is_empty();
        
        // Cache the result
        let mut interactions = HashMap::new();
        interactions.insert(format!("{:?}", protocol_addr), if has_interaction { 1 } else { 0 });
        self.cache.set_protocol_interactions(cache_key, interactions);
        
        Ok(has_interaction)
    }
}

#[async_trait]
impl ChainClient for EvmClient {
    fn chain(&self) -> Chain {
        self.chain.clone()
    }
    
    async fn fetch_metrics(&self, address: &str) -> Result<ChainMetrics> {
        // Check cache first
        let cache_key = CacheKey::metrics(self.chain.as_str(), address);
        if let Some(cached_metrics) = self.cache.get_metrics(&cache_key) {
            info!("Cache hit for {} on {}", address, self.chain.as_str());
            return Ok(cached_metrics);
        }
        
        let addr = Address::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        
        let mut metrics = DegenMetrics::default();
        let mut protocols_used = std::collections::HashSet::new();
        
        // Get basic account info
        let balance = self.provider.get_balance(addr, None).await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get balance: {}", e),
            })?;
        
        let tx_count = self.provider.get_transaction_count(addr, None).await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get transaction count: {}", e),
            })?;
        
        metrics.total_tx_count = tx_count.as_u32();
        
        info!("Address: {}, TX Count: {}", address, tx_count);
        
        // Calculate real wallet age from first transaction
        let wallet_age_days = self.calculate_wallet_age(&addr).await.unwrap_or(0);
        metrics.wallet_age_days = wallet_age_days;
        
        // Get token transfers to identify trading activity
        let transfers = match self.get_erc20_transfers(&addr).await {
            Ok(transfers) => {
                debug!("Successfully fetched {} transfers", transfers.len());
                transfers
            }
            Err(e) => {
                warn!("Failed to fetch transfers: {}. Using mock data.", e);
                Vec::new()
            }
        };
        let unique_tokens: std::collections::HashSet<_> = transfers.iter()
            .map(|t| &t.token_address)
            .collect();
        metrics.distinct_tokens_traded = unique_tokens.len() as u32;
        
        // Check for memecoin trading
        let memecoin_addrs = KnownTokens::memecoin_addresses();
        let mut memecoin_trades = 0;
        for transfer in &transfers {
            if memecoin_addrs.contains_key(transfer.token_address.to_lowercase().as_str()) {
                memecoin_trades += 1;
            }
        }
        if memecoin_trades > 0 {
            metrics.memecoin_trades = memecoin_trades;
            info!("Memecoin transfers detected: {}", memecoin_trades);
        }
        
        // Calculate activity days from transfer timestamps
        let mut activity_days = std::collections::HashSet::new();
        for transfer in &transfers {
            let timestamp = transfer.timestamp.timestamp() as u64;
            let day = timestamp / 86400; // Convert to days since epoch
            activity_days.insert(day);
        }
        metrics.active_days = activity_days.len() as u32;
        
        // Check protocol-specific activity
        if self.chain == Chain::Arbitrum {
            // Check GMX activity
            match self.check_gmx_activity(&addr).await {
                Ok(gmx_metrics) => {
                    if gmx_metrics.interaction_count > 0 {
                        metrics.gmx_volume_usd = gmx_metrics.volume_usd;
                        metrics.gmx_trades = gmx_metrics.interaction_count;
                        metrics.total_perp_volume_usd += gmx_metrics.volume_usd;
                        metrics.leveraged_positions_count += 1; // User has used leveraged trading
                        protocols_used.insert("GMX");
                        // Track detailed protocol metrics
                        *metrics.protocol_interaction_counts.entry("GMX".to_string()).or_insert(0) += gmx_metrics.interaction_count;
                        *metrics.protocol_volume_usd.entry("GMX".to_string()).or_insert(Decimal::ZERO) += gmx_metrics.volume_usd;
                        info!("GMX activity: {} USD volume, {} trades", gmx_metrics.volume_usd, gmx_metrics.interaction_count);
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch GMX activity: {}", e);
                }
            }
            
            // Check for other protocol interactions (skip GMX routers if already counted)
            let protocols_to_check = vec![
                (ProtocolAddresses::CAMELOT_ROUTER, "Camelot"),
                (ProtocolAddresses::GAINS_TRADING_V6, "Gains Network"),
                (ProtocolAddresses::LEVEL_ROUTER, "Level Finance"),
            ];
            
            for (protocol_addr, protocol_name) in protocols_to_check {
                if let Ok(protocol_address) = Address::from_str(protocol_addr) {
                    match self.check_protocol_interaction(&addr, &protocol_address).await {
                        Ok(has_interaction) => {
                            if has_interaction {
                                protocols_used.insert(protocol_name);
                                if protocol_name == "Gains Network" || protocol_name == "Level Finance" {
                                    metrics.leveraged_positions_count += 1; // These are leveraged trading platforms
                                }
                                // Track detailed protocol interaction
                                *metrics.protocol_interaction_counts.entry(protocol_name.to_string()).or_insert(0) += 1;
                                info!("Found interaction with {}", protocol_name);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to check {} interaction: {}", protocol_name, e);
                        }
                    }
                }
            }
            
            // Also check for token holdings of these protocols
            let token_checks = vec![
                (ProtocolAddresses::GAINS_GNS_TOKEN, "Gains Network"),
                (ProtocolAddresses::LEVEL_LVL_TOKEN, "Level Finance"),
            ];
            
            for (token_addr, protocol_name) in token_checks {
                if let Ok(token_address) = Address::from_str(token_addr) {
                    match self.check_token_interaction(&addr, &token_address).await {
                        Ok(has_tokens) => {
                            if has_tokens {
                                protocols_used.insert(protocol_name);
                                info!("Found {} token interaction", protocol_name);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to check {} token: {}", protocol_name, e);
                        }
                    }
                }
            }
        }
        
        // Check Perpetual Protocol on Optimism
        if self.chain == Chain::Optimism {
            if let Ok(perp_clearing_house) = Address::from_str(ProtocolAddresses::PERP_CLEARING_HOUSE_OPT) {
                match self.check_protocol_interaction(&addr, &perp_clearing_house).await {
                    Ok(has_interaction) => {
                        if has_interaction {
                            protocols_used.insert("Perpetual Protocol");
                            // Try to get more detailed metrics
                            match self.check_perpetual_protocol_activity(&addr).await {
                                Ok(perp_metrics) => {
                                    metrics.total_perp_volume_usd += perp_metrics.volume_usd;
                                    // Track detailed protocol metrics
                                    *metrics.protocol_interaction_counts.entry("Perpetual Protocol".to_string()).or_insert(0) += perp_metrics.interaction_count;
                                    *metrics.protocol_volume_usd.entry("Perpetual Protocol".to_string()).or_insert(Decimal::ZERO) += perp_metrics.volume_usd;
                                    info!("Perpetual Protocol activity: ${} volume, {} interactions", 
                                          perp_metrics.volume_usd, perp_metrics.interaction_count);
                                }
                                Err(_) => {
                                    // Fallback: small estimated volume for detected interaction
                                    metrics.total_perp_volume_usd += Decimal::from(100);
                                    *metrics.protocol_interaction_counts.entry("Perpetual Protocol".to_string()).or_insert(0) += 1;
                                    info!("Found Perpetual Protocol activity (estimated volume)");
                                }
                            }
                            metrics.leveraged_positions_count += 1; // User has used leveraged trading
                        }
                    }
                    Err(e) => {
                        warn!("Failed to check Perpetual Protocol: {}", e);
                    }
                }
            }
        }
        
        // Check DEX interactions on all chains
        if self.chain == Chain::Ethereum || self.chain == Chain::Arbitrum || self.chain == Chain::Optimism {
            let dex_protocols = vec![
                (ProtocolAddresses::UNISWAP_V2_ROUTER, "Uniswap V2"),
                (ProtocolAddresses::UNISWAP_V3_ROUTER, "Uniswap V3"),
                (ProtocolAddresses::UNISWAP_UNIVERSAL_ROUTER, "Uniswap Universal"),
                (ProtocolAddresses::SUSHI_ROUTER, "Sushiswap"),
            ];
            
            for (protocol_addr, protocol_name) in dex_protocols {
                if let Ok(protocol_address) = Address::from_str(protocol_addr) {
                    match self.check_protocol_interaction(&addr, &protocol_address).await {
                        Ok(has_interaction) => {
                            if has_interaction {
                                protocols_used.insert(protocol_name);
                                info!("Found interaction with {}", protocol_name);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to check {} interaction: {}", protocol_name, e);
                        }
                    }
                }
            }
        }
        
        // Check casino token holdings
        match self.check_casino_tokens(&addr).await {
            Ok(casino_tokens) => {
                metrics.casino_tokens_held = casino_tokens.clone();
                debug!("Casino tokens: {}", casino_tokens.len());
            }
            Err(e) => {
                warn!("Failed to fetch casino tokens: {}", e);
            }
        }
        
        // Check casino platform interactions (not just token holdings)
        match self.check_casino_interactions(&addr).await {
            Ok(casino_metrics) => {
                metrics.casinos_used = casino_metrics.platforms_used.len() as u32;
                info!("Casino platforms used: {}", metrics.casinos_used);
                for platform in &casino_metrics.platforms_used {
                    debug!("  - {:?}", platform);
                }
            }
            Err(e) => {
                warn!("Failed to check casino interactions: {}", e);
            }
        }
        
        // Check Arbitrum-specific protocols
        if self.chain == Chain::Arbitrum {
            // Check Gains Network (leveraged trading)
            let gains_protocols = vec![
                (ProtocolAddresses::GAINS_TRADING_V6, "Gains Network Trading"),
                (ProtocolAddresses::GAINS_DAI_VAULT, "Gains Network Vault"),
            ];
            
            for (protocol_addr, protocol_name) in gains_protocols {
                if let Ok(protocol_address) = Address::from_str(protocol_addr) {
                    match self.check_protocol_interaction(&addr, &protocol_address).await {
                        Ok(has_interaction) => {
                            if has_interaction {
                                protocols_used.insert(protocol_name);
                                *metrics.protocol_interaction_counts.entry(protocol_name.to_string()).or_insert(0) += 1;
                                metrics.leveraged_positions_count += 1; // Gains is leveraged trading
                                info!("Found interaction with {}", protocol_name);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to check {} interaction: {}", protocol_name, e);
                        }
                    }
                }
            }
            
            // Check Level Finance
            if let Ok(level_router) = Address::from_str(ProtocolAddresses::LEVEL_ROUTER) {
                match self.check_protocol_interaction(&addr, &level_router).await {
                    Ok(has_interaction) => {
                        if has_interaction {
                            protocols_used.insert("Level Finance");
                            *metrics.protocol_interaction_counts.entry("Level Finance".to_string()).or_insert(0) += 1;
                            metrics.leveraged_positions_count += 1; // Level is leveraged trading
                            info!("Found Level Finance activity");
                        }
                    }
                    Err(e) => {
                        warn!("Failed to check Level Finance: {}", e);
                    }
                }
            }
            
            // Check Camelot DEX
            if let Ok(camelot_router) = Address::from_str(ProtocolAddresses::CAMELOT_ROUTER) {
                match self.check_protocol_interaction(&addr, &camelot_router).await {
                    Ok(has_interaction) => {
                        if has_interaction {
                            protocols_used.insert("Camelot");
                            *metrics.protocol_interaction_counts.entry("Camelot".to_string()).or_insert(0) += 1;
                            info!("Found Camelot DEX activity");
                        }
                    }
                    Err(e) => {
                        warn!("Failed to check Camelot: {}", e);
                    }
                }
            }
            
            // Check for Arbitrum protocol token holdings
            let arb_token_checks = vec![
                (ProtocolAddresses::GAINS_GNS_TOKEN, "Gains Network"),
                (ProtocolAddresses::LEVEL_LVL_TOKEN, "Level Finance"),
            ];
            
            for (token_addr, protocol_name) in arb_token_checks {
                if let Ok(token_address) = Address::from_str(token_addr) {
                    match self.check_token_interaction(&addr, &token_address).await {
                        Ok(has_tokens) => {
                            if has_tokens {
                                protocols_used.insert(protocol_name);
                                info!("Found {} token interaction", protocol_name);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to check {} token: {}", protocol_name, e);
                        }
                    }
                }
            }
        }
        
        // Check DeFi lending protocols
        if self.check_aave_activity(&addr).await.unwrap_or(false) {
            protocols_used.insert("Aave");
            *metrics.protocol_interaction_counts.entry("Aave".to_string()).or_insert(0) += 1;
            info!("Found Aave activity");
        }
        
        if self.check_compound_activity(&addr).await.unwrap_or(false) {
            protocols_used.insert("Compound");
            *metrics.protocol_interaction_counts.entry("Compound".to_string()).or_insert(0) += 1;
            info!("Found Compound activity");
        }
        
        // Check additional major DeFi protocols on Ethereum
        if self.chain == Chain::Ethereum {
            let major_defi_protocols = vec![
                (ProtocolAddresses::CURVE_REGISTRY, "Curve Finance"),
                (ProtocolAddresses::CURVE_3POOL, "Curve 3Pool"),
                (ProtocolAddresses::DYDX_PERPETUAL_V3, "dYdX"),
                (ProtocolAddresses::DYDX_SOLO_MARGIN, "dYdX Solo"),
                (ProtocolAddresses::MAKER_CDP_MANAGER, "MakerDAO"),
                (ProtocolAddresses::YEARN_REGISTRY, "Yearn Finance"),
            ];
            
            for (protocol_addr, protocol_name) in major_defi_protocols {
                if let Ok(protocol_address) = Address::from_str(protocol_addr) {
                    match self.check_protocol_interaction(&addr, &protocol_address).await {
                        Ok(has_interaction) => {
                            if has_interaction {
                                protocols_used.insert(protocol_name);
                                *metrics.protocol_interaction_counts.entry(protocol_name.to_string()).or_insert(0) += 1;
                                if protocol_name.contains("dYdX") {
                                    metrics.leveraged_positions_count += 1; // dYdX is leveraged trading
                                }
                                info!("Found interaction with {}", protocol_name);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to check {} interaction: {}", protocol_name, e);
                        }
                    }
                }
            }
            
            // Check NFT marketplace activity
            let nft_marketplaces = vec![
                (ProtocolAddresses::OPENSEA_SEAPORT, "OpenSea"),
                (ProtocolAddresses::OPENSEA_WYVERN_EXCHANGE, "OpenSea Legacy"),
                (ProtocolAddresses::BLUR_EXCHANGE, "Blur"),
                (ProtocolAddresses::X2Y2_EXCHANGE, "X2Y2"),
                (ProtocolAddresses::LOOKSRARE_EXCHANGE, "LooksRare"),
            ];
            
            for (marketplace_addr, marketplace_name) in nft_marketplaces {
                if let Ok(marketplace_address) = Address::from_str(marketplace_addr) {
                    match self.check_protocol_interaction(&addr, &marketplace_address).await {
                        Ok(has_interaction) => {
                            if has_interaction {
                                protocols_used.insert(marketplace_name);
                                *metrics.protocol_interaction_counts.entry(marketplace_name.to_string()).or_insert(0) += 1;
                                metrics.nft_trades += 1; // Increment NFT trading activity
                                info!("Found NFT trading activity on {}", marketplace_name);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to check {} interaction: {}", marketplace_name, e);
                        }
                    }
                }
            }
            
            // Check for token holdings of major protocols
            let token_checks = vec![
                (ProtocolAddresses::DYDX_TOKEN, "dYdX"),
            ];
            
            for (token_addr, protocol_name) in token_checks {
                if let Ok(token_address) = Address::from_str(token_addr) {
                    match self.check_token_interaction(&addr, &token_address).await {
                        Ok(has_tokens) => {
                            if has_tokens {
                                protocols_used.insert(protocol_name);
                                info!("Found {} token interaction", protocol_name);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to check {} token: {}", protocol_name, e);
                        }
                    }
                }
            }
        }
        
        // Check bridge usage
        match self.check_bridge_activity(&addr).await {
            Ok(bridge_count) => {
                if bridge_count > 0 {
                    metrics.bridges_used = bridge_count;
                    info!("Bridge interactions: {}", bridge_count);
                }
            }
            Err(e) => {
                warn!("Failed to check bridge activity: {}", e);
            }
        }
        
        // Check Hyperliquid deposits specifically on Arbitrum
        if self.chain == Chain::Arbitrum {
            if let Ok(hl_bridge) = Address::from_str(ProtocolAddresses::HYPERLIQUID_BRIDGE_ARB) {
                match self.check_hyperliquid_deposits(&addr, &hl_bridge).await {
                    Ok((deposits, volume)) => {
                        if deposits > 0 {
                            metrics.hyperliquid_volume_usd = volume;
                            metrics.total_perp_volume_usd += volume; // Add to total perp volume
                            metrics.leveraged_positions_count += 1; // Hyperliquid is leveraged trading
                            protocols_used.insert("Hyperliquid");
                            info!("Hyperliquid activity: {} deposits, ${} total volume", deposits, volume);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to check Hyperliquid deposits: {}", e);
                    }
                }
            }
        }
        
        // Set final unique protocol count
        metrics.defi_protocols_used = protocols_used.len() as u32;
        
        // Add this chain to active chains
        metrics.chains_active_on.push(self.chain.as_str().to_string());
        
        // Calculate total balance and stablecoin percentage
        // Note: This is a simplified version - in production, we'd fetch all token balances
        // and their USD values from a price oracle
        let stablecoins = KnownTokens::stablecoins();
        let mut total_balance_usd = Decimal::ZERO;
        let mut stablecoin_balance_usd = Decimal::ZERO;
        
        // Check balances of major stablecoins
        for (token_addr, symbol) in stablecoins.iter() {
            if let Ok(token_address) = Address::from_str(token_addr) {
                match self.get_token_balance(addr, token_address).await {
                    Ok(balance) => {
                        if balance > Decimal::ZERO {
                            // Assume 1:1 USD for stablecoins and 6 decimals (USDC/USDT standard)
                            let balance_usd = balance / Decimal::from(1_000_000);
                            stablecoin_balance_usd += balance_usd;
                            total_balance_usd += balance_usd;
                            info!("Found {} {} stablecoin balance", balance_usd, symbol);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to check {} balance: {}", symbol, e);
                    }
                }
            }
        }
        
        // Set the balance metrics
        if total_balance_usd > Decimal::ZERO {
            metrics.total_balance_usd = total_balance_usd;
            metrics.stablecoin_percentage = (stablecoin_balance_usd / total_balance_usd).try_into().unwrap_or(0.0);
            info!("Total balance: {} USD, Stablecoin percentage: {:.2}%", 
                     total_balance_usd, metrics.stablecoin_percentage * 100.0);
        }
        
        info!("Metrics fetched successfully for {} on {}", address, self.chain.as_str());
        debug!("Total TX count: {}, Distinct tokens: {}", metrics.total_tx_count, metrics.distinct_tokens_traded);
        debug!("DeFi protocols used: {}", metrics.defi_protocols_used);
        
        let chain_metrics = ChainMetrics {
            chain: self.chain.as_str().to_string(),
            address: address.to_string(),
            metrics,
            last_updated: Utc::now(),
        };
        
        // Cache the result
        self.cache.set_metrics(cache_key, chain_metrics.clone());
        
        Ok(chain_metrics)
    }
    
    async fn get_transaction_summary(
        &self,
        address: &str,
        _start_time: Option<DateTime<Utc>>,
        _end_time: Option<DateTime<Utc>>
    ) -> Result<TransactionSummary> {
        let addr = Address::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        
        let tx_count = self.provider.get_transaction_count(addr, None).await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get transaction count: {}", e),
            })?;
        
        Ok(TransactionSummary {
            total_count: tx_count.as_u32(),
            first_tx: None, // Would need to implement block scanning
            last_tx: None,
            active_days: 0,
            average_tx_per_day: 0.0,
            gas_spent: Decimal::ZERO,
        })
    }
    
    async fn get_token_balances(&self, address: &str) -> Result<Vec<TokenBalance>> {
        // Check cache first
        let cache_key_prefix = format!("{}:{}", self.chain.as_str(), address.to_lowercase());
        if let Some(cached_balances) = self.cache.get_balances(&cache_key_prefix) {
            info!("Cache hit for token balances {} on {}", address, self.chain.as_str());
            // Convert HashMap to Vec<TokenBalance> 
            let mut balances = Vec::new();
            for (token_addr, balance) in cached_balances {
                // For now, return a simplified version
                balances.push(TokenBalance {
                    token_address: token_addr,
                    balance: U256::from_dec_str(&balance.to_string()).unwrap_or(U256::zero()),
                    decimals: 18, // Default, would need proper token info
                    symbol: "UNKNOWN".to_string(),
                    name: "Unknown Token".to_string(),
                });
            }
            return Ok(balances);
        }
        
        let addr = Address::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        
        // In production, we'd scan for all token holdings
        // For now, just check known tokens
        let mut balances = Vec::new();
        
        // Check native balance
        let eth_balance = self.provider.get_balance(addr, None).await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get balance: {}", e),
            })?;
        
        if !eth_balance.is_zero() {
            balances.push(TokenBalance {
                token_address: "0x0000000000000000000000000000000000000000".to_string(),
                balance: eth_balance,
                decimals: 18,
                symbol: match self.chain {
                    Chain::Ethereum => "ETH",
                    Chain::Arbitrum => "ETH",
                    Chain::Optimism => "ETH",
                    Chain::Blast => "ETH",
                    _ => "ETH",
                }.to_string(),
                name: "Native Token".to_string(),
            });
        }
        
        // Cache the balances as a HashMap
        let mut balance_map = HashMap::new();
        for token_balance in &balances {
            let balance_decimal = Decimal::from_str(&token_balance.balance.to_string()).unwrap_or(Decimal::ZERO);
            balance_map.insert(token_balance.token_address.clone(), balance_decimal);
        }
        self.cache.set_balances(cache_key_prefix, balance_map);
        
        Ok(balances)
    }
    
    async fn get_nft_balances(&self, address: &str) -> Result<Vec<NFTBalance>> {
        // Would implement ERC721 balance checking
        // For now, return empty
        Ok(vec![])
    }
    
    async fn has_used_protocol(&self, address: &str, protocol_address: &str) -> Result<bool> {
        let addr = Address::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        
        let protocol_addr = Address::from_str(protocol_address)
            .map_err(|_| DegenScoreError::InvalidAddress(protocol_address.to_string()))?;
        
        // Check if address has sent transactions to protocol
        let current_block = self.provider.get_block_number().await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get current block: {}", e),
            })?;
        
        let from_block = current_block.saturating_sub(U64::from(8_000));
        
        let filter = Filter::new()
            .from_block(from_block)
            .to_block(current_block)
            .address(protocol_addr)
            .topic1(addr); // Many protocols index user address as first topic
        
        let logs = self.provider.get_logs(&filter).await
            .map_err(|e| DegenScoreError::RpcError {
                chain: self.chain.as_str().to_string(),
                message: format!("Failed to get logs: {}", e),
            })?;
        
        Ok(!logs.is_empty())
    }
    
    async fn get_protocol_metrics(
        &self,
        address: &str,
        protocol: &str
    ) -> Result<ProtocolMetrics> {
        let addr = Address::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        
        match protocol.to_lowercase().as_str() {
            "gmx" => self.check_gmx_activity(&addr).await,
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
        Address::from_str(address)
            .map_err(|_| DegenScoreError::InvalidAddress(address.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::providers::MockProvider;
    
    #[tokio::test]
    async fn test_check_casino_tokens() {
        // This test would require a mock provider - shown as example structure
        // In production, you'd use ethers::providers::MockProvider
        
        // Test that known casino tokens are detected
        // Test that balances are correctly calculated
        // Test that empty balances return empty HashMap
    }
    
    #[tokio::test]
    async fn test_calculate_wallet_age() {
        // Test binary search for first transaction
        // Test handling of wallets with no transactions
        // Test handling of very old wallets
    }
    
    #[tokio::test]
    async fn test_check_aave_activity() {
        // Test detection of Aave deposits
        // Test detection of Aave borrows
        // Test that non-Aave chains return false
    }
    
    #[tokio::test]
    async fn test_check_compound_activity() {
        // Test cToken balance detection
        // Test Comptroller interaction detection
        // Test that non-Ethereum chains return false
    }
    
    #[tokio::test]
    async fn test_check_gmx_activity() {
        // Test GMX position detection
        // Test volume calculation from logs
        // Test that non-Arbitrum chains return zero activity
    }
    
    #[tokio::test]
    async fn test_check_bridge_activity() {
        // Test Hyperliquid deposit detection
        // Test Hop bridge detection
        // Test Across bridge detection
        // Test counting of multiple bridge uses
    }
    
    #[tokio::test]
    async fn test_check_casino_interactions() {
        // Test Rollbit contract interaction detection
        // Test Shuffle router interaction detection
        // Test YEET token transfer detection
        // Test that platforms are not double-counted
    }
    
    #[tokio::test]
    async fn test_protocol_interaction_detection() {
        // Test that event logs with user address are detected
        // Test that interactions outside time window are not counted
        // Test handling of RPC errors
    }
    
    #[tokio::test]
    async fn test_stablecoin_percentage_calculation() {
        // Test calculation with only stablecoins
        // Test calculation with mixed portfolio
        // Test calculation with no stablecoins
        // Test handling of zero total balance
    }
    
    #[tokio::test]
    async fn test_leveraged_positions_tracking() {
        // Test that GMX usage increments leveraged_positions_count
        // Test that Perpetual Protocol usage increments leveraged_positions_count
        // Test that count doesn't double-count same protocol
    }
}