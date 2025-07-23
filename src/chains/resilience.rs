use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{warn, info, error};

use crate::models::{DegenScoreError, Result};

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Failing, don't try
    HalfOpen,  // Testing if service recovered
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_duration: Duration,
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_duration: Duration::from_secs(60),
            half_open_max_calls: 3,
        }
    }
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    half_open_calls: Arc<RwLock<u32>>,
    config: CircuitBreakerConfig,
    name: String,
}

impl CircuitBreaker {
    pub fn new(name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            half_open_calls: Arc::new(RwLock::new(0)),
            config,
            name,
        }
    }
    
    /// Execute a function with circuit breaker protection
    pub async fn call<F, T, Fut>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Check if circuit is open
        if self.is_open().await {
            return Err(DegenScoreError::CircuitBreakerOpen(
                format!("Circuit breaker {} is open", self.name)
            ));
        }
        
        // If half-open, check if we can make a call
        if self.is_half_open().await {
            let mut half_open_calls = self.half_open_calls.write().unwrap();
            if *half_open_calls >= self.config.half_open_max_calls {
                return Err(DegenScoreError::CircuitBreakerOpen(
                    format!("Circuit breaker {} half-open limit reached", self.name)
                ));
            }
            *half_open_calls += 1;
        }
        
        // Execute the function
        match f().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }
    
    async fn is_open(&self) -> bool {
        let state = self.state.read().unwrap();
        *state == CircuitState::Open
    }
    
    async fn is_half_open(&self) -> bool {
        let (current_state, should_transition) = {
            let state = self.state.read().unwrap();
            let should_transition = if *state == CircuitState::Open {
                // Check if timeout has passed
                if let Some(last_failure) = *self.last_failure_time.read().unwrap() {
                    last_failure.elapsed() >= self.config.timeout_duration
                } else {
                    false
                }
            } else {
                false
            };
            (state.clone(), should_transition)
        };
        
        if should_transition {
            self.transition_to_half_open().await;
            return true;
        }
        
        current_state == CircuitState::HalfOpen
    }
    
    async fn on_success(&self) {
        let current_state = {
            let state = self.state.read().unwrap();
            state.clone()
        };
        
        match current_state {
            CircuitState::Closed => {
                // Reset failure count
                *self.failure_count.write().unwrap() = 0;
            }
            CircuitState::HalfOpen => {
                let should_close = {
                    let mut success_count = self.success_count.write().unwrap();
                    *success_count += 1;
                    *success_count >= self.config.success_threshold
                };
                
                if should_close {
                    self.transition_to_closed().await;
                    info!("Circuit breaker {} transitioned to CLOSED", self.name);
                }
            }
            CircuitState::Open => {
                // Should not happen, but reset if it does
                warn!("Success recorded while circuit breaker {} is OPEN", self.name);
            }
        }
    }
    
    async fn on_failure(&self) {
        let current_state = {
            let state = self.state.read().unwrap();
            state.clone()
        };
        
        match current_state {
            CircuitState::Closed => {
                let should_open = {
                    let mut failure_count = self.failure_count.write().unwrap();
                    *failure_count += 1;
                    *failure_count >= self.config.failure_threshold
                };
                
                if should_open {
                    self.transition_to_open().await;
                    error!("Circuit breaker {} transitioned to OPEN after {} failures", 
                          self.name, self.config.failure_threshold);
                }
            }
            CircuitState::HalfOpen => {
                self.transition_to_open().await;
                warn!("Circuit breaker {} transitioned back to OPEN from HALF_OPEN", self.name);
            }
            CircuitState::Open => {
                // Already open, update last failure time
                *self.last_failure_time.write().unwrap() = Some(Instant::now());
            }
        }
    }
    
    async fn transition_to_open(&self) {
        *self.state.write().unwrap() = CircuitState::Open;
        *self.last_failure_time.write().unwrap() = Some(Instant::now());
        *self.success_count.write().unwrap() = 0;
        *self.half_open_calls.write().unwrap() = 0;
    }
    
    async fn transition_to_half_open(&self) {
        *self.state.write().unwrap() = CircuitState::HalfOpen;
        *self.success_count.write().unwrap() = 0;
        *self.half_open_calls.write().unwrap() = 0;
        info!("Circuit breaker {} transitioned to HALF_OPEN", self.name);
    }
    
    async fn transition_to_closed(&self) {
        *self.state.write().unwrap() = CircuitState::Closed;
        *self.failure_count.write().unwrap() = 0;
        *self.success_count.write().unwrap() = 0;
        *self.half_open_calls.write().unwrap() = 0;
    }
    
    /// Get current circuit breaker state for monitoring
    pub fn get_state(&self) -> CircuitState {
        self.state.read().unwrap().clone()
    }
    
    /// Get current failure count
    pub fn get_failure_count(&self) -> u32 {
        *self.failure_count.read().unwrap()
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
        }
    }
}

/// Retry with exponential backoff
pub async fn retry_with_backoff<F, T, Fut, E>(
    config: &RetryConfig,
    operation_name: &str,
    mut operation: F,
) -> std::result::Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_error = None;
    
    for attempt in 1..=config.max_attempts {
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    info!("Operation {} succeeded on attempt {}", operation_name, attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                warn!("Operation {} failed on attempt {}: {}", operation_name, attempt, e);
                last_error = Some(e);
                
                if attempt < config.max_attempts {
                    let delay = calculate_delay(config, attempt);
                    sleep(delay).await;
                }
            }
        }
    }
    
    error!("Operation {} failed after {} attempts", operation_name, config.max_attempts);
    Err(last_error.unwrap())
}

fn calculate_delay(config: &RetryConfig, attempt: u32) -> Duration {
    let delay_ms = config.base_delay.as_millis() as f64 
        * config.backoff_multiplier.powi((attempt - 1) as i32);
    
    let delay = Duration::from_millis(delay_ms as u64);
    
    if delay > config.max_delay {
        config.max_delay
    } else {
        delay
    }
}

/// Resilient RPC client wrapper
pub struct ResilientRpcClient {
    circuit_breaker: CircuitBreaker,
    retry_config: RetryConfig,
    client_name: String,
}

impl ResilientRpcClient {
    pub fn new(
        client_name: String,
        circuit_config: CircuitBreakerConfig,
        retry_config: RetryConfig,
    ) -> Self {
        let circuit_breaker = CircuitBreaker::new(
            format!("{}_circuit", client_name),
            circuit_config,
        );
        
        Self {
            circuit_breaker,
            retry_config,
            client_name,
        }
    }
    
    /// Execute an RPC call with circuit breaker and retry logic
    pub async fn call<F, T, Fut>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut + Clone,
        Fut: std::future::Future<Output = Result<T>>,
    {
        self.circuit_breaker.call(|| {
            let op = operation.clone();
            async move {
                retry_with_backoff(
                    &self.retry_config,
                    &self.client_name,
                    || op(),
                ).await
            }
        }).await
    }
    
    /// Get circuit breaker status for monitoring
    pub fn get_circuit_state(&self) -> CircuitState {
        self.circuit_breaker.get_state()
    }
    
    /// Get failure count for monitoring
    pub fn get_failure_count(&self) -> u32 {
        self.circuit_breaker.get_failure_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    
    #[tokio::test]
    async fn test_circuit_breaker_closed_to_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        
        let cb = CircuitBreaker::new("test".to_string(), config);
        
        // Should be closed initially
        assert_eq!(cb.get_state(), CircuitState::Closed);
        
        // First failure
        let result = cb.call(|| async { Err(DegenScoreError::RpcError {
            chain: "test".to_string(),
            message: "test error".to_string(),
        }) }).await;
        assert!(result.is_err());
        assert_eq!(cb.get_state(), CircuitState::Closed);
        
        // Second failure - should open circuit
        let result = cb.call(|| async { Err(DegenScoreError::RpcError {
            chain: "test".to_string(),
            message: "test error".to_string(),
        }) }).await;
        assert!(result.is_err());
        assert_eq!(cb.get_state(), CircuitState::Open);
    }
    
    #[tokio::test]
    async fn test_retry_with_backoff() {
        let config = RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(1),
            ..Default::default()
        };
        
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = retry_with_backoff(
            &config,
            "test_operation",
            || {
                let c = counter_clone.clone();
                async move {
                    let count = c.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err("Simulated failure")
                    } else {
                        Ok("Success")
                    }
                }
            },
        ).await;
        
        assert_eq!(result, Ok("Success"));
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}