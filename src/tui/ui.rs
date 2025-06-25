use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, InputMode, Screen};

pub fn draw(frame: &mut Frame, app: &App) {
    match app.current_screen {
        Screen::Main => draw_main_screen(frame, app),
        Screen::Results => draw_results_screen(frame, app),
        Screen::Loading => draw_loading_screen(frame, app),
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
                Span::styled(&score.user_id, Style::default().fg(Color::Cyan)),
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
                    &score.tier,
                    Style::default().fg(match score.tier.as_str() {
                        "Legendary" => Color::Magenta,
                        "Diamond" => Color::Cyan,
                        "Platinum" => Color::White,
                        "Gold" => Color::Yellow,
                        "Silver" => Color::Gray,
                        "Bronze" => Color::Rgb(205, 127, 50),
                        _ => Color::Gray,
                    }).add_modifier(Modifier::BOLD)
                ),
            ]),
            Line::from(""),
            Line::from("Score Breakdown:"),
        ];

        // Add breakdown
        for (category, value) in &score.breakdown {
            text.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{}: ", category),
                    Style::default().fg(Color::White)
                ),
                Span::styled(
                    format!("{:.2}", value),
                    Style::default().fg(Color::Cyan)
                ),
            ]));
        }

        text.push(Line::from(""));
        
        // Airdrop eligibility
        if score.airdrop_eligible {
            text.push(Line::from(vec![
                Span::styled("✅ ", Style::default().fg(Color::Green)),
                Span::styled(
                    "Eligible for airdrop!",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                ),
            ]));
            if let Some(allocation) = score.airdrop_allocation {
                text.push(Line::from(vec![
                    Span::raw("Estimated allocation: "),
                    Span::styled(
                        format!("{:.2} tokens", allocation),
                        Style::default().fg(Color::Yellow)
                    ),
                ]));
            }
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