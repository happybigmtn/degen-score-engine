use degen_scorer::{
    chains::{EvmClient, ChainClient, client::ChainClientConfig},
    models::Chain,
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
    
    info!("Starting basic Degen Score test with Quicknode endpoint");
    
    let settings = Settings::default();
    
    // Create Ethereum client with very conservative settings
    let eth_config = ChainClientConfig {
        rpc_url: "https://twilight-crimson-sailboat.quiknode.pro/bffa6f76bba7ac3a3eaa237b6f0b35598e8b3981".to_string(),
        chain_id: Some(1),
        max_retries: 1, // Reduced to 1 to avoid retries
        rate_limit_per_second: 1.0, // Very conservative - 1 call per second
        timeout_seconds: 30,
    };
    
    let eth_client = Arc::new(EvmClient::new(eth_config, Chain::Ethereum).await?) as Arc<dyn ChainClient>;
    
    let test_addresses = vec![
        ("vitalik.eth", "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
        ("rng.eth", "0x52A90BfEc58cc5394A52aD53Fc83ebEF5B0119b6"),
    ];
    
    for (name, address) in test_addresses {
        info!("\\n=== Testing {} ({}) ===", name, address);
        
        // Test basic connectivity and transaction count
        match eth_client.fetch_metrics(address).await {
            Ok(metrics) => {
                info!("✅ SUCCESS for {}", name);
                info!("  Chain: {}", metrics.chain);
                info!("  Address: {}", metrics.address);
                info!("  TX Count: {}", metrics.metrics.total_tx_count);
                info!("  Wallet Age: {} days", metrics.metrics.wallet_age_days);
                info!("  Active Days: {}", metrics.metrics.active_days);
                info!("  DeFi Protocols: {}", metrics.metrics.defi_protocols_used);
                info!("  GMX Volume: ${}", metrics.metrics.gmx_volume_usd);
                info!("  Casino Tokens: {}", metrics.metrics.casino_tokens_held.len());
                info!("  NFT Count: {}", metrics.metrics.nft_count);
                info!("  Bridges Used: {}", metrics.metrics.bridges_used);
                info!("  Stablecoin %: {:.1}%", metrics.metrics.stablecoin_percentage * 100.0);
                info!("  Total Balance: ${}", metrics.metrics.total_balance_usd);
            }
            Err(e) => {
                error!("❌ FAILED for {}: {}", name, e);
            }
        }
        
        // Wait between tests to respect rate limits
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    
    Ok(())
}