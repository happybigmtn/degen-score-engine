use degen_scorer::{
    scoring::{ScoreCalculator, ScoringAlgorithm},
    chains::{EvmClient, ChainClient, client::ChainClientConfig},
    models::{UserProfile, VerifiedAddress, Chain, VerificationMethod},
    config::Settings,
};
use std::sync::Arc;
use tokio;
use tracing::{info, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("Starting Degen Score analysis for rng.eth and vitalik.eth");
    
    // Create default settings for testing
    let settings = Settings::default();
    
    // Create EVM clients for each chain
    let mut evm_clients: Vec<Arc<dyn ChainClient>> = Vec::new();
    
    // Ethereum client - using Quicknode endpoint with reduced rate limit
    let eth_config = ChainClientConfig {
        rpc_url: "https://twilight-crimson-sailboat.quiknode.pro/bffa6f76bba7ac3a3eaa237b6f0b35598e8b3981".to_string(),
        chain_id: Some(1),
        max_retries: 3,
        rate_limit_per_second: 0.5, // Extremely conservative - 1 call every 2 seconds
        timeout_seconds: 30,
    };
    let eth_client = Arc::new(EvmClient::new(eth_config, Chain::Ethereum).await?) as Arc<dyn ChainClient>;
    evm_clients.push(eth_client);
    
    // Arbitrum client - using public endpoint for now
    let arb_config = ChainClientConfig {
        rpc_url: "https://arbitrum-one.publicnode.com".to_string(),
        chain_id: Some(42161),
        max_retries: 3,
        rate_limit_per_second: 10.0,
        timeout_seconds: 30,
    };
    let arb_client = Arc::new(EvmClient::new(arb_config, Chain::Arbitrum).await?) as Arc<dyn ChainClient>;
    evm_clients.push(arb_client);
    
    // Optimism client - using public endpoint for now
    let opt_config = ChainClientConfig {
        rpc_url: "https://optimism.publicnode.com".to_string(),
        chain_id: Some(10),
        max_retries: 3,
        rate_limit_per_second: 10.0,
        timeout_seconds: 30,
    };
    let opt_client = Arc::new(EvmClient::new(opt_config, Chain::Optimism).await?) as Arc<dyn ChainClient>;
    evm_clients.push(opt_client);
    
    // For now, use mock Solana client
    use degen_scorer::chains::SolanaClient;
    let solana_config = ChainClientConfig {
        rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
        chain_id: None,
        max_retries: 3,
        rate_limit_per_second: 10.0,
        timeout_seconds: 30,
    };
    let solana_client = Arc::new(SolanaClient::new(solana_config)?) as Arc<dyn ChainClient>;
    
    // Create scoring calculator
    let calculator = ScoreCalculator::new(evm_clients, solana_client, settings);
    
    // Test addresses - we need to resolve ENS names first
    let test_cases = vec![
        ("rng.eth", "0x52A90BfEc58cc5394A52aD53Fc83ebEF5B0119b6"), // rng.eth resolved
    ];
    
    for (ens_name, address) in test_cases {
        info!("\n============================================================");
        info!("Analyzing: {} ({})", ens_name, address);
        info!("============================================================");
        
        // Create user profile
        let mut user = UserProfile::new(format!("user_{}", ens_name));
        
        // Add verified addresses for each chain
        user.add_verified_address(VerifiedAddress {
            address: address.to_string(),
            chain: Chain::Ethereum,
            verification_method: VerificationMethod::Signature {
                message: "Test verification".to_string(),
                signature: "0x123...".to_string(),
            },
            verified_at: chrono::Utc::now(),
            nonce: "test".to_string(),
        });
        
        user.add_verified_address(VerifiedAddress {
            address: address.to_string(),
            chain: Chain::Arbitrum,
            verification_method: VerificationMethod::Signature {
                message: "Test verification".to_string(),
                signature: "0x123...".to_string(),
            },
            verified_at: chrono::Utc::now(),
            nonce: "test".to_string(),
        });
        
        user.add_verified_address(VerifiedAddress {
            address: address.to_string(),
            chain: Chain::Optimism,
            verification_method: VerificationMethod::Signature {
                message: "Test verification".to_string(),
                signature: "0x123...".to_string(),
            },
            verified_at: chrono::Utc::now(),
            nonce: "test".to_string(),
        });
        
        // Calculate score
        match calculator.calculate_user_score(&user).await {
            Ok(score) => {
                info!("\n🎯 DEGEN SCORE: {:.1}/100", score.total_score);
                info!("🏆 Tier: {:?}", score.tier);
                info!("📊 Percentile: {:.1}%", score.percentile);
                
                info!("\n📊 Score Breakdown:");
                info!("  Trading Activity:    {:.1}", score.breakdown.trading_score);
                info!("  Gambling Activity:   {:.1}", score.breakdown.gambling_score);
                info!("  DeFi Activity:       {:.1}", score.breakdown.defi_activity_score);
                info!("  NFT Portfolio:       {:.1}", score.breakdown.nft_portfolio_score);
                info!("  Longevity:           {:.1}", score.breakdown.longevity_score);
                info!("  Risk Profile:        {:.1}", score.breakdown.risk_profile_score);
                
                info!("\n📈 User Metrics:");
                let metrics = &user.aggregated_metrics;
                info!("  Total TX Count:      {}", metrics.total_tx_count);
                info!("  Wallet Age (days):   {}", metrics.wallet_age_days);
                info!("  Active Days:         {}", metrics.active_days);
                info!("  Chains Active:       {}", metrics.chains_active_on.join(", "));
                
                info!("\n💰 Trading & DeFi:");
                info!("  GMX Volume:          ${}", metrics.gmx_volume_usd);
                info!("  Total Perp Volume:   ${}", metrics.total_perp_volume_usd);
                info!("  Leveraged Positions: {}", metrics.leveraged_positions_count);
                info!("  DeFi Protocols Used: {}", metrics.defi_protocols_used);
                info!("  Distinct Tokens:     {}", metrics.distinct_tokens_traded);
                info!("  Jupiter Swaps:       {}", metrics.jupiter_swaps);
                
                if !metrics.protocol_interaction_counts.is_empty() {
                    info!("\n🔧 Protocol Usage:");
                    let mut protocols: Vec<_> = metrics.protocol_interaction_counts.iter().collect();
                    protocols.sort_by(|a, b| b.1.cmp(a.1));
                    for (protocol, count) in protocols.iter().take(10) {
                        info!("  {}: {} interactions", protocol, count);
                    }
                }
                
                info!("\n🎲 Gambling:");
                info!("  Casinos Used:        {}", metrics.casinos_used);
                info!("  Casino Tokens Held:  {}", metrics.casino_tokens_held.len());
                if !metrics.casino_tokens_held.is_empty() {
                    for (token, balance) in &metrics.casino_tokens_held {
                        info!("    - {}: {}", token, balance);
                    }
                }
                
                info!("\n🖼️  NFT Activity:");
                info!("  NFT Count:           {}", metrics.nft_count);
                info!("  NFT Total Value:     ${}", metrics.nft_total_value_usd);
                info!("  NFT Trades:          {}", metrics.nft_trades);
                
                info!("\n🌉 Cross-chain:");
                info!("  Bridges Used:        {}", metrics.bridges_used);
                
                info!("\n💎 Other:");
                info!("  Stablecoin %:        {:.1}%", metrics.stablecoin_percentage * 100.0);
                info!("  Total Balance (USD): ${}", metrics.total_balance_usd);
                
                info!("\n⏰ Calculated at: {}", score.calculated_at);
            }
            Err(e) => {
                error!("Failed to calculate score for {}: {}", ens_name, e);
            }
        }
    }
    
    Ok(())
}