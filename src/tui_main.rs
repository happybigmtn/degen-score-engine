use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, sync::Arc, time::Duration};
use tokio::sync::Mutex;

use degen_scorer::{
    scoring::SimpleScoreCalculator as ScoreCalculator,
    tui::{App, Event as TuiEvent, EventHandler, ui},
};

pub async fn run_tui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and event handler
    let app = Arc::new(Mutex::new(App::new()));
    let events = EventHandler::new(250);
    
    // Create score calculator
    let calculator = Arc::new(ScoreCalculator::new().await?);

    // Clear terminal
    terminal.clear()?;

    let res = run_app(&mut terminal, app.clone(), events, calculator).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: Arc<Mutex<App>>,
    events: EventHandler,
    calculator: Arc<ScoreCalculator>,
) -> Result<()> {
    loop {
        // Check if we should quit
        {
            let app_guard = app.lock().await;
            if app_guard.should_quit {
                return Ok(());
            }
        }
        
        // Draw UI
        {
            let app = app.lock().await;
            terminal.draw(|f| ui::draw(f, &app))?;
        }

        // Handle events
        match events.next()? {
            TuiEvent::Key(key) => {
                let mut app_guard = app.lock().await;
                
                match app_guard.current_screen {
                    degen_scorer::tui::app::Screen::Main => {
                        match app_guard.input_mode {
                            degen_scorer::tui::app::InputMode::Normal => {
                                match key.code {
                                    KeyCode::Char('q') => {
                                        app_guard.should_quit = true;
                                    }
                                    KeyCode::Char('a') => {
                                        app_guard.input_mode = degen_scorer::tui::app::InputMode::AddingAddress;
                                        app_guard.current_input.clear();
                                    }
                                    KeyCode::Up => {
                                        app_guard.move_selection_up();
                                    }
                                    KeyCode::Down => {
                                        app_guard.move_selection_down();
                                    }
                                    KeyCode::Delete => {
                                        app_guard.remove_selected_address();
                                    }
                                    KeyCode::Enter => {
                                        if !app_guard.addresses.is_empty() {
                                            // Calculate score
                                            app_guard.set_loading("Fetching blockchain data...");
                                            
                                            let addresses = app_guard.get_addresses_by_chain();
                                            let user_id = app_guard.user_id.clone();
                                            let calculator = calculator.clone();
                                            
                                            // Drop the lock before spawning
                                            drop(app_guard);
                                            
                                            let app_clone = Arc::clone(&app);
                                            
                                            tokio::spawn(async move {
                                                match calculate_score(calculator, user_id, addresses).await {
                                                    Ok(score) => {
                                                        let mut app = app_clone.lock().await;
                                                        app.set_score_result(score);
                                                    }
                                                    Err(e) => {
                                                        let mut app = app_clone.lock().await;
                                                        app.set_error(&format!("Failed to calculate score: {}", e));
                                                    }
                                                }
                                            });
                                            
                                            continue;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            degen_scorer::tui::app::InputMode::AddingAddress => {
                                match key.code {
                                    KeyCode::Esc => {
                                        app_guard.input_mode = degen_scorer::tui::app::InputMode::Normal;
                                        app_guard.current_input.clear();
                                    }
                                    KeyCode::Enter => {
                                        app_guard.add_address();
                                    }
                                    KeyCode::Char(c) => {
                                        app_guard.current_input.push(c);
                                    }
                                    KeyCode::Backspace => {
                                        app_guard.current_input.pop();
                                    }
                                    KeyCode::Tab => {
                                        app_guard.toggle_chain();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    degen_scorer::tui::app::Screen::Results => {
                        match key.code {
                            KeyCode::Char('b') => {
                                app_guard.back_to_main();
                            }
                            KeyCode::Char('q') => {
                                app_guard.should_quit = true;
                            }
                            _ => {}
                        }
                    }
                    degen_scorer::tui::app::Screen::Loading => {
                        // Don't respond to keys while loading
                    }
                }
            }
            TuiEvent::Resize(_, _) => {
                // Terminal was resized, redraw will happen automatically
            }
            _ => {}
        }
    }
}

async fn calculate_score(
    calculator: Arc<ScoreCalculator>,
    user_id: String,
    addresses: std::collections::HashMap<degen_scorer::models::Chain, Vec<String>>,
) -> Result<degen_scorer::models::DegenScore> {
    let mut eth_address = None;
    let mut arb_address = None;
    let mut opt_address = None;
    let mut blast_address = None;
    let mut sol_address = None;

    for (chain, addrs) in addresses {
        if let Some(addr) = addrs.first() {
            match chain {
                degen_scorer::models::Chain::Ethereum => eth_address = Some(addr.clone()),
                degen_scorer::models::Chain::Arbitrum => arb_address = Some(addr.clone()),
                degen_scorer::models::Chain::Optimism => opt_address = Some(addr.clone()),
                degen_scorer::models::Chain::Blast => blast_address = Some(addr.clone()),
                degen_scorer::models::Chain::Solana => sol_address = Some(addr.clone()),
            }
        }
    }

    calculator.calculate_score(
        &user_id,
        eth_address,
        arb_address,
        opt_address,
        blast_address,
        sol_address,
    ).await
}