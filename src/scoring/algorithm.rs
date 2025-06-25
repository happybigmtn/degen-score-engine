use crate::{
    models::{DegenMetrics, DegenScore, ScoreBreakdown, ScoreTier},
    config::ScoringWeights,
};
use rust_decimal::Decimal;
use chrono::Utc;
use std::cmp::min;

pub struct ScoringAlgorithm {
    weights: ScoringWeights,
    max_trading_volume: f64,
    max_trades_count: u32,
    max_protocols_count: u32,
    max_nft_count: u32,
    max_wallet_age_days: u32,
}

impl ScoringAlgorithm {
    pub fn new(weights: ScoringWeights) -> Self {
        Self {
            weights,
            max_trading_volume: 10_000_000.0, // $10M
            max_trades_count: 100,
            max_protocols_count: 20,
            max_nft_count: 50,
            max_wallet_age_days: 1825, // 5 years
        }
    }
    
    pub fn calculate_score(&self, metrics: &DegenMetrics) -> DegenScore {
        let breakdown = self.calculate_breakdown(metrics);
        let total_score = self.sum_breakdown(&breakdown);
        
        DegenScore {
            total_score,
            percentile: 0.0, // Would be calculated based on all users
            breakdown,
            calculated_at: Utc::now(),
            tier: ScoreTier::from_score(total_score),
        }
    }
    
    fn calculate_breakdown(&self, metrics: &DegenMetrics) -> ScoreBreakdown {
        ScoreBreakdown {
            trading_score: self.calculate_trading_score(metrics),
            gambling_score: self.calculate_gambling_score(metrics),
            defi_activity_score: self.calculate_defi_score(metrics),
            nft_portfolio_score: self.calculate_nft_score(metrics),
            longevity_score: self.calculate_longevity_score(metrics),
            risk_profile_score: self.calculate_risk_score(metrics),
        }
    }
    
    fn sum_breakdown(&self, breakdown: &ScoreBreakdown) -> f64 {
        let base_score = breakdown.trading_score +
            breakdown.gambling_score +
            breakdown.defi_activity_score +
            breakdown.nft_portfolio_score +
            breakdown.longevity_score;
        
        // Add risk profile bonus (up to 15 points)
        base_score + breakdown.risk_profile_score
    }
    
    fn calculate_trading_score(&self, metrics: &DegenMetrics) -> f64 {
        let volume_score = self.calculate_volume_component(metrics);
        let count_score = self.calculate_trades_count_component(metrics);
        
        volume_score + count_score
    }
    
    fn calculate_volume_component(&self, metrics: &DegenMetrics) -> f64 {
        let total_volume = metrics.gmx_volume_usd + metrics.total_perp_volume_usd;
        let volume_f64: f64 = total_volume.try_into().unwrap_or(0.0);
        
        if volume_f64 <= 0.0 {
            return 0.0;
        }
        
        // Logarithmic scaling for volume
        let capped_volume = volume_f64.min(self.max_trading_volume);
        let volume_factor = (capped_volume.ln() / self.max_trading_volume.ln()).min(1.0).max(0.0);
        
        volume_factor * self.weights.trading_volume
    }
    
    fn calculate_trades_count_component(&self, metrics: &DegenMetrics) -> f64 {
        let total_trades = metrics.gmx_trades + metrics.jupiter_swaps;
        let trades_factor = (total_trades as f64 / self.max_trades_count as f64).min(1.0);
        
        trades_factor * self.weights.trading_count
    }
    
    fn calculate_gambling_score(&self, metrics: &DegenMetrics) -> f64 {
        // Platforms used component
        let platforms_score = {
            let platforms_factor = (metrics.casinos_used.min(3) as f64 / 3.0).min(1.0);
            platforms_factor * self.weights.gambling_platforms
        };
        
        // Casino tokens held component
        let tokens_score = {
            let token_count = metrics.casino_tokens_held.len() as u32;
            let tokens_factor = (token_count.min(2) as f64 / 2.0).min(1.0);
            tokens_factor * self.weights.casino_tokens
        };
        
        platforms_score + tokens_score
    }
    
    fn calculate_defi_score(&self, metrics: &DegenMetrics) -> f64 {
        // Protocol diversity
        let protocols_score = {
            let protocols_factor = (metrics.defi_protocols_used as f64 / 
                self.max_protocols_count as f64).min(1.0);
            protocols_factor * self.weights.defi_protocols
        };
        
        // Token diversity (memecoin trading indicator)
        let tokens_score = {
            let tokens_factor = (metrics.distinct_tokens_traded as f64 / 50.0).min(1.0);
            tokens_factor * self.weights.token_diversity
        };
        
        protocols_score + tokens_score
    }
    
    fn calculate_nft_score(&self, metrics: &DegenMetrics) -> f64 {
        // NFT count component (50% of NFT score)
        let count_score = {
            let count_factor = (metrics.nft_count as f64 / self.max_nft_count as f64).min(1.0);
            count_factor * (self.weights.nft_holdings / 2.0)
        };
        
        // NFT value component (50% of NFT score)
        let value_score = {
            let value_f64: f64 = metrics.nft_total_value_usd.try_into().unwrap_or(0.0);
            let value_factor = if value_f64 > 0.0 {
                (value_f64.min(100_000.0) / 100_000.0).min(1.0)
            } else {
                0.0
            };
            value_factor * (self.weights.nft_holdings / 2.0)
        };
        
        count_score + value_score
    }
    
    fn calculate_longevity_score(&self, metrics: &DegenMetrics) -> f64 {
        // Wallet age component (50% of longevity score)
        let age_score = {
            let age_factor = (metrics.wallet_age_days as f64 / 
                self.max_wallet_age_days as f64).min(1.0);
            age_factor * (self.weights.wallet_age / 2.0)
        };
        
        // Activity consistency component (50% of longevity score)
        let consistency_score = {
            let consistency_factor = (metrics.active_days as f64 / 365.0).min(1.0);
            consistency_factor * (self.weights.activity_consistency / 2.0)
        };
        
        age_score + consistency_score
    }
    
    fn calculate_risk_score(&self, metrics: &DegenMetrics) -> f64 {
        let mut risk_score = 0.0;
        
        // High volatility portfolio (less stablecoins = more degen)
        if metrics.total_balance_usd > Decimal::ZERO {
            let volatile_factor = 1.0 - metrics.stablecoin_percentage;
            risk_score += volatile_factor * 2.5;
        }
        
        // Multiple chains active (cross-chain degen)
        let chains_factor = (metrics.chains_active_on.len() as f64 / 5.0).min(1.0);
        risk_score += chains_factor * 2.5;
        
        // Leveraged positions
        if metrics.leveraged_positions_count > 0 {
            risk_score += 2.5;
        }
        
        // Survived liquidations/rugpulls (battle-tested degen)
        if metrics.liquidations_count > 0 || metrics.rugpull_exposure_count > 0 {
            risk_score += 2.5;
        }
        
        risk_score.min(10.0) // Cap at 10 points
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    fn default_weights() -> ScoringWeights {
        ScoringWeights {
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
    
    #[test]
    fn test_zero_metrics_zero_score() {
        let algo = ScoringAlgorithm::new(default_weights());
        let metrics = DegenMetrics::default();
        let score = algo.calculate_score(&metrics);
        
        assert_eq!(score.total_score, 0.0);
        assert_eq!(score.tier, ScoreTier::Novice);
    }
    
    #[test]
    fn test_max_trading_score() {
        let algo = ScoringAlgorithm::new(default_weights());
        let mut metrics = DegenMetrics::default();
        
        metrics.gmx_volume_usd = Decimal::from(10_000_000);
        metrics.gmx_trades = 100;
        
        let score = algo.calculate_score(&metrics);
        let expected_trading_score = 15.0 + 10.0; // volume + count weights
        
        assert!((score.breakdown.trading_score - expected_trading_score).abs() < 0.1);
    }
}