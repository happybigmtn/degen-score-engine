#!/usr/bin/env cargo +nightly -Zscript
//! ```cargo
//! [dependencies]
//! ratatui = "0.26"
//! crossterm = "0.27"
//! anyhow = "1.0"
//! ```

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::{io, time::Duration};

#[derive(Debug, Clone, PartialEq)]
enum InputMode {
    Normal,
    AddingAddress,
}

#[derive(Debug, Clone)]
struct AddressEntry {
    chain: String,
    address: String,
}

struct App {
    input_mode: InputMode,
    current_input: String,
    selected_chain: String,
    addresses: Vec<AddressEntry>,
    selected_address_index: usize,
    should_quit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            input_mode: InputMode::Normal,
            current_input: String::new(),
            selected_chain: "ethereum".to_string(),
            addresses: Vec::new(),
            selected_address_index: 0,
            should_quit: false,
        }
    }
}

impl App {
    fn new() -> Self {
        Self::default()
    }

    fn add_address(&mut self) {
        if !self.current_input.trim().is_empty() {
            self.addresses.push(AddressEntry {
                chain: self.selected_chain.clone(),
                address: self.current_input.trim().to_string(),
            });
            self.current_input.clear();
            self.input_mode = InputMode::Normal;
        }
    }

    fn remove_selected_address(&mut self) {
        if !self.addresses.is_empty() && self.selected_address_index < self.addresses.len() {
            self.addresses.remove(self.selected_address_index);
            if self.selected_address_index > 0 && self.selected_address_index >= self.addresses.len() {
                self.selected_address_index -= 1;
            }
        }
    }

    fn move_selection_up(&mut self) {
        if self.selected_address_index > 0 {
            self.selected_address_index -= 1;
        }
    }

    fn move_selection_down(&mut self) {
        if self.selected_address_index < self.addresses.len().saturating_sub(1) {
            self.selected_address_index += 1;
        }
    }

    fn toggle_chain(&mut self) {
        self.selected_chain = match self.selected_chain.as_str() {
            "ethereum" => "arbitrum",
            "arbitrum" => "optimism",
            "optimism" => "blast",
            "blast" => "solana",
            "solana" => "ethereum",
            _ => "ethereum",
        }.to_string();
    }
}

fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(6),
            Constraint::Length(3),
        ])
        .split(frame.size());

    // Title
    let title = Paragraph::new("🎰 Degen Score Calculator - Address Input")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Address list
    let addresses: Vec<ListItem> = app
        .addresses
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let content = format!("{}: {} ", entry.chain, entry.address);
            let style = if i == app.selected_address_index {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let addresses_list = List::new(addresses)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Addresses (↑/↓ to select, Delete to remove)")
        );
    frame.render_widget(addresses_list, chunks[1]);

    // Input area
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            "Add Address - Chain: {} (Tab to change) {}",
            app.selected_chain,
            if app.input_mode == InputMode::AddingAddress {
                "[ESC to cancel]"
            } else {
                "[a to add]"
            }
        ));

    let input = Paragraph::new(app.current_input.as_str())
        .style(match app.input_mode {
            InputMode::AddingAddress => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })
        .block(input_block);
    frame.render_widget(input, chunks[2]);

    // Help
    let help_text = if app.input_mode == InputMode::Normal {
        vec![
            Line::from(vec![
                Span::raw("Commands: "),
                Span::styled("a", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" add address | "),
                Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" calculate score | "),
                Span::styled("q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw(" quit"),
            ]),
        ]
    } else {
        vec![Line::from("Type address and press Enter to add")]
    };

    let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[3]);

    // Show cursor when in input mode
    if app.input_mode == InputMode::AddingAddress {
        frame.set_cursor(
            chunks[2].x + app.current_input.len() as u16 + 1,
            chunks[2].y + 1,
        );
    }
}

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();

    // Clear terminal
    terminal.clear()?;

    loop {
        // Draw UI
        terminal.draw(|f| draw(f, &app))?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    InputMode::Normal => {
                        match key.code {
                            KeyCode::Char('q') => {
                                app.should_quit = true;
                            }
                            KeyCode::Char('a') => {
                                app.input_mode = InputMode::AddingAddress;
                                app.current_input.clear();
                            }
                            KeyCode::Up => {
                                app.move_selection_up();
                            }
                            KeyCode::Down => {
                                app.move_selection_down();
                            }
                            KeyCode::Delete => {
                                app.remove_selected_address();
                            }
                            KeyCode::Enter => {
                                if !app.addresses.is_empty() {
                                    // Exit and show results
                                    app.should_quit = true;
                                }
                            }
                            _ => {}
                        }
                    }
                    InputMode::AddingAddress => {
                        match key.code {
                            KeyCode::Esc => {
                                app.input_mode = InputMode::Normal;
                                app.current_input.clear();
                            }
                            KeyCode::Enter => {
                                app.add_address();
                            }
                            KeyCode::Char(c) => {
                                app.current_input.push(c);
                            }
                            KeyCode::Backspace => {
                                app.current_input.pop();
                            }
                            KeyCode::Tab => {
                                app.toggle_chain();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Print collected addresses
    println!("\nCollected addresses:");
    for entry in &app.addresses {
        println!("  {}: {}", entry.chain, entry.address);
    }
    
    println!("\nTo calculate scores, run:");
    let mut cmd = String::from("cargo run -- score --user-id demo");
    for entry in &app.addresses {
        match entry.chain.as_str() {
            "ethereum" => cmd.push_str(&format!(" --eth-address {}", entry.address)),
            "arbitrum" => cmd.push_str(&format!(" --arb-address {}", entry.address)),
            "optimism" => cmd.push_str(&format!(" --op-address {}", entry.address)),
            "blast" => cmd.push_str(&format!(" --blast-address {}", entry.address)),
            "solana" => cmd.push_str(&format!(" --sol-address {}", entry.address)),
            _ => {}
        }
    }
    println!("{}", cmd);

    Ok(())
}