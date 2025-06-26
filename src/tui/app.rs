use crate::models::{Chain, DegenScore};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    AddingAddress,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Main,
    Results,
    Loading,
    Error,
}

pub struct AddressEntry {
    pub chain: Chain,
    pub address: String,
}

pub struct App {
    pub input_mode: InputMode,
    pub current_screen: Screen,
    pub current_input: String,
    pub selected_chain: Chain,
    pub addresses: Vec<AddressEntry>,
    pub selected_address_index: usize,
    pub user_id: String,
    pub score_result: Option<DegenScore>,
    pub error_message: Option<String>,
    pub loading_message: Option<String>,
    pub should_quit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            input_mode: InputMode::Normal,
            current_screen: Screen::Main,
            current_input: String::new(),
            selected_chain: Chain::Ethereum,
            addresses: Vec::new(),
            selected_address_index: 0,
            user_id: format!("user_{}", chrono::Utc::now().timestamp()),
            score_result: None,
            error_message: None,
            loading_message: None,
            should_quit: false,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_address(&mut self) {
        let address = self.current_input.trim();
        if address.is_empty() {
            self.error_message = Some("Address cannot be empty".to_string());
            return;
        }

        // Basic address validation
        if let Err(err) = self.validate_address(address, &self.selected_chain) {
            self.error_message = Some(err);
            return;
        }

        // Check for duplicates
        if self.addresses.iter().any(|entry| entry.address == address && entry.chain == self.selected_chain) {
            self.error_message = Some("Address already added for this chain".to_string());
            return;
        }

        self.addresses.push(AddressEntry {
            chain: self.selected_chain.clone(),
            address: address.to_string(),
        });
        self.current_input.clear();
        self.input_mode = InputMode::Normal;
        self.error_message = None;
    }

    fn validate_address(&self, address: &str, chain: &Chain) -> Result<(), String> {
        match chain {
            Chain::Ethereum | Chain::Arbitrum | Chain::Optimism | Chain::Blast => {
                if !address.starts_with("0x") {
                    return Err("EVM address must start with 0x".to_string());
                }
                if address.len() != 42 {
                    return Err("EVM address must be 42 characters long".to_string());
                }
                if !address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err("EVM address must contain only hex characters".to_string());
                }
            }
            Chain::Solana => {
                if address.len() < 32 || address.len() > 44 {
                    return Err("Solana address must be 32-44 characters long".to_string());
                }
                if !address.chars().all(|c| c.is_ascii_alphanumeric()) {
                    return Err("Solana address contains invalid characters".to_string());
                }
            }
        }
        Ok(())
    }

    pub fn remove_selected_address(&mut self) {
        if !self.addresses.is_empty() && self.selected_address_index < self.addresses.len() {
            self.addresses.remove(self.selected_address_index);
            if self.selected_address_index > 0 && self.selected_address_index >= self.addresses.len() {
                self.selected_address_index -= 1;
            }
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_address_index > 0 {
            self.selected_address_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.selected_address_index < self.addresses.len().saturating_sub(1) {
            self.selected_address_index += 1;
        }
    }

    pub fn toggle_chain(&mut self) {
        self.selected_chain = match self.selected_chain {
            Chain::Ethereum => Chain::Arbitrum,
            Chain::Arbitrum => Chain::Optimism,
            Chain::Optimism => Chain::Blast,
            Chain::Blast => Chain::Solana,
            Chain::Solana => Chain::Ethereum,
        };
    }

    pub fn get_addresses_by_chain(&self) -> HashMap<Chain, Vec<String>> {
        let mut map = HashMap::new();
        for entry in &self.addresses {
            map.entry(entry.chain.clone())
                .or_insert_with(Vec::new)
                .push(entry.address.clone());
        }
        map
    }

    pub fn set_loading(&mut self, message: &str) {
        self.current_screen = Screen::Loading;
        self.loading_message = Some(message.to_string());
        self.error_message = None;
    }

    pub fn set_error(&mut self, error: &str) {
        self.error_message = Some(error.to_string());
        self.loading_message = None;
        // For serious errors, show error screen; for minor ones, stay on main
        if error.contains("Failed to") || error.contains("Network") || error.contains("Connection") {
            self.current_screen = Screen::Error;
        } else {
            self.current_screen = Screen::Main;
        }
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
        self.current_screen = Screen::Main;
    }

    pub fn set_score_result(&mut self, score: DegenScore) {
        self.score_result = Some(score);
        self.current_screen = Screen::Results;
        self.loading_message = None;
        self.error_message = None;
    }

    pub fn back_to_main(&mut self) {
        self.current_screen = Screen::Main;
        self.error_message = None;
    }
}