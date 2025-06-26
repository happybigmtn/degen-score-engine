use degen_scorer::{
    models::{DegenMetrics, Chain},
    scoring::ScoringAlgorithm,
    config::ScoringWeights,
};
use rust_decimal::Decimal;

#[test]
fn test_basic_scoring() {
    let weights = ScoringWeights::default();
    let algo = ScoringAlgorithm::new(weights);
    
    let mut metrics = DegenMetrics::default();
    metrics.gmx_volume_usd = Decimal::from(100000);
    metrics.gmx_trades = 10;
    metrics.jupiter_swaps = 5;
    metrics.casinos_used = 1;
    metrics.defi_protocols_used = 5;
    metrics.distinct_tokens_traded = 20;
    metrics.nft_count = 10;
    metrics.wallet_age_days = 365;
    metrics.active_days = 100;
    metrics.chains_active_on = vec!["ethereum".to_string(), "arbitrum".to_string()];
    
    let score = algo.calculate_score(&metrics);
    
    assert!(score.total_score > 0.0);
    assert!(score.total_score <= 100.0);
    assert_eq!(score.tier, degen_scorer::models::ScoreTier::Common);
}

#[test]
fn test_chain_parsing() {
    assert_eq!(Chain::from_str("ethereum"), Some(Chain::Ethereum));
    assert_eq!(Chain::from_str("arbitrum"), Some(Chain::Arbitrum));
    assert_eq!(Chain::from_str("optimism"), Some(Chain::Optimism));
    assert_eq!(Chain::from_str("blast"), Some(Chain::Blast));
    assert_eq!(Chain::from_str("solana"), Some(Chain::Solana));
    assert_eq!(Chain::from_str("invalid"), None);
}

#[test]
fn test_address_validation() {
    use degen_scorer::verification::WalletVerifier;
    
    // Valid EVM address
    assert!(WalletVerifier::validate_address_format(
        &Chain::Ethereum,
        "0x742d35Cc6634C0532925a3b844Bc9e7595f6e842"
    ).is_ok());
    
    // Valid Solana address
    assert!(WalletVerifier::validate_address_format(
        &Chain::Solana,
        "7VXNe1r6nTqVw6TKyBzt1TNSSQqPqNcEYizv8TduLWpU"
    ).is_ok());
    
    // Invalid addresses
    assert!(WalletVerifier::validate_address_format(
        &Chain::Ethereum,
        "not_an_address"
    ).is_err());
}