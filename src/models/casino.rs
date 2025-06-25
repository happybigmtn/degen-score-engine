use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasinoInteraction {
    pub platform: CasinoPlatform,
    pub interaction_type: InteractionType,
    pub timestamp: DateTime<Utc>,
    pub value_usd: Option<Decimal>,
    pub tx_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CasinoPlatform {
    Rollbit,
    Shuffle,
    Yeet,
    Winr,
    ChipBets,
    Unknown(String),
}

impl CasinoPlatform {
    pub fn from_contract(address: &str) -> Option<Self> {
        match address.to_lowercase().as_str() {
            // Rollbit contracts
            "0xda83c3bdbed4ec35f87d75d718556dd60e07f201" => Some(CasinoPlatform::Rollbit),
            "0x6ef13c2dbdcf8691d8d311f7e4558b5b3eb3d3c7" => Some(CasinoPlatform::Rollbit),
            // Shuffle contracts
            "0xa56472f02f29b3c3b5e29f0be08bb3639abe86c0" => Some(CasinoPlatform::Shuffle),
            // Add more as discovered
            _ => None,
        }
    }
    
    pub fn from_token(token_symbol: &str) -> Option<Self> {
        match token_symbol.to_uppercase().as_str() {
            "RLB" => Some(CasinoPlatform::Rollbit),
            "SHFL" => Some(CasinoPlatform::Shuffle),
            "YEET" => Some(CasinoPlatform::Yeet),
            "WINR" => Some(CasinoPlatform::Winr),
            "CHIPS" => Some(CasinoPlatform::ChipBets),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InteractionType {
    Deposit,
    Withdrawal,
    Bet,
    Stake,
    TokenTransfer,
    ContractInteraction,
}

#[derive(Debug, Clone, Default)]
pub struct CasinoMetrics {
    pub platforms_used: HashSet<CasinoPlatform>,
    pub total_interactions: u32,
    pub total_volume_usd: Decimal,
    pub last_interaction: Option<DateTime<Utc>>,
    pub interactions: Vec<CasinoInteraction>,
}