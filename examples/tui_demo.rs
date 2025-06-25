use degen_scorer::tui::{App, Event as TuiEvent, EventHandler, ui};
use degen_scorer::models::score::DegenScore;
use degen_scorer::tui::app::{InputMode, Screen};

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, sync::Arc};
use tokio::sync::Mutex;
use chrono::Utc;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and event handler
    let app = Arc::new(Mutex::new(App::new()));
    let events = EventHandler::new(250);

    // Clear terminal
    terminal.clear()?;

    let res = run_app(&mut terminal, app.clone(), events).await;

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
) -> Result<()> {
    loop {
        // Draw UI
        {
            let app = app.lock().await;
            terminal.draw(|f| ui::draw(f, &app))?;
        }

        // Handle events
        match events.next()? {
            TuiEvent::Key(key) => {
                let mut app = app.lock().await;
                
                match app.current_screen {
                    Screen::Main => {
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
                                            // Simulate score calculation
                                            app.set_loading("Calculating score...");
                                            
                                            // Create a mock score
                                            let mut breakdown = HashMap::new();
                                            breakdown.insert("Trading".to_string(), 5.5);
                                            breakdown.insert("Gambling".to_string(), 2.0);
                                            breakdown.insert("DeFi Activity".to_string(), 8.0);
                                            breakdown.insert("NFT Portfolio".to_string(), 3.5);
                                            breakdown.insert("Longevity".to_string(), 6.0);
                                            breakdown.insert("Risk Profile".to_string(), 7.0);
                                            
                                            let total_score = 32.0;
                                            
                                            let score = DegenScore {
                                                user_id: app.user_id.clone(),
                                                total_score,
                                                percentile: 65.0,
                                                breakdown,
                                                calculated_at: Utc::now(),
                                                tier: DegenScore::tier_from_score(total_score).to_string(),
                                                airdrop_eligible: total_score >= 20.0,
                                                airdrop_allocation: Some(3200.0),
                                            };
                                            
                                            app.set_score_result(score);
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
                    Screen::Results => {
                        match key.code {
                            KeyCode::Char('b') => {
                                app.back_to_main();
                            }
                            KeyCode::Char('q') => {
                                app.should_quit = true;
                            }
                            _ => {}
                        }
                    }
                    Screen::Loading => {
                        // Don't respond to keys while loading
                    }
                }
                
                if app.should_quit {
                    return Ok(());
                }
            }
            TuiEvent::Resize(_, _) => {
                // Terminal was resized, redraw will happen automatically
            }
            _ => {}
        }
    }
}