use serde::{Deserialize, Serialize};
use ethers::types::{Address as EthAddress, H256, U256};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use crate::models::Chain;

// Protocol addresses and identifiers
pub struct ProtocolAddresses;

impl ProtocolAddresses {
    // GMX on Arbitrum
    pub const GMX_ROUTER: &'static str = "0xaBBc5F99639c9B6bCb58544ddf04EFA6802F4064";
    pub const GMX_VAULT: &'static str = "0x489ee077994B6658eAfA855C308275EAd8097C4A";
    pub const GMX_POSITION_ROUTER: &'static str = "0xb87a436B93fFE9D75c5cFA7bAcFff96430b09868";
    pub const GMX_REWARD_ROUTER: &'static str = "0xA906F338CB21815cBc4Bc87ace9e68c87eF8d8F1";
    
    // GMX V2 on Arbitrum
    pub const GMX_V2_ROUTER: &'static str = "0x7C68C7866A64FA2160F78EEaE12217FFbf871fa8";
    pub const GMX_V2_EXCHANGE_ROUTER: &'static str = "0x7C68C7866A64FA2160F78EEaE12217FFbf871fa8";
    
    // Jupiter on Solana
    pub const JUPITER_V4: &'static str = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB";
    pub const JUPITER_V6: &'static str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
    
    // Hyperliquid on Arbitrum
    pub const HYPERLIQUID_BRIDGE_ARB: &'static str = "0x2Df1c51E09aECF9cacB7bc98cB1742757f163dF7";
    
    // Perpetual Protocol on Optimism
    pub const PERP_CLEARING_HOUSE_OPT: &'static str = "0x82ac2CE43e33583Cd50c42a43B7b4a525F0459Bc";
    pub const PERP_VAULT_OPT: &'static str = "0xAD7b4C162707E0B2b5f6fdDbD3f8538A620C6CB4";
    
    // Casino tokens and platforms
    pub const RLB_TOKEN_ETH: &'static str = "0x046EeE2cc3188071C02BfC1745A6b17c656e3f3d";
    pub const SHFL_TOKEN_ETH: &'static str = "0x8881562783028F5c1BCB985d2283D5E170D88888";
    pub const RLB_TOKEN_ARB: &'static str = "0x1bE3735Dd0C0Eb229fB11094B6c277192349EBbf";
    
    // Rollbit Casino Contracts
    pub const ROLLBIT_LOTTERY: &'static str = "0xDa83c3BdBCD4Ec35f87d75D718556Dd60e07F201";
    pub const ROLLBIT_STAKING: &'static str = "0x6Ef13c2DbdcF8691D8d311F7E4558b5B3Eb3D3C7";
    
    // Shuffle Casino Contracts  
    pub const SHUFFLE_ROUTER: &'static str = "0xA56472f02F29B3C3b5E29F0be08Bb3639aBe86C0";
    
    // YEET Casino Token
    pub const YEET_TOKEN: &'static str = "0x89581561f1F98584F88b0d57c2180fb89225388f";
    
    // Other gambling/casino platforms
    pub const WINR_TOKEN: &'static str = "0xD77B108d4f6cefaa0Cae9506A934e825BEccA46E"; // WINR Protocol on Arbitrum
    // Note: CHIPS token address needs verification - previous address was incorrect (LP pool)
    
    // DeFi Lending Protocols
    pub const AAVE_V2_POOL_ETH: &'static str = "0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9";
    pub const AAVE_V3_POOL_ARB: &'static str = "0x794a61358D6845594F94dc1DB02A252b5b4814aD";
    pub const AAVE_V3_POOL_OPT: &'static str = "0x794a61358D6845594F94dc1DB02A252b5b4814aD";
    
    // Compound Protocol
    pub const COMPOUND_COMPTROLLER: &'static str = "0x3d9819210A31b4961b30EF54bE2aeD79B9c9Cd3B";
    pub const COMPOUND_CDAI: &'static str = "0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643";
    pub const COMPOUND_CUSDC: &'static str = "0x39AA39c021dfbaE8faC545936693aC917d5E7563";
    pub const COMPOUND_CETH: &'static str = "0x4Ddc2D193948926D02f9B1fE9e1daa0718270ED5";
    
    // Bridges
    pub const HOP_BRIDGE_ETH: &'static str = "0x3666f603Cc164936C1b87e207F36BEBa4AC5f18a";
    pub const ACROSS_BRIDGE_ETH: &'static str = "0x4D9079Bb4165aeb4084c526a32695dCfd2F77381";
    
    // Uniswap V2/V3 routers (all chains)
    pub const UNISWAP_V2_ROUTER: &'static str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
    pub const UNISWAP_V3_ROUTER: &'static str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
    pub const UNISWAP_UNIVERSAL_ROUTER: &'static str = "0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD";
    
    // Sushiswap
    pub const SUSHI_ROUTER: &'static str = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F";
    
    // Camelot (Arbitrum DEX)
    pub const CAMELOT_ROUTER: &'static str = "0xc873fEcbd354f5A56E00E710B90EF4201db2448d";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EVMTransaction {
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
    pub value: U256,
    pub gas_used: U256,
    pub gas_price: U256,
    pub timestamp: DateTime<Utc>,
    pub block_number: u64,
    pub input_data: Vec<u8>,
    pub status: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EVMTokenTransfer {
    pub token_address: String,
    pub from: String,
    pub to: String,
    pub value: U256,
    pub tx_hash: String,
    pub log_index: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaTransaction {
    pub signature: String,
    pub slot: u64,
    pub timestamp: Option<DateTime<Utc>>,
    pub fee: u64,
    pub instructions: Vec<SolanaInstruction>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaInstruction {
    pub program_id: String,
    pub accounts: Vec<String>,
    pub data: Vec<u8>,
    pub instruction_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub token_address: String,
    pub balance: U256,
    pub decimals: u8,
    pub symbol: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NFTBalance {
    pub contract_address: String,
    pub token_id: String,
    pub token_uri: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeFiPosition {
    pub protocol: String,
    pub position_type: PositionType,
    pub value_usd: Decimal,
    pub collateral_usd: Option<Decimal>,
    pub debt_usd: Option<Decimal>,
    pub apy: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionType {
    Lending,
    Borrowing,
    LiquidityProviding,
    Staking,
    Farming,
    Leveraged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GMXPosition {
    pub size_usd: Decimal,
    pub collateral_usd: Decimal,
    pub average_price: Decimal,
    pub entry_funding_rate: Decimal,
    pub reserve_amount: Decimal,
    pub realised_pnl: Decimal,
    pub is_long: bool,
    pub last_increased_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStats {
    pub chain: super::Chain,
    pub total_transactions: u64,
    pub unique_protocols_used: u32,
    pub total_gas_spent: Decimal,
    pub first_activity: Option<DateTime<Utc>>,
    pub last_activity: Option<DateTime<Utc>>,
    pub active_days: u32,
}

// Known token lists for categorization
pub struct KnownTokens;

impl KnownTokens {
    pub fn stablecoins() -> HashMap<&'static str, &'static str> {
        [
            ("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "USDC"),
            ("0xdAC17F958D2ee523a2206206994597C13D831ec7", "USDT"),
            ("0x6B175474E89094C44Da98b954EedeAC495271d0F", "DAI"),
            ("0x4Fabb145d64652a948d72533023f6E7A623C7C53", "BUSD"),
            ("0x8E870D67F660D95d5be530380D0eC0bd388289E1", "USDP"),
        ].iter().cloned().collect()
    }
    
    pub fn casino_tokens() -> HashMap<&'static str, &'static str> {
        [
            // Ethereum
            ("0x046EeE2cc3188071C02BfC1745A6b17c656e3f3d", "RLB"),
            ("0x8881562783028F5c1BCB985d2283D5E170D88888", "SHFL"),
            ("0x89581561f1F98584F88b0d57c2180fb89225388f", "YEET"),
            // Arbitrum
            ("0x1bE3735Dd0C0Eb229fB11094B6c277192349EBbf", "RLB"),
            ("0xD77B108d4f6cefaa0Cae9506A934e825BEccA46E", "WINR"),
        ].iter().cloned().collect()
    }
    
    pub fn casino_tokens_by_chain(chain: &Chain) -> HashMap<&'static str, &'static str> {
        match chain {
            Chain::Ethereum => [
                ("0x046EeE2cc3188071C02BfC1745A6b17c656e3f3d", "RLB"),
                ("0x8881562783028F5c1BCB985d2283D5E170D88888", "SHFL"),
                ("0x89581561f1F98584F88b0d57c2180fb89225388f", "YEET"),
            ].iter().cloned().collect(),
            Chain::Arbitrum => [
                ("0x1bE3735Dd0C0Eb229fB11094B6c277192349EBbf", "RLB"),
                ("0xD77B108d4f6cefaa0Cae9506A934e825BEccA46E", "WINR"),
            ].iter().cloned().collect(),
            _ => HashMap::new(),
        }
    }
    
    pub fn memecoins() -> Vec<&'static str> {
        vec![
            "PEPE", "DOGE", "SHIB", "FLOKI", "ELON", "SAFEMOON",
            "BABYDOGE", "AKITA", "KISHU", "HOKK", "FEG", "PIG",
            "WOJAK", "TURBO", "LADYS", "BOB", "PSYOP", "MONG",
            "JEFF", "BEN", "AIDOGE", "SPONGE", "WAGMI", "COPE"
        ]
    }
    
    pub fn memecoin_addresses() -> HashMap<&'static str, &'static str> {
        [
            // Ethereum memecoins
            ("0x6982508145454Ce325dDbE47a25d4ec3d2311933", "PEPE"),
            ("0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE", "SHIB"),
            ("0x761D38e5ddf6ccf6Cf7c55759d5210750B5D60F3", "ELON"),
            ("0xD29DA236dd4AAc627346e1bBa06A619E8c22d7C5", "MONG"),
            ("0x5026F006B85729a8b14553FAE6af249aD16c9aaB", "WOJAK"),
            ("0xA0b73E1Ff0B80914AB6fe0444E65848C4C34450b", "TURBO"),
            ("0x12970E6868f88f6557B76120662c1B3E50A646bf", "LADYS"),
            // Arbitrum memecoins
            ("0x09E18590E8f76b6Cf471b3cd75fE1A1a9D2B2c2b", "AIDOGE"),
        ].iter().cloned().collect()
    }
}

// Event signatures for parsing
pub struct EventSignatures;

impl EventSignatures {
    pub const ERC20_TRANSFER: &'static str = "Transfer(address,address,uint256)";
    pub const ERC721_TRANSFER: &'static str = "Transfer(address,address,uint256)";
    pub const UNISWAP_SWAP: &'static str = "Swap(address,uint256,uint256,uint256,uint256,address)";
    pub const GMX_INCREASE_POSITION: &'static str = "IncreasePosition(bytes32,address,address,address,uint256,uint256,bool,uint256,uint256)";
    pub const GMX_DECREASE_POSITION: &'static str = "DecreasePosition(bytes32,address,address,address,uint256,uint256,bool,uint256,uint256)";
    
    // Aave events
    pub const AAVE_DEPOSIT: &'static str = "Deposit(address,address,address,uint256,address,uint16)";
    pub const AAVE_BORROW: &'static str = "Borrow(address,address,address,uint256,uint256,uint256,uint16)";
    pub const AAVE_LIQUIDATION: &'static str = "LiquidationCall(address,address,address,uint256,uint256,address,bool)";
    
    // Compound events
    pub const COMPOUND_MINT: &'static str = "Mint(address,uint256,uint256)";
    pub const COMPOUND_BORROW: &'static str = "Borrow(address,uint256,uint256,uint256)";
    
    // Casino events
    pub const ROLLBIT_BET: &'static str = "BetPlaced(address,uint256,uint256)";
    pub const SHUFFLE_DEPOSIT: &'static str = "Deposit(address,uint256)";
}