use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::models::Chain;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    pub endpoints: HashMap<String, RpcEndpoint>,
    pub rate_limits: HashMap<String, RateLimit>,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcEndpoint {
    pub url: String,
    pub chain: Chain,
    pub chain_id: Option<u64>,
    pub priority: u8, // Lower is higher priority
    pub is_public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_second: f64,
    pub burst_size: u32,
}

impl Default for RpcConfig {
    fn default() -> Self {
        let mut endpoints = HashMap::new();
        
        // Ethereum mainnet
        endpoints.insert("ethereum_primary".to_string(), RpcEndpoint {
            url: "https://ethereum.publicnode.com".to_string(),
            chain: Chain::Ethereum,
            chain_id: Some(1),
            priority: 1,
            is_public: true,
        });
        
        endpoints.insert("ethereum_backup".to_string(), RpcEndpoint {
            url: "https://1rpc.io/eth".to_string(),
            chain: Chain::Ethereum,
            chain_id: Some(1),
            priority: 2,
            is_public: true,
        });
        
        // Arbitrum One
        endpoints.insert("arbitrum_primary".to_string(), RpcEndpoint {
            url: "https://arbitrum-one.publicnode.com".to_string(),
            chain: Chain::Arbitrum,
            chain_id: Some(42161),
            priority: 1,
            is_public: true,
        });
        
        endpoints.insert("arbitrum_backup".to_string(), RpcEndpoint {
            url: "https://arb1.arbitrum.io/rpc".to_string(),
            chain: Chain::Arbitrum,
            chain_id: Some(42161),
            priority: 2,
            is_public: true,
        });
        
        // Optimism
        endpoints.insert("optimism_primary".to_string(), RpcEndpoint {
            url: "https://optimism.publicnode.com".to_string(),
            chain: Chain::Optimism,
            chain_id: Some(10),
            priority: 1,
            is_public: true,
        });
        
        endpoints.insert("optimism_backup".to_string(), RpcEndpoint {
            url: "https://mainnet.optimism.io".to_string(),
            chain: Chain::Optimism,
            chain_id: Some(10),
            priority: 2,
            is_public: true,
        });
        
        // Blast
        endpoints.insert("blast_primary".to_string(), RpcEndpoint {
            url: "https://rpc.blast.io".to_string(),
            chain: Chain::Blast,
            chain_id: Some(81457),
            priority: 1,
            is_public: true,
        });
        
        // Solana
        endpoints.insert("solana_primary".to_string(), RpcEndpoint {
            url: "https://api.mainnet-beta.solana.com".to_string(),
            chain: Chain::Solana,
            chain_id: None,
            priority: 1,
            is_public: true,
        });
        
        endpoints.insert("solana_backup".to_string(), RpcEndpoint {
            url: "https://solana-api.projectserum.com".to_string(),
            chain: Chain::Solana,
            chain_id: None,
            priority: 2,
            is_public: true,
        });
        
        // Rate limits for public endpoints
        let mut rate_limits = HashMap::new();
        rate_limits.insert("ankr".to_string(), RateLimit {
            requests_per_second: 2.0,
            burst_size: 10,
        });
        
        rate_limits.insert("infura_public".to_string(), RateLimit {
            requests_per_second: 10.0,
            burst_size: 20,
        });
        
        rate_limits.insert("publicnode".to_string(), RateLimit {
            requests_per_second: 5.0,
            burst_size: 15,
        });
        
        rate_limits.insert("solana".to_string(), RateLimit {
            requests_per_second: 10.0,
            burst_size: 20,
        });
        
        Self {
            endpoints,
            rate_limits,
            timeout_seconds: 30,
            max_retries: 3,
        }
    }
}

impl RpcConfig {
    pub fn get_endpoints_for_chain(&self, chain: &Chain) -> Vec<&RpcEndpoint> {
        let mut endpoints: Vec<&RpcEndpoint> = self.endpoints
            .values()
            .filter(|e| &e.chain == chain)
            .collect();
        
        // Sort by priority (lower number = higher priority)
        endpoints.sort_by_key(|e| e.priority);
        
        endpoints
    }
    
    pub fn get_primary_endpoint(&self, chain: &Chain) -> Option<&RpcEndpoint> {
        self.get_endpoints_for_chain(chain).first().copied()
    }
}

// Explorer API configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerConfig {
    pub etherscan: ExplorerEndpoint,
    pub arbiscan: ExplorerEndpoint,
    pub optimistic_etherscan: ExplorerEndpoint,
    pub blastscan: Option<ExplorerEndpoint>,
    pub solscan: ExplorerEndpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerEndpoint {
    pub base_url: String,
    pub api_key: Option<String>,
    pub rate_limit: RateLimit,
}

impl Default for ExplorerConfig {
    fn default() -> Self {
        Self {
            etherscan: ExplorerEndpoint {
                base_url: "https://api.etherscan.io/api".to_string(),
                api_key: None,
                rate_limit: RateLimit {
                    requests_per_second: 5.0,
                    burst_size: 5,
                },
            },
            arbiscan: ExplorerEndpoint {
                base_url: "https://api.arbiscan.io/api".to_string(),
                api_key: None,
                rate_limit: RateLimit {
                    requests_per_second: 5.0,
                    burst_size: 5,
                },
            },
            optimistic_etherscan: ExplorerEndpoint {
                base_url: "https://api-optimistic.etherscan.io/api".to_string(),
                api_key: None,
                rate_limit: RateLimit {
                    requests_per_second: 5.0,
                    burst_size: 5,
                },
            },
            blastscan: None, // Will be added when available
            solscan: ExplorerEndpoint {
                base_url: "https://public-api.solscan.io".to_string(),
                api_key: None,
                rate_limit: RateLimit {
                    requests_per_second: 5.0,
                    burst_size: 10,
                },
            },
        }
    }
}