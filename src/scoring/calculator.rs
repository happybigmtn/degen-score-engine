use crate::{
    models::{UserProfile, DegenMetrics, DegenScore, Chain, Result},
    chains::ChainClient,
    scoring::ScoringAlgorithm,
    config::Settings,
};
use std::sync::Arc;
// use futures::future::join_all;  // Not needed with sequential approach
use tracing::{info, warn, error};

pub struct ScoreCalculator {
    evm_clients: Vec<Arc<dyn ChainClient>>,
    solana_client: Arc<dyn ChainClient>,
    algorithm: ScoringAlgorithm,
    settings: Settings,
}

impl ScoreCalculator {
    pub fn new(
        evm_clients: Vec<Arc<dyn ChainClient>>,
        solana_client: Arc<dyn ChainClient>,
        settings: Settings,
    ) -> Self {
        let algorithm = ScoringAlgorithm::new(settings.scoring.weights.clone());
        
        Self {
            evm_clients,
            solana_client,
            algorithm,
            settings,
        }
    }
    
    pub async fn calculate_user_score(&self, user: &UserProfile) -> Result<DegenScore> {
        info!("Calculating score for user: {}", user.id);
        
        // Aggregate metrics from all chains
        let mut aggregated_metrics = DegenMetrics::default();
        let mut successful_fetches = 0;
        
        // EVM chains
        for client in &self.evm_clients {
            let chain = client.chain();
            let addresses = user.get_addresses_by_chain(chain.clone());
            
            for address in addresses {
                match client.fetch_metrics(address).await {
                    Ok(chain_metrics) => {
                        aggregated_metrics.merge(&chain_metrics.metrics);
                        successful_fetches += 1;
                        info!("Fetched metrics for {} on {}", address, chain.as_str());
                    }
                    Err(e) => {
                        warn!("Failed to fetch metrics for {} on {}: {}", 
                            address, chain.as_str(), e);
                    }
                }
            }
        }
        
        // Solana
        let solana_addresses = user.get_addresses_by_chain(Chain::Solana);
        for address in solana_addresses {
            match self.solana_client.fetch_metrics(address).await {
                Ok(chain_metrics) => {
                    aggregated_metrics.merge(&chain_metrics.metrics);
                    successful_fetches += 1;
                    info!("Fetched Solana metrics for {}", address);
                }
                Err(e) => {
                    warn!("Failed to fetch Solana metrics for {}: {}", address, e);
                }
            }
        }
        
        if successful_fetches == 0 {
            return Err(crate::models::DegenScoreError::ScoreCalculationError(
                "No metrics could be fetched from any chain".to_string()
            ));
        }
        
        info!("Successfully fetched metrics from {} chains/addresses", successful_fetches);
        
        // Calculate final score
        let score = self.algorithm.calculate_score(&aggregated_metrics);
        
        // Check if score meets minimum threshold
        if score.total_score < self.settings.scoring.min_score_for_airdrop {
            info!("User {} score {} is below minimum threshold {}", 
                user.id, score.total_score, self.settings.scoring.min_score_for_airdrop);
        }
        
        Ok(score)
    }
    
    pub async fn calculate_batch_scores(&self, users: &[UserProfile]) -> Vec<Result<DegenScore>> {
        info!("Calculating scores for {} users", users.len());
        
        let mut results = Vec::new();
        for user in users {
            results.push(self.calculate_user_score(user).await);
        }
        
        results
    }
    
    pub fn is_eligible_for_airdrop(&self, score: &DegenScore) -> bool {
        score.total_score >= self.settings.scoring.min_score_for_airdrop
    }
    
    pub fn calculate_airdrop_amount(&self, score: &DegenScore, total_pool: u64) -> u64 {
        if !self.is_eligible_for_airdrop(score) {
            return 0;
        }
        
        // This would be calculated based on all users' scores
        // For now, return a proportional amount based on score
        let score_factor = score.total_score / 100.0;
        let base_amount = (total_pool as f64 * score_factor) as u64;
        
        // Apply wagering requirement multiplier info
        base_amount
    }
}