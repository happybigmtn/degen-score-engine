# Degen Score Engine - LLM Implementation Guide

## Project Overview
You are implementing a Rust-based "Degen Score" calculation engine for the Craps-Anchor gambling platform. This system analyzes users' on-chain activity across multiple blockchains to assign them a degeneracy score, which determines their airdrop allocation (50% of total token supply).

## Core Objectives
1. Build a fully automated, off-chain scoring service in Rust
2. Aggregate data from Solana, Ethereum, Arbitrum, Optimism, and Blast L2
3. Use only free public RPC endpoints (no paid APIs initially)
4. Implement secure wallet verification without compromising user safety
5. Calculate composite scores based on leveraged trading, gambling, DeFi activity, NFTs, and longevity

## Architecture Guidelines

### Module Structure
```
src/
├── main.rs              # CLI/API entry point
├── lib.rs              # Library root
├── chains/
│   ├── mod.rs          # Chain module exports
│   ├── evm.rs          # EVM chain interactions
│   └── solana.rs       # Solana chain interactions
├── scoring/
│   ├── mod.rs          # Scoring module exports
│   ├── algorithm.rs    # Core scoring logic
│   └── weights.rs      # Configurable weights
├── verification/
│   ├── mod.rs          # Verification exports
│   ├── signature.rs    # Signature verification
│   └── deposit.rs      # Micro-deposit verification
├── models/
│   ├── mod.rs          # Data model exports
│   ├── metrics.rs      # DegenMetrics struct
│   ├── user.rs         # User profile & addresses
│   └── chain_data.rs   # Chain-specific data types
└── config/
    ├── mod.rs          # Config exports
    └── rpc.rs          # RPC endpoints configuration
```

### Key Implementation Details

#### 1. Data Fetching Strategy
- Use `ethers-rs` for all EVM chains (unified interface)
- Use `solana-client` for Solana RPC calls
- Implement parallel fetching with Tokio for performance
- Handle rate limits gracefully with exponential backoff
- Cache results in SQLite to avoid redundant queries

#### 2. Metric Extraction

**EVM Chains:**
- Protocol usage: Maintain hardcoded list of protocol addresses
- Transaction history: Use explorer APIs where available, fallback to getLogs
- Token balances: Direct RPC calls to token contracts
- NFT detection: Filter ERC-721 Transfer events

**Solana:**
- Use `get_signatures_for_address` for transaction history
- Parse Jupiter aggregator program interactions
- Detect SPL tokens via `get_token_accounts_by_owner`
- Identify NFTs by supply=1, decimals=0

#### 3. Scoring Algorithm
```rust
struct DegenMetrics {
    // Trading metrics
    gmx_volume_usd: f64,
    gmx_trades: u32,
    jupiter_swaps: u32,
    
    // Gambling metrics
    casinos_used: u32,
    casino_tokens_held: u32,
    
    // DeFi metrics
    defi_protocols_used: u32,
    distinct_tokens_traded: u32,
    
    // NFT metrics
    nft_count: u32,
    nft_total_value_usd: f64,
    
    // Activity metrics
    total_tx_count: u32,
    wallet_age_days: u32,
    active_days: u32,
}
```

#### 4. Verification Implementation
- **Signature**: Use EIP-191 for EVM, Ed25519 for Solana
- **Message format**: Include nonce and clear "no permissions" text
- **Storage**: Map user_id -> Vec<verified_addresses> in database

## Critical Implementation Rules

1. **Security First**
   - NEVER ask for private keys or seed phrases
   - Clearly communicate that signatures grant NO permissions
   - Validate all input addresses before processing

2. **Error Handling**
   - Use `anyhow` for application errors
   - Use `thiserror` for custom error types
   - Always provide meaningful error messages
   - Handle RPC failures gracefully

3. **Performance**
   - Batch RPC calls where possible
   - Use connection pooling for HTTP clients
   - Implement request caching (15-minute TTL)
   - Process addresses concurrently

4. **Data Accuracy**
   - Use BigDecimal for precise financial calculations
   - Store timestamps as UTC
   - Normalize token amounts by decimals
   - Handle chain-specific nuances

## RPC Endpoints Configuration

```rust
const RPC_ENDPOINTS: &[(&str, &str)] = &[
    ("ethereum", "https://rpc.ankr.com/eth"),
    ("arbitrum", "https://arbitrum-one.publicnode.com"),
    ("optimism", "https://optimism.publicnode.com"),
    ("blast", "https://blast-mainnet.public.blastapi.io"),
    ("solana", "https://api.mainnet-beta.solana.com"),
];
```

## Protocol Addresses (Examples)

```rust
// GMX on Arbitrum
const GMX_ROUTER: &str = "0xaBBc5F99639c9B6bCb58544ddf04EFA6802F4064";
const GMX_VAULT: &str = "0x489ee077994B6658eAfA855C308275EAd8097C4A";

// Jupiter on Solana
const JUPITER_V4: &str = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB";

// Casino tokens
const RLB_TOKEN: &str = "0x046EeE2cc3188071C02BfC1745A6b17c656e3f3d"; // Ethereum
const SHFL_TOKEN: &str = "0x8881562783028F5c1BCB985d2283D5E170D88888"; // Example
```

## Testing Approach

1. **Known Addresses**: Test with team wallets first
2. **Edge Cases**: Zero activity, whale accounts, multi-chain users
3. **Scoring Validation**: Ensure scores are 0-100 range
4. **Performance**: Target <5s per address full scan

## Common Pitfalls to Avoid

1. Don't assume token decimals (always fetch)
2. Handle pagination for large transaction histories
3. Account for chain-specific block times
4. Validate signatures match claimed addresses
5. Don't hardcode chain IDs in RPC calls

## Progress Tracking

Always update `/plan.md` with:
- Completed tasks
- Current blockers
- Test results
- Performance metrics

## Next Steps Priority

1. Get basic EVM data fetching working
2. Implement signature verification
3. Add Solana support
4. Build scoring algorithm
5. Create CLI interface
6. Add caching layer
7. Implement API endpoints
8. Write comprehensive tests

Remember: This is for defensive security analysis only. The goal is to create a fair, transparent system that rewards genuine crypto degens while preventing sybil attacks.