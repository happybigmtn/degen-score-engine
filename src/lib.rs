pub mod models;
pub mod config;
pub mod chains;
pub mod scoring;
pub mod verification;
pub mod utils;
pub mod tui;

pub use models::{DegenMetrics, UserProfile, Chain, DegenScore, DegenScoreError, Result};
pub use config::{Settings, RpcConfig};

// Re-export commonly used types
pub use rust_decimal::Decimal;