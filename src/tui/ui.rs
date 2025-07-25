use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, InputMode, Screen};
use crate::models::ScoreTier;

pub fn draw(frame: &mut Frame, app: &App) {
    match app.current_screen {
        Screen::Main => draw_main_screen(frame, app),
        Screen::Results => draw_results_screen(frame, app),
        Screen::Loading => draw_loading_screen(frame, app),
        Screen::Error => draw_error_screen(frame, app),
    }
}

fn draw_main_screen(frame: &mut Frame, app: &App) {
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
    let title = Paragraph::new("🎰 Degen Score Calculator")
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
            let content = format!("{}: {} ", entry.chain.as_str(), entry.address);
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
        )
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_widget(addresses_list, chunks[1]);

    // Input area
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            "Add Address - Chain: {} (Tab to change) {}",
            app.selected_chain.as_str(),
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

    // Help and status
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

    if let Some(error) = &app.error_message {
        let error_line = Line::from(vec![
            Span::styled("Error: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(error, Style::default().fg(Color::Red)),
        ]);
        let mut lines = help_text;
        lines.push(error_line);
        let help = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[3]);
    } else {
        let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[3]);
    }

    // Show cursor when in input mode
    if app.input_mode == InputMode::AddingAddress {
        frame.set_cursor(
            chunks[2].x + app.current_input.len() as u16 + 1,
            chunks[2].y + 1,
        );
    }
}

fn draw_results_screen(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(15),
            Constraint::Length(3),
        ])
        .split(frame.size());

    // Title
    let title = Paragraph::new("🎯 Degen Score Results")
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Results
    if let Some(score) = &app.score_result {
        let mut text = vec![
            Line::from(vec![
                Span::raw("User ID: "),
                Span::styled(&app.user_id, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Total Score: "),
                Span::styled(
                    format!("{:.2}/100", score.total_score),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                ),
            ]),
            Line::from(vec![
                Span::raw("Tier: "),
                Span::styled(
                    format!("{:?}", &score.tier),
                    Style::default().fg(match &score.tier {
                        ScoreTier::Legendary => Color::Magenta,
                        ScoreTier::Epic => Color::Cyan,
                        ScoreTier::Rare => Color::White,
                        ScoreTier::Uncommon => Color::Yellow,
                        ScoreTier::Common => Color::Gray,
                        ScoreTier::Novice => Color::Rgb(205, 127, 50),
                    }).add_modifier(Modifier::BOLD)
                ),
            ]),
            Line::from(""),
            Line::from("Score Breakdown:"),
        ];

        // Add breakdown
        text.push(Line::from(vec![
            Span::raw("  Trading: "),
            Span::styled(format!("{:.2}", score.breakdown.trading_score), Style::default().fg(Color::Cyan)),
        ]));
        text.push(Line::from(vec![
            Span::raw("  Gambling: "),
            Span::styled(format!("{:.2}", score.breakdown.gambling_score), Style::default().fg(Color::Cyan)),
        ]));
        text.push(Line::from(vec![
            Span::raw("  DeFi Activity: "),
            Span::styled(format!("{:.2}", score.breakdown.defi_activity_score), Style::default().fg(Color::Cyan)),
        ]));
        text.push(Line::from(vec![
            Span::raw("  NFT Portfolio: "),
            Span::styled(format!("{:.2}", score.breakdown.nft_portfolio_score), Style::default().fg(Color::Cyan)),
        ]));
        text.push(Line::from(vec![
            Span::raw("  Longevity: "),
            Span::styled(format!("{:.2}", score.breakdown.longevity_score), Style::default().fg(Color::Cyan)),
        ]));
        text.push(Line::from(vec![
            Span::raw("  Risk Profile: "),
            Span::styled(format!("{:.2}", score.breakdown.risk_profile_score), Style::default().fg(Color::Cyan)),
        ]));

        text.push(Line::from(""));
        
        // Airdrop eligibility
        let airdrop_eligible = score.total_score >= 20.0;
        if airdrop_eligible {
            text.push(Line::from(vec![
                Span::styled("✅ ", Style::default().fg(Color::Green)),
                Span::styled(
                    "Eligible for airdrop!",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                ),
            ]));
            // Calculate estimated allocation based on score
            let allocation = score.total_score * 1000.0; // Example calculation
            text.push(Line::from(vec![
                Span::raw("Estimated allocation: "),
                Span::styled(
                    format!("{:.2} tokens", allocation),
                    Style::default().fg(Color::Yellow)
                ),
            ]));
        } else {
            text.push(Line::from(vec![
                Span::styled("❌ ", Style::default().fg(Color::Red)),
                Span::styled(
                    "Not eligible for airdrop (minimum score: 20)",
                    Style::default().fg(Color::Red)
                ),
            ]));
        }

        let results = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Results"))
            .wrap(Wrap { trim: true });
        frame.render_widget(results, chunks[1]);
    }

    // Help
    let help = Paragraph::new("Press 'b' to go back | 'q' to quit")
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[2]);
}

fn draw_loading_screen(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, frame.size());
    
    let loading_text = app.loading_message.as_deref().unwrap_or("Loading...");
    
    let loading = Paragraph::new(loading_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("⏳ Processing")
        );
    
    frame.render_widget(Clear, area);
    frame.render_widget(loading, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_error_screen(frame: &mut Frame, app: &App) {
    let area = centered_rect(80, 30, frame.size());
    
    let error_message = app.error_message.as_deref().unwrap_or("Unknown error occurred");
    
    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("🚨 Error", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),
        Line::from(error_message),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" to continue or "),
            Span::styled("q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" to quit"),
        ]),
        Line::from(""),
    ];
    
    let error_popup = Paragraph::new(text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title("Error")
        );
    
    frame.render_widget(Clear, area);
    frame.render_widget(error_popup, area);
}