# Degen Scorer

A Rust-based engine for calculating "Degen Scores" - composite on-chain reputation scores for crypto users across multiple blockchains. Built for the Craps-Anchor gambling platform to fairly distribute airdrops to the most active crypto degens.

## Features

- **Multi-Chain Support**: Analyzes activity on Ethereum, Arbitrum, Optimism, Blast L2, and Solana
- **Comprehensive Metrics**: Tracks leveraged trading, gambling, DeFi usage, NFT holdings, and more
- **Secure Verification**: Proves wallet ownership without requiring private keys
- **Fair Scoring**: Weighted algorithm prevents gaming and rewards genuine activity
- **Free RPCs Only**: Uses public endpoints - no paid APIs required

## Architecture

```
src/
├── chains/          # Blockchain integrations (EVM & Solana)
├── scoring/         # Scoring algorithm and calculations  
├── verification/    # Wallet ownership verification
├── models/          # Data structures and types
└── config/          # Configuration and RPC endpoints
```

## Scoring Categories

The Degen Score is calculated across 6 weighted categories (100 points total):

1. **Trading Activity (25%)**: GMX volume, leveraged positions, Jupiter swaps
2. **Gambling Behavior (15%)**: Casino token holdings (RLB, SHFL), platform usage
3. **DeFi Engagement (15%)**: Protocol diversity, token variety, memecoin trading
4. **NFT Portfolio (10%)**: Collection count and estimated value
5. **Account Longevity (20%)**: Wallet age and activity consistency
6. **Risk Profile (15%)**: Portfolio volatility, multi-chain usage, liquidations

## Installation

```bash
# Clone the repository
git clone https://github.com/happybigmtn/degen-score-engine.git
cd degen-score-engine

# Build the project
cargo build --release

# Run tests
cargo test
```

## Usage

### Calculate a Degen Score

```bash
# Score a user with addresses on multiple chains
cargo run -- score --user-id alice \
  --eth-address 0x742d35Cc6634C0532925a3b844Bc9e7595f6e842 \
  --arb-address 0x742d35Cc6634C0532925a3b844Bc9e7595f6e842 \
  --sol-address 7VXNK6XaXQPZnqVwGHXBuCfLj9jfJzRy3aqf9PCYizv
```

Output:
```
=== Degen Score Results ===
User ID: alice
Total Score: 67.50/100
Tier: Rare

Breakdown:
  Trading: 18.75
  Gambling: 7.50
  DeFi Activity: 11.25
  NFT Portfolio: 6.00
  Longevity: 14.00
  Risk Profile: 10.00

✅ Eligible for airdrop!
```

### Verify Wallet Ownership

```bash
# Generate message for signing
cargo run -- verify-message --address 0x742d... --chain ethereum

# Verify a signature
cargo run -- verify \
  --address 0x742d35Cc6634C0532925a3b844Bc9e7595f6e842 \
  --chain ethereum \
  --signature 0x... \
  --message "I verify that I own..."
```

### Start API Server (Coming Soon)

```bash
cargo run -- serve --port 8080
```

## Configuration

Create a `config/settings.toml` file to customize:

```toml
[scoring.weights]
trading_volume = 15.0
trading_count = 10.0
gambling_platforms = 10.0
casino_tokens = 5.0
defi_protocols = 10.0
token_diversity = 5.0
nft_holdings = 10.0
wallet_age = 10.0
activity_consistency = 10.0

[scoring]
min_score_for_airdrop = 20.0
```

## RPC Endpoints

The system uses these free public RPC endpoints by default:

- **Ethereum**: https://rpc.ankr.com/eth
- **Arbitrum**: https://arbitrum-one.publicnode.com
- **Optimism**: https://optimism.publicnode.com
- **Blast**: https://rpc.blast.io
- **Solana**: https://api.mainnet-beta.solana.com

## Wallet Verification

Users must verify wallet ownership before their score is calculated:

### Method 1: Signature Verification (Recommended)
1. User signs a message with their wallet
2. Message format: "I verify that I own wallet {address} for Craps Anchor Degen Score (nonce: {nonce}). This signature does NOT grant any permissions or approvals."
3. System verifies the signature matches the address

### Method 2: Micro-Deposit (Alternative)
1. User sends 0.001 ETH/SOL to a verification address
2. System detects the transaction
3. Optional: refund minus gas fees

## Security Considerations

- **No Private Keys**: Never asks for or stores private keys
- **Read-Only**: Only reads public blockchain data
- **Safe Messages**: Signature messages explicitly state "no permissions granted"
- **Open Source**: Fully auditable scoring algorithm

## Development

### Running Tests
```bash
cargo test
```

### Building for Production
```bash
cargo build --release
```

### Database Setup
```bash
# Create SQLite database for caching
sqlx database create
sqlx migrate run
```

## API Endpoints (Coming Soon)

- `POST /verify` - Verify wallet ownership
- `GET /score/{user_id}` - Get calculated score
- `POST /score/calculate` - Trigger score calculation
- `GET /leaderboard` - View top scores

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Inspired by DegenScore and Nomis reputation systems
- Built for the Craps-Anchor gambling platform
- Uses ethers-rs and solana-client for blockchain interactions