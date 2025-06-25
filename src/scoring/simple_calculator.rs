use crate::{
    models::{Chain},
    chains::{ChainClient, EvmClient, SolanaClient},
    config::{RpcConfig, ScoringWeights},
    scoring::algorithm::ScoringAlgorithm,
};
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, warn};
use chrono::Utc;

pub struct ScoreCalculator {
    eth_client: Arc<EvmClient>,
    arb_client: Arc<EvmClient>,
    opt_client: Arc<EvmClient>,
    blast_client: Arc<EvmClient>,
    sol_client: Arc<SolanaClient>,
    algorithm: ScoringAlgorithm,
}

impl ScoreCalculator {
    pub async fn new() -> Result<Self> {
        let config = RpcConfig::default();
        
        // Create chain clients
        let eth_client = Arc::new(EvmClient::new(
            crate::chains::client::ChainClientConfig {
                rpc_url: config.get_primary_endpoint(&Chain::Ethereum)
                    .ok_or_else(|| anyhow::anyhow!("No Ethereum RPC endpoint"))?
                    .url.clone(),
                chain_id: Some(1),
                timeout_seconds: 30,
                max_retries: 3,
                rate_limit_per_second: 5.0,
            },
            Chain::Ethereum,
        ).await?);
        
        let arb_client = Arc::new(EvmClient::new(
            crate::chains::client::ChainClientConfig {
                rpc_url: config.get_primary_endpoint(&Chain::Arbitrum)
                    .ok_or_else(|| anyhow::anyhow!("No Arbitrum RPC endpoint"))?
                    .url.clone(),
                chain_id: Some(42161),
                timeout_seconds: 30,
                max_retries: 3,
                rate_limit_per_second: 5.0,
            },
            Chain::Arbitrum,
        ).await?);
        
        let opt_client = Arc::new(EvmClient::new(
            crate::chains::client::ChainClientConfig {
                rpc_url: config.get_primary_endpoint(&Chain::Optimism)
                    .ok_or_else(|| anyhow::anyhow!("No Optimism RPC endpoint"))?
                    .url.clone(),
                chain_id: Some(10),
                timeout_seconds: 30,
                max_retries: 3,
                rate_limit_per_second: 5.0,
            },
            Chain::Optimism,
        ).await?);
        
        let blast_client = Arc::new(EvmClient::new(
            crate::chains::client::ChainClientConfig {
                rpc_url: config.get_primary_endpoint(&Chain::Blast)
                    .ok_or_else(|| anyhow::anyhow!("No Blast RPC endpoint"))?
                    .url.clone(),
                chain_id: Some(81457),
                timeout_seconds: 30,
                max_retries: 3,
                rate_limit_per_second: 5.0,
            },
            Chain::Blast,
        ).await?);
        
        let sol_client = Arc::new(SolanaClient::new(
            crate::chains::client::ChainClientConfig {
                rpc_url: config.get_primary_endpoint(&Chain::Solana)
                    .ok_or_else(|| anyhow::anyhow!("No Solana RPC endpoint"))?
                    .url.clone(),
                chain_id: None,
                timeout_seconds: 30,
                max_retries: 3,
                rate_limit_per_second: 10.0,
            },
        )?);
        
        Ok(Self {
            eth_client,
            arb_client,
            opt_client,
            blast_client,
            sol_client,
            algorithm: ScoringAlgorithm::new(ScoringWeights::default()),
        })
    }
    
    pub async fn calculate_score(
        &self,
        user_id: &str,
        eth_address: Option<String>,
        arb_address: Option<String>,
        opt_address: Option<String>,
        blast_address: Option<String>,
        sol_address: Option<String>,
    ) -> Result<crate::models::DegenScore> {
        let mut all_metrics = Vec::new();
        
        // Fetch metrics from each chain in parallel
        let mut tasks = Vec::new();
        
        if let Some(addr) = eth_address {
            let client = self.eth_client.clone();
            tasks.push(tokio::spawn(async move {
                client.fetch_metrics(&addr).await
            }));
        }
        
        if let Some(addr) = arb_address {
            let client = self.arb_client.clone();
            tasks.push(tokio::spawn(async move {
                client.fetch_metrics(&addr).await
            }));
        }
        
        if let Some(addr) = opt_address {
            let client = self.opt_client.clone();
            tasks.push(tokio::spawn(async move {
                client.fetch_metrics(&addr).await
            }));
        }
        
        if let Some(addr) = blast_address {
            let client = self.blast_client.clone();
            tasks.push(tokio::spawn(async move {
                client.fetch_metrics(&addr).await
            }));
        }
        
        if let Some(addr) = sol_address {
            let client = self.sol_client.clone();
            tasks.push(tokio::spawn(async move {
                client.fetch_metrics(&addr).await
            }));
        }
        
        // Wait for all tasks and collect results
        for task in tasks {
            match task.await? {
                Ok(metrics) => all_metrics.push(metrics),
                Err(e) => warn!("Failed to fetch metrics: {}", e),
            }
        }
        
        // Aggregate metrics
        let mut aggregated = crate::models::DegenMetrics::default();
        for metrics in &all_metrics {
            aggregated.merge(&metrics.metrics);
        }
        
        // Calculate score
        let score_result = self.algorithm.calculate_score(&aggregated);
        let total_score = score_result.total_score;
        
        
        let tier = crate::models::ScoreTier::from_score(total_score);
        
        Ok(crate::models::DegenScore {
            total_score,
            percentile: 0.0, // Would calculate from database in production
            breakdown: score_result.breakdown,
            calculated_at: Utc::now(),
            tier,
        })
    }
}