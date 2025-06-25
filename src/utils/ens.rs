use ethers::prelude::*;
use crate::models::{Result, DegenScoreError};

/// Resolve an ENS name to an Ethereum address
pub async fn resolve_ens_name(provider: &Provider<Http>, ens_name: &str) -> Result<String> {
    // Check if it's already an address
    if ens_name.starts_with("0x") && ens_name.len() == 42 {
        return Ok(ens_name.to_string());
    }
    
    // Ensure it ends with .eth
    if !ens_name.ends_with(".eth") {
        return Err(DegenScoreError::InvalidAddress(
            format!("Invalid ENS name: {}", ens_name)
        ));
    }
    
    // Resolve the ENS name
    match provider.resolve_name(ens_name).await {
        Ok(address) => Ok(format!("{:?}", address)),
        Err(e) => Err(DegenScoreError::RpcError {
            chain: "ethereum".to_string(),
            message: format!("Failed to resolve ENS name: {}", e),
        }),
    }
}