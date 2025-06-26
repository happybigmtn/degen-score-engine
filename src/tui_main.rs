use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, sync::Arc};
use tokio::sync::Mutex;

use degen_scorer::{
    scoring::SimpleScoreCalculator as ScoreCalculator,
    tui::{App, Event as TuiEvent, EventHandler, ui},
};

pub async fn run_tui() -> Result<()> {
    // Disable logging to prevent screen corruption
    disable_logging_output();
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and event handler
    let app = Arc::new(Mutex::new(App::new()));
    let events = EventHandler::new(250);
    
    // Create score calculator with error handling
    let calculator = match ScoreCalculator::new().await {
        Ok(calc) => Arc::new(calc),
        Err(e) => {
            // Restore terminal before showing error
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            return Err(anyhow::anyhow!("Failed to initialize score calculator: {}", e));
        }
    };

    // Clear terminal
    terminal.clear()?;

    let res = run_app(&mut terminal, app.clone(), events, calculator).await;

    // Always restore terminal state, even if there was an error
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Return the app result
    if let Err(err) = res {
        eprintln!("Application error: {}", err);
        Err(err)
    } else {
        Ok(())
    }
}

fn disable_logging_output() {
    // Redirect tracing output to a null writer to prevent screen corruption
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    
    let null_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::sink)
        .with_filter(tracing_subscriber::filter::LevelFilter::OFF);
    
    let _ = tracing_subscriber::registry()
        .with(null_layer)
        .try_init();
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

        // Handle events with graceful error recovery
        let event_result = events.next();
        let event = match event_result {
            Ok(event) => event,
            Err(e) => {
                // If we can't get events, try to set an error message and continue
                if let Ok(mut app_guard) = app.try_lock() {
                    app_guard.set_error(&format!("Input error: {}", e));
                }
                continue;
            }
        };
        
        match event {
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
                                                        let error_msg = format_user_friendly_error(&e);
                                                        app.set_error(&error_msg);
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
                    degen_scorer::tui::app::Screen::Error => {
                        match key.code {
                            KeyCode::Enter => {
                                app_guard.clear_error();
                            }
                            KeyCode::Char('q') => {
                                app_guard.should_quit = true;
                            }
                            _ => {}
                        }
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

fn format_user_friendly_error(error: &anyhow::Error) -> String {
    let error_str = error.to_string();
    
    // Convert technical errors to user-friendly messages
    if error_str.contains("RPC error") && error_str.contains("exceed maximum block range") {
        "⚠️  RPC rate limit reached. Using available data for scoring.".to_string()
    } else if error_str.contains("RPC error") && error_str.contains("code: -32701") {
        "⚠️  Free RPC endpoint has restrictions. Some features may be limited.".to_string()
    } else if error_str.contains("Invalid address") {
        "❌ Invalid address format. Please check and try again.".to_string()
    } else if error_str.contains("Failed to connect") || error_str.contains("Connection") {
        "🌐 Network connection issue. Please check your internet and try again.".to_string()
    } else if error_str.contains("timeout") || error_str.contains("Timeout") {
        "⏱️  Request timed out. Please try again.".to_string()
    } else if error_str.len() > 100 {
        // Truncate very long error messages
        format!("❌ {}", &error_str[..97].trim_end())
    } else {
        format!("❌ {}", error_str)
    }
}