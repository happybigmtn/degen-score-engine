use crate::{
    models::{UserProfile, DegenMetrics, DegenScore, Chain, Result},
    chains::ChainClient,
    scoring::ScoringAlgorithm,
    config::Settings,
};
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;
use futures::future::join_all;
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
        
        // Collect all futures for parallel execution
        let mut metric_futures: Vec<Pin<Box<dyn Future<Output = Option<DegenMetrics>> + Send>>> = Vec::new();
        
        // EVM chains - create futures for parallel execution
        for client in &self.evm_clients {
            let chain = client.chain();
            let addresses = user.get_addresses_by_chain(chain.clone());
            
            for address in addresses {
                let client_ref = Arc::clone(client);
                let address_owned = address.to_string();
                let chain_name = chain.as_str().to_string();
                
                let future = Box::pin(async move {
                    match client_ref.fetch_metrics(&address_owned).await {
                        Ok(chain_metrics) => {
                            info!("Fetched metrics for {} on {}", address_owned, chain_name);
                            Some(chain_metrics.metrics)
                        }
                        Err(e) => {
                            warn!("Failed to fetch metrics for {} on {}: {}", 
                                address_owned, chain_name, e);
                            None
                        }
                    }
                });
                metric_futures.push(future);
            }
        }
        
        // Solana - add to parallel futures
        let solana_addresses = user.get_addresses_by_chain(Chain::Solana);
        for address in solana_addresses {
            let client_ref = Arc::clone(&self.solana_client);
            let address_owned = address.to_string();
            
            let future = Box::pin(async move {
                match client_ref.fetch_metrics(&address_owned).await {
                    Ok(chain_metrics) => {
                        info!("Fetched Solana metrics for {}", address_owned);
                        Some(chain_metrics.metrics)
                    }
                    Err(e) => {
                        warn!("Failed to fetch Solana metrics for {}: {}", address_owned, e);
                        None
                    }
                }
            });
            metric_futures.push(future);
        }
        
        // Execute all futures in parallel
        let results = join_all(metric_futures).await;
        
        // Aggregate all successful results
        let mut aggregated_metrics = DegenMetrics::default();
        let mut successful_fetches = 0;
        
        for result in results {
            if let Some(metrics) = result {
                aggregated_metrics.merge(&metrics);
                successful_fetches += 1;
            }
        }
        
        if successful_fetches == 0 {
            return Err(crate::models::DegenScoreError::ScoreCalculationError(
                "No metrics could be fetched from any chain".to_string()
            ));
        }
        
        info!("Successfully fetched metrics from {} chains/addresses in parallel", successful_fetches);
        
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
        info!("Calculating scores for {} users in parallel", users.len());
        
        // Create futures for parallel user score calculation
        let score_futures = users.iter().map(|user| {
            self.calculate_user_score(user)
        });
        
        // Execute all score calculations in parallel
        join_all(score_futures).await
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