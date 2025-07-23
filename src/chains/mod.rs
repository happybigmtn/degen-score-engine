pub mod evm;
// pub mod solana;  // Disabled due to dependency conflicts
pub mod solana_mock;
pub mod solana_rpc;
pub mod client;
pub mod resilience;

pub use client::ChainClient;
pub use evm::EvmClient;
// Use JSON-RPC client to avoid dependency conflicts
pub use solana_rpc::SolanaRpcClient as SolanaClient;
pub use resilience::{CircuitBreaker, ResilientRpcClient, RetryConfig, CircuitBreakerConfig};