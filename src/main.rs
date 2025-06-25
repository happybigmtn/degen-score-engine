use clap::{Parser, Subcommand};
use degen_scorer::{
    models::{UserProfile, VerifiedAddress, Chain, VerificationMethod},
    chains::{EvmClient, SolanaClient, ChainClient, client::ChainClientConfig},
    scoring::ScoreCalculator,
    config::{Settings, RpcConfig},
    verification::WalletVerifier,
    utils,
};
use std::sync::Arc;
use tracing::{info, error};
use tracing_subscriber;
use chrono::Utc;

mod tui_main;

#[derive(Parser)]
#[clap(name = "degen-scorer")]
#[clap(about = "Calculate Degen Scores for crypto users", long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Calculate score for a single user
    Score {
        /// User ID
        #[clap(short, long)]
        user_id: String,
        
        /// Ethereum address
        #[clap(long)]
        eth_address: Option<String>,
        
        /// Arbitrum address
        #[clap(long)]
        arb_address: Option<String>,
        
        /// Optimism address
        #[clap(long)]
        op_address: Option<String>,
        
        /// Solana address
        #[clap(long)]
        sol_address: Option<String>,
    },
    
    /// Verify a wallet address
    Verify {
        /// Address to verify
        #[clap(short, long)]
        address: String,
        
        /// Chain (ethereum, arbitrum, optimism, blast, solana)
        #[clap(short, long)]
        chain: String,
        
        /// Signature for verification
        #[clap(short, long)]
        signature: String,
        
        /// Message that was signed
        #[clap(short, long)]
        message: String,
    },
    
    /// Start the API server
    Serve {
        /// Port to listen on
        #[clap(short, long, default_value = "8080")]
        port: u16,
    },
    
    /// Launch interactive TUI
    Tui,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    // Load configuration
    let settings = Settings::new().unwrap_or_else(|_| {
        info!("Using default settings");
        Settings::default()
    });
    
    // Validate settings
    if let Err(e) = settings.validate() {
        error!("Invalid settings: {}", e);
        return Err(anyhow::anyhow!(e));
    }
    
    match cli.command {
        Commands::Score {
            user_id,
            eth_address,
            arb_address,
            op_address,
            sol_address,
        } => {
            info!("Calculating score for user: {}", user_id);
            
            // Create user profile with provided addresses
            let mut user = UserProfile::new(user_id.clone());
            
            // Initialize configuration first
            let rpc_config = RpcConfig::default();
            
            // Initialize an Ethereum provider for ENS resolution
            let eth_provider = if let Some(endpoint) = rpc_config.get_primary_endpoint(&Chain::Ethereum) {
                Some(ethers::providers::Provider::<ethers::providers::Http>::try_from(endpoint.url.as_str())
                    .map_err(|e| anyhow::anyhow!("Failed to create Ethereum provider: {}", e))?)
            } else {
                None
            };
            
            // Add verified addresses (in production, these would be verified first)
            if let Some(addr) = eth_address {
                // Resolve ENS if needed
                let resolved_addr = if addr.ends_with(".eth") && eth_provider.is_some() {
                    match utils::resolve_ens_name(eth_provider.as_ref().unwrap(), &addr).await {
                        Ok(resolved) => {
                            info!("Resolved {} to {}", addr, resolved);
                            resolved
                        }
                        Err(e) => {
                            error!("Failed to resolve ENS name {}: {}", addr, e);
                            return Ok(());
                        }
                    }
                } else {
                    addr
                };
                user.add_verified_address(VerifiedAddress {
                    address: resolved_addr,
                    chain: Chain::Ethereum,
                    verification_method: VerificationMethod::Signature {
                        message: "verified".to_string(),
                        signature: "placeholder".to_string(),
                    },
                    verified_at: Utc::now(),
                    nonce: "123".to_string(),
                });
            }
            
            if let Some(addr) = arb_address {
                user.add_verified_address(VerifiedAddress {
                    address: addr,
                    chain: Chain::Arbitrum,
                    verification_method: VerificationMethod::Signature {
                        message: "verified".to_string(),
                        signature: "placeholder".to_string(),
                    },
                    verified_at: Utc::now(),
                    nonce: "123".to_string(),
                });
            }
            
            if let Some(addr) = op_address {
                user.add_verified_address(VerifiedAddress {
                    address: addr,
                    chain: Chain::Optimism,
                    verification_method: VerificationMethod::Signature {
                        message: "verified".to_string(),
                        signature: "placeholder".to_string(),
                    },
                    verified_at: Utc::now(),
                    nonce: "123".to_string(),
                });
            }
            
            if let Some(addr) = sol_address {
                user.add_verified_address(VerifiedAddress {
                    address: addr,
                    chain: Chain::Solana,
                    verification_method: VerificationMethod::Signature {
                        message: "verified".to_string(),
                        signature: "placeholder".to_string(),
                    },
                    verified_at: Utc::now(),
                    nonce: "123".to_string(),
                });
            }
            
            if user.verified_addresses.is_empty() {
                error!("No addresses provided");
                return Ok(());
            }
            
            // Initialize chain clients
            let mut evm_clients: Vec<Arc<dyn ChainClient>> = Vec::new();
            
            // Create EVM clients
            for chain in &[Chain::Ethereum, Chain::Arbitrum, Chain::Optimism] {
                if let Some(endpoint) = rpc_config.get_primary_endpoint(chain) {
                    let config = ChainClientConfig {
                        rpc_url: endpoint.url.clone(),
                        chain_id: endpoint.chain_id,
                        timeout_seconds: rpc_config.timeout_seconds,
                        max_retries: rpc_config.max_retries,
                        rate_limit_per_second: 5.0,
                    };
                    
                    match EvmClient::new(config, chain.clone()).await {
                        Ok(client) => {
                            info!("Initialized {} client", chain.as_str());
                            evm_clients.push(Arc::new(client));
                        }
                        Err(e) => {
                            error!("Failed to initialize {} client: {}", chain.as_str(), e);
                        }
                    }
                }
            }
            
            // Create Solana client
            let sol_endpoint = rpc_config.get_primary_endpoint(&Chain::Solana)
                .ok_or_else(|| anyhow::anyhow!("No Solana RPC endpoint configured"))?;
            
            let sol_config = ChainClientConfig {
                rpc_url: sol_endpoint.url.clone(),
                chain_id: None,
                timeout_seconds: rpc_config.timeout_seconds,
                max_retries: rpc_config.max_retries,
                rate_limit_per_second: 10.0,
            };
            
            let solana_client = Arc::new(
                SolanaClient::new(sol_config)?
            ) as Arc<dyn ChainClient>;
            
            // Create score calculator
            let calculator = ScoreCalculator::new(
                evm_clients,
                solana_client,
                settings.clone(),
            );
            
            // Calculate score
            match calculator.calculate_user_score(&user).await {
                Ok(score) => {
                    println!("\n=== Degen Score Results ===");
                    println!("User ID: {}", user_id);
                    println!("Total Score: {:.2}/100", score.total_score);
                    println!("Tier: {:?}", score.tier);
                    println!("\nBreakdown:");
                    println!("  Trading: {:.2}", score.breakdown.trading_score);
                    println!("  Gambling: {:.2}", score.breakdown.gambling_score);
                    println!("  DeFi Activity: {:.2}", score.breakdown.defi_activity_score);
                    println!("  NFT Portfolio: {:.2}", score.breakdown.nft_portfolio_score);
                    println!("  Longevity: {:.2}", score.breakdown.longevity_score);
                    println!("  Risk Profile: {:.2}", score.breakdown.risk_profile_score);
                    
                    if calculator.is_eligible_for_airdrop(&score) {
                        println!("\n✅ Eligible for airdrop!");
                    } else {
                        println!("\n❌ Not eligible for airdrop (minimum score: {})", 
                            settings.scoring.min_score_for_airdrop);
                    }
                }
                Err(e) => {
                    error!("Failed to calculate score: {}", e);
                }
            }
        }
        
        Commands::Verify { address, chain, signature, message } => {
            let chain_enum = Chain::from_str(&chain)
                .ok_or_else(|| anyhow::anyhow!("Invalid chain: {}", chain))?;
            
            let verifier = WalletVerifier::new();
            
            // Validate address format
            if let Err(e) = WalletVerifier::validate_address_format(&chain_enum, &address) {
                error!("Invalid address format: {}", e);
                return Ok(());
            }
            
            // Create verification request
            let request = WalletVerifier::create_verification_request(
                chain_enum.clone(),
                address.clone(),
            );
            
            // Verify signature
            match verifier.verify_with_signature(request, signature).await {
                Ok(verified) => {
                    println!("\n✅ Wallet verification successful!");
                    println!("Address: {}", verified.address);
                    println!("Chain: {}", verified.chain.as_str());
                    println!("Verified at: {}", verified.verified_at);
                    println!("\nThis address can now be used for score calculation.");
                }
                Err(e) => {
                    error!("Verification failed: {}", e);
                    println!("\n❌ Wallet verification failed");
                    println!("Please ensure:");
                    println!("1. The signature matches the exact message");
                    println!("2. The address owns the private key that signed");
                    println!("3. The message was signed with the correct wallet");
                }
            }
        }
        
        Commands::Serve { port } => {
            println!("API server not yet implemented");
            println!("Would start server on port {}", port);
        }
        
        Commands::Tui => {
            tui_main::run_tui().await?;
        }
    }
    
    Ok(())
}
