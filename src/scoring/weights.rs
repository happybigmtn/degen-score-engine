use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    pub trading_volume: f64,
    pub trading_count: f64,
    pub gambling_platforms: f64,
    pub casino_tokens: f64,
    pub defi_protocols: f64,
    pub token_diversity: f64,
    pub nft_holdings: f64,
    pub wallet_age: f64,
    pub activity_consistency: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            trading_volume: 15.0,
            trading_count: 10.0,
            gambling_platforms: 10.0,
            casino_tokens: 5.0,
            defi_protocols: 10.0,
            token_diversity: 5.0,
            nft_holdings: 10.0,
            wallet_age: 10.0,
            activity_consistency: 10.0,
        }
    }
}

impl ScoringWeights {
    pub fn total(&self) -> f64 {
        self.trading_volume +
        self.trading_count +
        self.gambling_platforms +
        self.casino_tokens +
        self.defi_protocols +
        self.token_diversity +
        self.nft_holdings +
        self.wallet_age +
        self.activity_consistency
    }
    
    pub fn validate(&self) -> Result<(), String> {
        let total = self.total();
        if (total - 100.0).abs() > 0.01 {
            return Err(format!("Weights must sum to 100, got {}", total));
        }
        
        // Check that all weights are non-negative
        if self.trading_volume < 0.0 ||
           self.trading_count < 0.0 ||
           self.gambling_platforms < 0.0 ||
           self.casino_tokens < 0.0 ||
           self.defi_protocols < 0.0 ||
           self.token_diversity < 0.0 ||
           self.nft_holdings < 0.0 ||
           self.wallet_age < 0.0 ||
           self.activity_consistency < 0.0 {
            return Err("All weights must be non-negative".to_string());
        }
        
        Ok(())
    }
    
    pub fn normalize(&mut self) {
        let total = self.total();
        if total > 0.0 {
            let factor = 100.0 / total;
            self.trading_volume *= factor;
            self.trading_count *= factor;
            self.gambling_platforms *= factor;
            self.casino_tokens *= factor;
            self.defi_protocols *= factor;
            self.token_diversity *= factor;
            self.nft_holdings *= factor;
            self.wallet_age *= factor;
            self.activity_consistency *= factor;
        }
    }
}