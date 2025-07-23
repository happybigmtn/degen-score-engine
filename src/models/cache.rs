use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::models::{DegenMetrics, ChainMetrics};

/// Cache entry with TTL
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub value: T,
    pub created_at: Instant,
    pub ttl: Duration,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            created_at: Instant::now(),
            ttl,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// In-memory cache for scoring data
#[derive(Debug)]
pub struct ScoreCache {
    // Address -> ChainMetrics cache
    metrics_cache: Arc<RwLock<HashMap<String, CacheEntry<ChainMetrics>>>>,
    
    // Address -> token balance cache (token_address -> balance)
    balance_cache: Arc<RwLock<HashMap<String, CacheEntry<HashMap<String, Decimal>>>>>,
    
    // Address -> protocol interaction cache (protocol -> interaction_count)
    protocol_cache: Arc<RwLock<HashMap<String, CacheEntry<HashMap<String, u32>>>>>,
    
    // Default TTL values
    metrics_ttl: Duration,
    balance_ttl: Duration,
    protocol_ttl: Duration,
}

impl Default for ScoreCache {
    fn default() -> Self {
        Self::new(
            Duration::from_secs(300),  // 5 minutes for metrics
            Duration::from_secs(60),   // 1 minute for balances (more volatile)
            Duration::from_secs(600),  // 10 minutes for protocol interactions
        )
    }
}

impl ScoreCache {
    pub fn new(metrics_ttl: Duration, balance_ttl: Duration, protocol_ttl: Duration) -> Self {
        Self {
            metrics_cache: Arc::new(RwLock::new(HashMap::new())),
            balance_cache: Arc::new(RwLock::new(HashMap::new())),
            protocol_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics_ttl,
            balance_ttl,
            protocol_ttl,
        }
    }
    
    /// Get cached metrics for an address
    pub fn get_metrics(&self, address: &str) -> Option<ChainMetrics> {
        let cache = self.metrics_cache.read().ok()?;
        let entry = cache.get(address)?;
        
        if entry.is_expired() {
            return None;
        }
        
        Some(entry.value.clone())
    }
    
    /// Cache metrics for an address
    pub fn set_metrics(&self, address: String, metrics: ChainMetrics) {
        if let Ok(mut cache) = self.metrics_cache.write() {
            cache.insert(address, CacheEntry::new(metrics, self.metrics_ttl));
        }
    }
    
    /// Get cached token balances for an address
    pub fn get_balances(&self, address: &str) -> Option<HashMap<String, Decimal>> {
        let cache = self.balance_cache.read().ok()?;
        let entry = cache.get(address)?;
        
        if entry.is_expired() {
            return None;
        }
        
        Some(entry.value.clone())
    }
    
    /// Cache token balances for an address
    pub fn set_balances(&self, address: String, balances: HashMap<String, Decimal>) {
        if let Ok(mut cache) = self.balance_cache.write() {
            cache.insert(address, CacheEntry::new(balances, self.balance_ttl));
        }
    }
    
    /// Get cached protocol interactions for an address
    pub fn get_protocol_interactions(&self, address: &str) -> Option<HashMap<String, u32>> {
        let cache = self.protocol_cache.read().ok()?;
        let entry = cache.get(address)?;
        
        if entry.is_expired() {
            return None;
        }
        
        Some(entry.value.clone())
    }
    
    /// Cache protocol interactions for an address
    pub fn set_protocol_interactions(&self, address: String, interactions: HashMap<String, u32>) {
        if let Ok(mut cache) = self.protocol_cache.write() {
            cache.insert(address, CacheEntry::new(interactions, self.protocol_ttl));
        }
    }
    
    /// Clean up expired entries
    pub fn cleanup_expired(&self) {
        // Cleanup metrics cache
        if let Ok(mut cache) = self.metrics_cache.write() {
            cache.retain(|_, entry| !entry.is_expired());
        }
        
        // Cleanup balance cache
        if let Ok(mut cache) = self.balance_cache.write() {
            cache.retain(|_, entry| !entry.is_expired());
        }
        
        // Cleanup protocol cache
        if let Ok(mut cache) = self.protocol_cache.write() {
            cache.retain(|_, entry| !entry.is_expired());
        }
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let metrics_count = self.metrics_cache.read().map(|c| c.len()).unwrap_or(0);
        let balance_count = self.balance_cache.read().map(|c| c.len()).unwrap_or(0);
        let protocol_count = self.protocol_cache.read().map(|c| c.len()).unwrap_or(0);
        
        CacheStats {
            metrics_entries: metrics_count,
            balance_entries: balance_count,
            protocol_entries: protocol_count,
            total_entries: metrics_count + balance_count + protocol_count,
        }
    }
    
    /// Clear all cache entries
    pub fn clear_all(&self) {
        if let Ok(mut cache) = self.metrics_cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.balance_cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.protocol_cache.write() {
            cache.clear();
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub metrics_entries: usize,
    pub balance_entries: usize,
    pub protocol_entries: usize,
    pub total_entries: usize,
}

/// Cache key builder for consistent key generation
pub struct CacheKey;

impl CacheKey {
    pub fn metrics(chain: &str, address: &str) -> String {
        format!("metrics:{}:{}", chain, address.to_lowercase())
    }
    
    pub fn balance(chain: &str, address: &str, token: &str) -> String {
        format!("balance:{}:{}:{}", chain, address.to_lowercase(), token.to_lowercase())
    }
    
    pub fn protocol(chain: &str, address: &str, protocol: &str) -> String {
        format!("protocol:{}:{}:{}", chain, address.to_lowercase(), protocol.to_lowercase())
    }
    
    pub fn token_interaction(chain: &str, address: &str, token: &str) -> String {
        format!("token_interaction:{}:{}:{}", chain, address.to_lowercase(), token.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new("test_value".to_string(), Duration::from_millis(10));
        assert!(!entry.is_expired());
        
        std::thread::sleep(Duration::from_millis(15));
        assert!(entry.is_expired());
    }
    
    #[test]
    fn test_cache_basic_operations() {
        let cache = ScoreCache::default();
        
        // Test metrics cache
        let test_metrics = ChainMetrics {
            chain: "ethereum".to_string(),
            address: "0x123".to_string(),
            metrics: DegenMetrics::default(),
            last_updated: Utc::now(),
        };
        
        cache.set_metrics("0x123".to_string(), test_metrics.clone());
        let retrieved = cache.get_metrics("0x123");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().address, "0x123");
        
        // Test non-existent key
        assert!(cache.get_metrics("0x456").is_none());
    }
    
    #[test]
    fn test_cache_key_generation() {
        assert_eq!(
            CacheKey::metrics("ethereum", "0xABC"),
            "metrics:ethereum:0xabc"
        );
        
        assert_eq!(
            CacheKey::balance("arbitrum", "0xDEF", "0x123"),
            "balance:arbitrum:0xdef:0x123"
        );
    }
}