use degen_scorer::{
    chains::{EvmClient, ChainClient, client::ChainClientConfig},
    models::Chain,
    config::Settings,
};
use std::sync::Arc;
use tokio;
use tracing::{info, error, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with more detail
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    info!("Starting debug test for address: 0x52A90BfEc58cc5394A52aD53Fc83ebEF5B0119b6");
    
    let settings = Settings::default();
    
    // Try different RPC endpoints to identify the issue
    let rpc_endpoints = vec![
        ("quicknode", "https://twilight-crimson-sailboat.quiknode.pro/bffa6f76bba7ac3a3eaa237b6f0b35598e8b3981"),
        ("ethereum.publicnode.com", "https://ethereum.publicnode.com"),
        ("cloudflare-eth", "https://cloudflare-eth.com"),
    ];
    
    for (name, url) in rpc_endpoints {
        info!("Testing RPC endpoint: {} ({})", name, url);
        
        let config = ChainClientConfig {
            rpc_url: url.to_string(),
            chain_id: Some(1),
            max_retries: 1, // Reduce retries for faster debugging
            rate_limit_per_second: 5.0,
            timeout_seconds: 10,
        };
        
        match EvmClient::new(config, Chain::Ethereum).await {
            Ok(client) => {
                info!("✅ Successfully connected to {}", name);
                
                // Test basic functionality
                match client.fetch_metrics("0x52A90BfEc58cc5394A52aD53Fc83ebEF5B0119b6").await {
                    Ok(metrics) => {
                        info!("✅ Successfully fetched metrics from {}", name);
                        info!("Chain: {}", metrics.chain);
                        info!("Address: {}", metrics.address);
                        info!("TX Count: {}", metrics.metrics.total_tx_count);
                        info!("Wallet Age: {} days", metrics.metrics.wallet_age_days);
                        info!("DeFi Protocols: {}", metrics.metrics.defi_protocols_used);
                        info!("Last Updated: {}", metrics.last_updated);
                        
                        // If we get here, the system is working
                        return Ok(());
                    }
                    Err(e) => {
                        error!("❌ Failed to fetch metrics from {}: {}", name, e);
                        
                        // Print detailed error information
                        match &e {
                            degen_scorer::models::DegenScoreError::RpcError { chain, message } => {
                                error!("RPC Error on {}: {}", chain, message);
                            }
                            degen_scorer::models::DegenScoreError::InvalidAddress(addr) => {
                                error!("Invalid address: {}", addr);
                            }
                            degen_scorer::models::DegenScoreError::CircuitBreakerOpen(msg) => {
                                error!("Circuit breaker open: {}", msg);
                            }
                            _ => {
                                error!("Other error type: {:?}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("❌ Failed to connect to {}: {}", name, e);
            }
        }
    }
    
    // If we get here, all endpoints failed
    error!("🚨 All RPC endpoints failed. Issues identified:");
    error!("1. RPC Connectivity: All tested endpoints are either:");
    error!("   - Requiring API keys");
    error!("   - Rate limiting requests");
    error!("   - Returning invalid responses");
    error!("   - Timing out");
    
    error!("2. Potential solutions:");
    error!("   - Use paid RPC endpoints with API keys");
    error!("   - Implement request queuing for rate limits");
    error!("   - Add fallback to local node");
    error!("   - Use archive node for historical data");
    
    Err("All RPC endpoints failed".into())
}