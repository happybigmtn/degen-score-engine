use serde::{Deserialize, Serialize};
use config::{Config, ConfigError, File};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub app: AppSettings,
    pub scoring: ScoringSettings,
    pub database: DatabaseSettings,
    pub cache: CacheSettings,
    pub api: ApiSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub name: String,
    pub version: String,
    pub log_level: String,
    pub environment: Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Environment {
    Development,
    Staging,
    Production,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringSettings {
    pub weights: ScoringWeights,
    pub thresholds: ScoringThresholds,
    pub min_score_for_airdrop: f64,
    pub airdrop_pool_percentage: f64,
    pub wagering_requirement_multiplier: u32,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringThresholds {
    pub max_trading_volume_usd: f64,
    pub max_trades_count: u32,
    pub max_protocols_count: u32,
    pub max_nft_count: u32,
    pub max_wallet_age_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSettings {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    pub ttl_seconds: u64,
    pub max_entries: usize,
    pub enable_persistence: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSettings {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub max_request_size_mb: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            app: AppSettings {
                name: "Degen Scorer".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                log_level: "info".to_string(),
                environment: Environment::Development,
            },
            scoring: ScoringSettings {
                weights: ScoringWeights {
                    trading_volume: 15.0,
                    trading_count: 10.0,
                    gambling_platforms: 10.0,
                    casino_tokens: 5.0,
                    defi_protocols: 10.0,
                    token_diversity: 5.0,
                    nft_holdings: 10.0,
                    wallet_age: 10.0,
                    activity_consistency: 10.0,
                },
                thresholds: ScoringThresholds {
                    max_trading_volume_usd: 10_000_000.0,
                    max_trades_count: 100,
                    max_protocols_count: 20,
                    max_nft_count: 50,
                    max_wallet_age_days: 1825, // 5 years
                },
                min_score_for_airdrop: 20.0,
                airdrop_pool_percentage: 50.0,
                wagering_requirement_multiplier: 100,
            },
            database: DatabaseSettings {
                url: "sqlite://degen_scores.db".to_string(),
                max_connections: 10,
                min_connections: 1,
                connect_timeout_seconds: 30,
            },
            cache: CacheSettings {
                ttl_seconds: 900, // 15 minutes
                max_entries: 10000,
                enable_persistence: true,
            },
            api: ApiSettings {
                host: "0.0.0.0".to_string(),
                port: 8080,
                cors_origins: vec!["*".to_string()],
                max_request_size_mb: 10,
            },
        }
    }
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::builder()
            .add_source(Config::try_from(&Settings::default())?)
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name("config/local").required(false))
            .add_source(config::Environment::with_prefix("DEGEN_SCORE"))
            .build()?;
        
        s.try_deserialize()
    }
    
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let mut s = Config::builder()
            .add_source(Config::try_from(&Settings::default())?)
            .add_source(File::from(path.as_ref()))
            .build()?;
        
        s.try_deserialize()
    }
    
    pub fn total_weight(&self) -> f64 {
        let w = &self.scoring.weights;
        w.trading_volume + w.trading_count + w.gambling_platforms + 
        w.casino_tokens + w.defi_protocols + w.token_diversity + 
        w.nft_holdings + w.wallet_age + w.activity_consistency
    }
    
    pub fn validate(&self) -> Result<(), String> {
        let total = self.total_weight();
        if (total - 100.0).abs() > 0.01 {
            return Err(format!("Scoring weights must sum to 100, got {}", total));
        }
        
        if self.scoring.min_score_for_airdrop < 0.0 || self.scoring.min_score_for_airdrop > 100.0 {
            return Err("Minimum score for airdrop must be between 0 and 100".to_string());
        }
        
        Ok(())
    }
}