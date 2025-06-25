use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegenScore {
    pub user_id: String,
    pub total_score: f64,
    pub percentile: f64,
    pub breakdown: HashMap<String, f64>,
    pub calculated_at: DateTime<Utc>,
    pub tier: String,
    pub airdrop_eligible: bool,
    pub airdrop_allocation: Option<f64>,
}

impl DegenScore {
    pub fn tier_from_score(score: f64) -> &'static str {
        match score {
            s if s >= 90.0 => "Legendary",
            s if s >= 75.0 => "Diamond",
            s if s >= 60.0 => "Platinum",
            s if s >= 45.0 => "Gold",
            s if s >= 30.0 => "Silver",
            s if s >= 20.0 => "Bronze",
            _ => "Novice",
        }
    }
}