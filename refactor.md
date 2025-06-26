# Degen Score Engine Refactoring Plan

## Overview
This document tracks improvements to the Degen Score Engine based on comprehensive review of DeFi and casino platform detection capabilities.

## Current Implementation Gaps

### 1. Casino Platform Detection
- **Current**: Only checks token holdings (RLB, SHFL, YEET)
- **Issue**: Misses users who deposited but don't currently hold tokens
- **Impact**: Users who actively gambled but withdrew tokens get `casinos_used = 0`

### 2. DeFi Protocol Coverage
- **Current**: Limited to DEX routers (Uniswap, Sushi) and GMX
- **Missing**: Aave, Compound, yield aggregators, bridges
- **Impact**: Incomplete DeFi engagement scoring (15% weight)

### 3. Historical Data Limitations
- **Current**: Only recent blocks (30k-50k) due to RPC limits
- **Issue**: Misses older activity
- **Impact**: Long-term users may be underscored

## Implementation Tasks

### Phase 1: Enhanced Casino Detection ✅
- [x] Add contract interaction detection for casino platforms
- [x] Implement Rollbit contract checks (Lottery: 0xDa83c3BdBCD4Ec35f87d75D718556Dd60e07F201)
- [x] Implement Shuffle contract checks (Router: 0xA56472f02F29B3C3b5E29F0be08Bb3639aBe86C0)
- [x] Add YEET platform detection
- [x] Track distinct platforms vs just token count
- [x] Add casino platform interaction events
- [x] Created CasinoMetrics structure for tracking interactions
- [x] Implemented check_casino_interactions method
- [x] Added contract interaction detection via transaction logs

### Phase 2: Expanded DeFi Coverage ✅
- [x] Add Aave protocol detection
  - [x] Aave V2 LendingPool (Ethereum: 0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9)
  - [x] Aave V3 on Arbitrum/Optimism
  - [x] Implemented check_aave_activity with Deposit/Borrow event detection
- [x] Add Compound protocol detection
  - [x] cToken balance checks (cDAI, cUSDC, cETH)
  - [x] Compound Comptroller interaction detection
  - [x] Implemented check_compound_activity method
- [x] Add bridge usage detection
  - [x] Hyperliquid bridge deposits on Arbitrum
  - [x] Other major bridges (Hop, Across)
  - [x] Implemented check_bridge_activity method

### Phase 3: Historical Data Integration
- [ ] Implement explorer API integration (Etherscan, Arbiscan)
- [ ] Add caching layer for historical interactions
- [ ] Extend event log scanning range for key contracts
- [ ] Create interaction history database

### Phase 4: Scoring Refinements ✅
- [x] Add memecoin trading detection improvements
  - [x] Extended memecoin list (PEPE, WOJAK, TURBO, etc.)
  - [x] Added memecoin contract addresses mapping
  - [x] Implemented memecoin transfer detection in fetch_metrics
- [ ] Implement liquidation tracking
- [ ] Add risk metrics (max drawdown, loss events)
- [ ] Cross-chain token bridge detection

## Detailed Implementation

### Casino Platform Interaction Detection

```rust
// New structure for tracking casino interactions
pub struct CasinoInteraction {
    platform: String,
    interaction_type: InteractionType,
    timestamp: DateTime<Utc>,
    value: Option<Decimal>,
}

pub enum InteractionType {
    Deposit,
    Withdrawal,
    Bet,
    Stake,
    TokenTransfer,
}
```

#### Implementation Steps:
1. **Event Monitoring**: Check for specific events from casino contracts
   - Rollbit: Monitor Lottery participation events
   - Shuffle: Track router interactions
   - YEET: Detect token transfers and contract calls

2. **Direct Transaction Detection**: Scan for ETH/token transfers TO casino addresses
   - Flag any transaction to known casino contracts
   - Store interaction type and timestamp

3. **Platform Enumeration**: Maintain distinct set of used platforms
   - Not just token count but actual platform interaction count
   - Weight by recency and volume

### DeFi Protocol Expansion

#### Aave Integration:
```rust
// Aave V2 LendingPool events to monitor
const AAVE_DEPOSIT_EVENT: &str = "Deposit(address,address,address,uint256,address,uint16)";
const AAVE_BORROW_EVENT: &str = "Borrow(address,address,address,uint256,uint256,uint256,uint16)";

// Check for user as depositor or borrower
async fn check_aave_activity(user: &Address) -> Result<AaveMetrics> {
    // Query Deposit events where user is the onBehalfOf address
    // Query Borrow events where user is the borrower
    // Return interaction count and volume
}
```

#### Compound Integration:
```rust
// Check cToken balances as proxy for Compound usage
const COMPOUND_CTOKENS: &[(&str, &str)] = &[
    ("0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643", "cDAI"),
    ("0x39AA39c021dfbaE8faC545936693aC917d5E7563", "cUSDC"),
    ("0x4Ddc2D193948926D02f9B1fE9e1daa0718270ED5", "cETH"),
];

async fn check_compound_activity(user: &Address) -> Result<CompoundMetrics> {
    // Check balances of all cTokens
    // Check Mint/Redeem events from cToken contracts
    // Return usage metrics
}
```

### Bridge and Cross-Chain Detection

```rust
// Hyperliquid bridge on Arbitrum
const HYPERLIQUID_BRIDGE: &str = "0x2Df1c51E09aECF9cacB7bc98cB1742757f163dF7";

// Other bridges to track
const BRIDGE_CONTRACTS: &[(&str, &str, &str)] = &[
    ("0x3666f603Cc164936C1b87e207F36BEBa4AC5f18a", "Hop", "ethereum"),
    ("0x4D71d4bC0bF6d6b30a4D29E70d6D0B918E8F5c36", "Across", "ethereum"),
    // Add more bridges
];
```

### Memecoin Detection Enhancement

```rust
// Expanded memecoin list with contract addresses
pub fn extended_memecoin_list() -> HashMap<&'static str, &'static str> {
    [
        ("0x6982508145454Ce325dDbE47a25d4ec3d2311933", "PEPE"),
        ("0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE", "SHIB"),
        ("0x761D38e5ddf6ccf6Cf7c55759d5210750B5D60F3", "ELON"),
        // Add more memecoins with addresses
    ].iter().cloned().collect()
}

// Track memecoin trading volume and frequency
async fn analyze_memecoin_activity(transfers: &[TokenTransfer]) -> MemeMetrics {
    // Count transfers involving known memecoins
    // Calculate volume in USD equivalent
    // Track unique memecoins traded
}
```

## Testing Strategy

### Test Wallets:
1. **Heavy Casino User**: Address with RLB, SHFL holdings + contract interactions
2. **DeFi Power User**: Address with Aave, Compound, GMX usage
3. **Bridge User**: Address with multiple cross-chain transactions
4. **Memecoin Trader**: Address with high PEPE, SHIB trading volume

### Validation Metrics:
- Casino platforms detected should match known usage
- DeFi protocol count should include all major platforms
- Historical interactions should be captured even without current holdings
- Score should reflect true "degen" behavior patterns

## Progress Tracking

### Completed ✅
- [x] Document refactoring plan
- [x] Identify missing casino platform contracts
- [x] Research DeFi protocol addresses
- [x] Design enhanced detection structures
- [x] Implement casino contract interaction detection
- [x] Add Aave protocol integration
- [x] Add Compound protocol detection
- [x] Implement bridge detection logic
- [x] Enhance memecoin detection

### Successfully Tested ✅
- [x] Enhanced casino detection runs correctly
- [x] Bridge detection logic is executed
- [x] All new detection methods are integrated into fetch_metrics
- [x] Wallet age calculation working properly
- [x] Transaction counting operational
- [x] Casino token balance checking functional
- [x] Protocol interaction detection via event logs (replacing dummy logic)
- [x] Hyperliquid and Perpetual Protocol addresses integrated
- [x] CHIPS token removed from casino list (was incorrect address)
- [x] HashSet implementation prevents double-counting protocols

### In Progress 🚧
- [ ] Historical data integration via explorer APIs (needed to overcome RPC limits)
- [ ] Liquidation event tracking
- [ ] Cross-chain token bridge tracking
- [ ] Volume metrics for swaps and gambling

### Build Issues Fixed ✅
- [x] Fixed compilation errors with Filter API usage
- [x] Resolved DegenScore struct conflicts between score.rs and user.rs
- [x] Fixed ScoreTier references and scoring algorithm
- [x] Corrected TUI rendering for new DegenScore structure
- [x] Fixed Arc<Mutex<App>> handling in tui_main.rs
- [x] Successfully compiled with all refactoring improvements

### TUI Improvements Fixed ✅
- [x] Fixed logging interference with TUI display
- [x] Added graceful error handling and recovery
- [x] Implemented user-friendly error messages
- [x] Added address validation for all chains
- [x] Created dedicated error screen with recovery options
- [x] Improved terminal state management
- [x] Added duplicate address detection
- [x] Enhanced error formatting for better UX

### Future Enhancements 📋
- [ ] Real-time monitoring via websockets
- [ ] Machine learning for pattern detection
- [ ] Social signals integration (Twitter, Discord activity)
- [ ] On-chain governance participation tracking

## Notes

- Prioritize accuracy over performance initially
- Use caching to avoid repeated RPC calls
- Consider rate limits when implementing explorer API calls
- Maintain backwards compatibility with existing scoring weights
- Test thoroughly with known degen wallets before production deployment