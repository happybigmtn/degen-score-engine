use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DegenMetrics {
    // Trading metrics
    pub gmx_volume_usd: Decimal,
    pub gmx_trades: u32,
    pub jupiter_swaps: u32,
    pub bridges_used: u32,  // Number of bridge interactions (Hyperliquid, Hop, Across, etc.)
    pub hyperliquid_volume_usd: Decimal, // Total USDC deposited to Hyperliquid
    pub total_perp_volume_usd: Decimal,
    
    // Gambling metrics
    pub casinos_used: u32,
    pub casino_tokens_held: HashMap<String, Decimal>, // token_symbol -> amount
    pub gambling_volume_usd: Decimal,
    
    // DeFi metrics
    pub defi_protocols_used: u32,
    pub distinct_tokens_traded: u32,
    pub memecoin_trades: u32,
    pub total_swap_volume_usd: Decimal,
    pub liquidity_provided_usd: Decimal,
    
    // NFT metrics
    pub nft_count: u32,
    pub nft_collections_count: u32,
    pub nft_total_value_usd: Decimal,
    pub nft_trades: u32,
    
    // Portfolio metrics
    pub total_balance_usd: Decimal,
    pub stablecoin_percentage: f64,
    pub volatile_token_count: u32,
    pub largest_position_percentage: f64,
    
    // Activity metrics
    pub total_tx_count: u32,
    pub wallet_age_days: u32,
    pub active_days: u32,
    pub active_months: u32,
    pub gas_spent_usd: Decimal,
    pub chains_active_on: Vec<String>,
    
    // Time-based metrics
    pub first_transaction: Option<DateTime<Utc>>,
    pub last_transaction: Option<DateTime<Utc>>,
    pub most_active_period: Option<(DateTime<Utc>, DateTime<Utc>)>,
    
    // Risk metrics
    pub leveraged_positions_count: u32,
    pub liquidations_count: u32,
    pub rugpull_exposure_count: u32,
    pub max_single_loss_usd: Decimal,
    
    // Enhanced protocol tracking
    pub protocol_interaction_counts: HashMap<String, u32>, // protocol_name -> interaction_count
    pub protocol_volume_usd: HashMap<String, Decimal>, // protocol_name -> total_volume_usd
    pub protocol_first_use: HashMap<String, DateTime<Utc>>, // protocol_name -> first_interaction_timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMetrics {
    pub chain: String,
    pub address: String,
    pub metrics: DegenMetrics,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolInteraction {
    pub protocol_name: String,
    pub protocol_type: ProtocolType,
    pub chain: String,
    pub contract_address: String,
    pub interaction_count: u32,
    pub volume_usd: Decimal,
    pub first_interaction: DateTime<Utc>,
    pub last_interaction: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProtocolType {
    DEX,
    PerpetualExchange,
    LendingProtocol,
    YieldFarm,
    Casino,
    NFTMarketplace,
    Bridge,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenHolding {
    pub token_address: String,
    pub token_symbol: String,
    pub token_name: String,
    pub balance: Decimal,
    pub decimals: u8,
    pub value_usd: Decimal,
    pub token_type: TokenType,
    pub chain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenType {
    Stablecoin,
    BluechipCrypto,
    CasinoToken,
    Memecoin,
    GovernanceToken,
    LPToken,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NFTHolding {
    pub collection_address: String,
    pub collection_name: String,
    pub token_id: String,
    pub chain: String,
    pub estimated_value_usd: Option<Decimal>,
    pub rarity_score: Option<f64>,
    pub acquisition_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSummary {
    pub total_count: u32,
    pub first_tx: Option<DateTime<Utc>>,
    pub last_tx: Option<DateTime<Utc>>,
    pub active_days: u32,
    pub average_tx_per_day: f64,
    pub gas_spent: Decimal,
}

impl DegenMetrics {
    pub fn merge(&mut self, other: &DegenMetrics) {
        self.gmx_volume_usd += other.gmx_volume_usd;
        self.gmx_trades += other.gmx_trades;
        self.jupiter_swaps += other.jupiter_swaps;
        self.bridges_used += other.bridges_used;
        self.hyperliquid_volume_usd += other.hyperliquid_volume_usd;
        self.total_perp_volume_usd += other.total_perp_volume_usd;
        
        self.casinos_used += other.casinos_used;
        for (token, amount) in &other.casino_tokens_held {
            *self.casino_tokens_held.entry(token.clone()).or_insert(Decimal::ZERO) += amount;
        }
        self.gambling_volume_usd += other.gambling_volume_usd;
        
        self.defi_protocols_used += other.defi_protocols_used;
        self.distinct_tokens_traded += other.distinct_tokens_traded;
        self.memecoin_trades += other.memecoin_trades;
        self.total_swap_volume_usd += other.total_swap_volume_usd;
        self.liquidity_provided_usd += other.liquidity_provided_usd;
        
        self.nft_count += other.nft_count;
        self.nft_collections_count += other.nft_collections_count;
        self.nft_total_value_usd += other.nft_total_value_usd;
        self.nft_trades += other.nft_trades;
        
        self.total_balance_usd += other.total_balance_usd;
        self.volatile_token_count += other.volatile_token_count;
        
        self.total_tx_count += other.total_tx_count;
        self.wallet_age_days = self.wallet_age_days.max(other.wallet_age_days);
        self.active_days += other.active_days;
        self.active_months += other.active_months;
        self.gas_spent_usd += other.gas_spent_usd;
        
        for chain in &other.chains_active_on {
            if !self.chains_active_on.contains(chain) {
                self.chains_active_on.push(chain.clone());
            }
        }
        
        self.leveraged_positions_count += other.leveraged_positions_count;
        self.liquidations_count += other.liquidations_count;
        self.rugpull_exposure_count += other.rugpull_exposure_count;
        
        if other.max_single_loss_usd > self.max_single_loss_usd {
            self.max_single_loss_usd = other.max_single_loss_usd;
        }
        
        // Merge protocol tracking data
        for (protocol, count) in &other.protocol_interaction_counts {
            *self.protocol_interaction_counts.entry(protocol.clone()).or_insert(0) += count;
        }
        
        for (protocol, volume) in &other.protocol_volume_usd {
            *self.protocol_volume_usd.entry(protocol.clone()).or_insert(Decimal::ZERO) += volume;
        }
        
        for (protocol, timestamp) in &other.protocol_first_use {
            self.protocol_first_use.entry(protocol.clone())
                .and_modify(|existing| {
                    if timestamp < existing {
                        *existing = *timestamp;
                    }
                })
                .or_insert(*timestamp);
        }
        
        // Update time-based metrics
        if let Some(other_first) = other.first_transaction {
            self.first_transaction = Some(match self.first_transaction {
                Some(self_first) => self_first.min(other_first),
                None => other_first,
            });
        }
        
        if let Some(other_last) = other.last_transaction {
            self.last_transaction = Some(match self.last_transaction {
                Some(self_last) => self_last.max(other_last),
                None => other_last,
            });
        }
    }
}