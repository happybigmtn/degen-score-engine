use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub verified_addresses: Vec<VerifiedAddress>,
    pub aggregated_metrics: super::DegenMetrics,
    pub degen_score: Option<DegenScore>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedAddress {
    pub address: String,
    pub chain: Chain,
    pub verification_method: VerificationMethod,
    pub verified_at: DateTime<Utc>,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Chain {
    Ethereum,
    Arbitrum,
    Optimism,
    Blast,
    Solana,
}

impl Chain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Chain::Ethereum => "ethereum",
            Chain::Arbitrum => "arbitrum",
            Chain::Optimism => "optimism",
            Chain::Blast => "blast",
            Chain::Solana => "solana",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ethereum" | "eth" => Some(Chain::Ethereum),
            "arbitrum" | "arb" => Some(Chain::Arbitrum),
            "optimism" | "op" => Some(Chain::Optimism),
            "blast" => Some(Chain::Blast),
            "solana" | "sol" => Some(Chain::Solana),
            _ => None,
        }
    }
    
    pub fn is_evm(&self) -> bool {
        matches!(self, Chain::Ethereum | Chain::Arbitrum | Chain::Optimism | Chain::Blast)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationMethod {
    Signature { message: String, signature: String },
    MicroDeposit { tx_hash: String, amount: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegenScore {
    pub total_score: f64,
    pub percentile: f64,
    pub breakdown: ScoreBreakdown,
    pub calculated_at: DateTime<Utc>,
    pub tier: ScoreTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub trading_score: f64,
    pub gambling_score: f64,
    pub defi_activity_score: f64,
    pub nft_portfolio_score: f64,
    pub longevity_score: f64,
    pub risk_profile_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScoreTier {
    Legendary,    // 90-100
    Epic,         // 75-89
    Rare,         // 60-74
    Uncommon,     // 40-59
    Common,       // 20-39
    Novice,       // 0-19
}

impl ScoreTier {
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s >= 90.0 => ScoreTier::Legendary,
            s if s >= 75.0 => ScoreTier::Epic,
            s if s >= 60.0 => ScoreTier::Rare,
            s if s >= 40.0 => ScoreTier::Uncommon,
            s if s >= 20.0 => ScoreTier::Common,
            _ => ScoreTier::Novice,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressVerificationRequest {
    pub address: String,
    pub chain: Chain,
    pub nonce: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreCalculationRequest {
    pub user_id: String,
    pub addresses: Vec<(String, Chain)>,
    pub force_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirdropAllocation {
    pub user_id: String,
    pub score: f64,
    pub token_amount: u64,
    pub credits_amount: u64,
    pub wagering_requirement: u64,
    pub eligible_addresses: Vec<String>,
}

impl UserProfile {
    pub fn new(id: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            verified_addresses: Vec::new(),
            aggregated_metrics: super::DegenMetrics::default(),
            degen_score: None,
            created_at: now,
            updated_at: now,
        }
    }
    
    pub fn add_verified_address(&mut self, address: VerifiedAddress) {
        if !self.verified_addresses.iter().any(|a| a.address == address.address) {
            self.verified_addresses.push(address);
            self.updated_at = Utc::now();
        }
    }
    
    pub fn get_addresses_by_chain(&self, chain: Chain) -> Vec<&str> {
        self.verified_addresses
            .iter()
            .filter(|a| a.chain == chain)
            .map(|a| a.address.as_str())
            .collect()
    }
}