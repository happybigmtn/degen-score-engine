use thiserror::Error;

#[derive(Error, Debug)]
pub enum DegenScoreError {
    #[error("Invalid address format: {0}")]
    InvalidAddress(String),
    
    #[error("RPC error on {chain}: {message}")]
    RpcError { chain: String, message: String },
    
    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),
    
    #[error("Data fetch timeout for {chain}")]
    DataFetchTimeout { chain: String },
    
    #[error("Rate limit exceeded for {service}")]
    RateLimitExceeded { service: String },
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Chain not supported: {0}")]
    ChainNotSupported(String),
    
    #[error("Score calculation error: {0}")]
    ScoreCalculationError(String),
}

pub type Result<T> = std::result::Result<T, DegenScoreError>;