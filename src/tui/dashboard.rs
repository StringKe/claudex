use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::config::ClaudexConfig;
use crate::proxy::health::HealthMap;

use super::{App, AppMode, RightPanel};

pub fn render(f: &mut Frame, app: &mut App, _config: &ClaudexConfig, health: &HealthMap) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(chunks[0]);

    render_profiles(f, app, health, main_chunks[0]);

    match app.right_panel {
        RightPanel::Logs => render_logs(f, app, main_chunks[1]),
        RightPanel::Detail => render_profile_detail(f, app, main_chunks[1]),
    }

    render_metrics(f, app, chunks[1]);
    render_status_bar(f, app, chunks[2]);
}

fn render_profiles(f: &mut Frame, app: &mut App, health: &HealthMap, area: Rect) {
    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &orig_idx)| {
            let profile = &app.profile_list[orig_idx];
            let health_status = health.get(&profile.name);
            let (indicator, latency_str) = match health_status {
                Some(h) if h.healthy => {
                    let lat = h
                        .latency_ms
                        .map(|l| format!("{l}ms"))
                        .unwrap_or_else(|| "--".to_string());
                    ("●", lat)
                }
                Some(_) => ("○", "ERR".to_string()),
                None => ("○", "--".to_string()),
            };

            let is_selected = app.profile_state.selected() == Some(display_idx);

            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if !profile.enabled {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            let prefix = if is_selected { ">" } else { " " };

            ListItem::new(Line::from(vec![Span::styled(
                format!("{prefix} {:<12} {indicator} {latency_str:>6}", profile.name),
                style,
            )]))
        })
        .collect();

    let title = if app.mode == AppMode::Search {
        format!(" Profiles [/{}] ", app.search_query)
    } else {
        format!(" Profiles ({}) ", app.filtered_indices.len())
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut app.profile_state);
}

fn render_profile_detail(f: &mut Frame, app: &App, area: Rect) {
    let detail = if let Some(profile) = app.selected_profile() {
        vec![
            Line::from(vec![
                Span::styled("Name:          ", Style::default().fg(Color::Cyan)),
                Span::raw(&profile.name),
            ]),
            Line::from(vec![
                Span::styled("Provider:      ", Style::default().fg(Color::Cyan)),
                Span::raw(&profile.provider_type),
            ]),
            Line::from(vec![
                Span::styled("Base URL:      ", Style::default().fg(Color::Cyan)),
                Span::raw(&profile.base_url),
            ]),
            Line::from(vec![
                Span::styled("Model:         ", Style::default().fg(Color::Cyan)),
                Span::raw(&profile.default_model),
            ]),
            Line::from(vec![
                Span::styled("Auth:          ", Style::default().fg(Color::Cyan)),
                Span::raw(&profile.auth_type),
                Span::raw(if profile.has_api_key {
                    " (key set)"
                } else {
                    " (no key)"
                }),
            ]),
            Line::from(vec![
                Span::styled("Priority:      ", Style::default().fg(Color::Cyan)),
                Span::raw(profile.priority.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Enabled:       ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    if profile.enabled { "yes" } else { "no" },
                    if profile.enabled {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Red)
                    },
                ),
            ]),
        ]
    } else {
        vec![Line::from("No profile selected")]
    };

    let paragraph = Paragraph::new(detail)
        .block(
            Block::default()
                .title(" Profile Detail ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn render_logs(f: &mut Frame, app: &App, area: Rect) {
    let log_widget = tui_logger::TuiLoggerSmartWidget::default()
        .style_error(Style::default().fg(Color::Red))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_info(Style::default().fg(Color::Green))
        .style_debug(Style::default().fg(Color::Blue))
        .style_trace(Style::default().fg(Color::DarkGray))
        .output_separator(':')
        .output_timestamp(Some("%H:%M:%S".to_string()))
        .output_level(Some(tui_logger::TuiLoggerLevelOutput::Abbreviated))
        .output_target(false)
        .output_file(false)
        .output_line(false)
        .title_log(" Logs ")
        .title_target("")
        .border_type(ratatui::widgets::BorderType::Plain)
        .style(Style::default().fg(Color::White))
        .state(&app.log_state);

    f.render_widget(log_widget, area);
}

fn render_metrics(f: &mut Frame, app: &App, area: Rect) {
    let snapshot = app.metrics.snapshot();

    let total_requests: u64 = snapshot
        .values()
        .map(|m| m.total_requests.load(std::sync::atomic::Ordering::Relaxed))
        .sum();
    let total_tokens: u64 = snapshot
        .values()
        .map(|m| m.total_tokens.load(std::sync::atomic::Ordering::Relaxed))
        .sum();

    let avg_latency = {
        let latencies: Vec<_> = snapshot.values().filter_map(|m| m.avg_latency()).collect();
        if latencies.is_empty() {
            "N/A".to_string()
        } else {
            let sum: std::time::Duration = latencies.iter().sum();
            let avg = sum / latencies.len() as u32;
            format!("{:.1}s", avg.as_secs_f64())
        }
    };

    let success_rate = {
        let total_success: u64 = snapshot
            .values()
            .map(|m| m.success_count.load(std::sync::atomic::Ordering::Relaxed))
            .sum();
        if total_requests == 0 {
            "100%".to_string()
        } else {
            format!(
                "{:.0}%",
                (total_success as f64 / total_requests as f64) * 100.0
            )
        }
    };

    let token_display = if total_tokens > 1000 {
        format!("{:.1}K", total_tokens as f64 / 1000.0)
    } else {
        total_tokens.to_string()
    };

    let proxy_status = if app.proxy_running {
        Span::styled(" Proxy: ON ", Style::default().fg(Color::Green))
    } else {
        Span::styled(" Proxy: OFF ", Style::default().fg(Color::Red))
    };

    let text = Line::from(vec![
        proxy_status,
        Span::raw(format!(
            " |  Requests: {total_requests}  |  Tokens: {token_display}  |  Avg: {avg_latency}  |  Success: {success_rate}"
        )),
    ]);

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .title(" Metrics ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(paragraph, area);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let text = match app.mode {
        AppMode::Normal => {
            " [Enter] Run  [a] Add  [e] Edit  [d] Delete  [t] Test  [p] Proxy  [/] Search  [Space] Panel  [q] Quit  [?] Help"
        }
        AppMode::Search => " Type to filter  [Enter] Confirm  [Esc] Cancel",
        AppMode::AddProfile | AppMode::EditProfile => {
            " [Tab/Down] Next  [Shift+Tab/Up] Prev  [Ctrl+S] Save  [Esc] Cancel"
        }
        AppMode::Confirm => " [y/Enter] Confirm  [n/Esc] Cancel",
    };
    let paragraph = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(paragraph, area);
}
