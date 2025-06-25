# Degen Score TUI

A Terminal User Interface (TUI) for inputting Ethereum and Solana addresses to calculate Degen Scores.

## Features

- Interactive address input with chain selection
- Support for Ethereum, Arbitrum, Optimism, Blast, and Solana
- Visual list of added addresses
- Keyboard navigation
- Direct integration with the Degen Score calculator

## Running the TUI

### Standalone Demo

The `tui_demo.rs` file is a self-contained TUI demo that can be run with Rust's script feature:

```bash
# Make sure you have Rust nightly installed
rustup install nightly

# Run the TUI demo
cargo +nightly -Zscript tui_demo.rs
```

### Using the TUI

1. **Start the TUI**: Run the command above
2. **Add addresses**: 
   - Press `a` to enter address input mode
   - Type or paste an address
   - Press `Tab` to cycle through chains (Ethereum, Arbitrum, Optimism, Blast, Solana)
   - Press `Enter` to add the address
   - Press `ESC` to cancel
3. **Navigate addresses**:
   - Use `↑`/`↓` arrow keys to select addresses
   - Press `Delete` to remove selected address
4. **Calculate scores**:
   - Press `Enter` when you have added all addresses
   - The TUI will exit and show the command to calculate scores
5. **Exit**: Press `q` to quit at any time

## Example Usage

```
┌─────────────────────────────────────────────────────────┐
│          🎰 Degen Score Calculator - Address Input       │
└─────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────┐
│ Addresses (↑/↓ to select, Delete to remove)             │
│ ethereum: 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045    │
│ solana: ApDjkHpiw2zgkXD3XupKnPnJddht4HTjfRgeRaopH3ME   │
│                                                          │
└─────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────┐
│ Add Address - Chain: ethereum (Tab to change) [a to add]│
│                                                          │
│                                                          │
│                                                          │
└─────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────┐
│ Commands: a add address | Enter calculate score | q quit│
└─────────────────────────────────────────────────────────┘
```

## Integration with Degen Score

After collecting addresses, the TUI will output a command like:

```bash
cargo run -- score --user-id demo --eth-address 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045 --sol-address ApDjkHpiw2zgkXD3XupKnPnJddht4HTjfRgeRaopH3ME
```

This command will:
1. Fetch on-chain data from all specified addresses
2. Calculate the Degen Score based on trading activity, DeFi usage, NFT holdings, etc.
3. Display the score breakdown and airdrop eligibility

## Technical Details

The TUI is built with:
- **ratatui**: Modern terminal UI framework
- **crossterm**: Cross-platform terminal manipulation
- **Rust**: For performance and safety

The TUI can be integrated into the main application or run as a standalone tool for address collection.