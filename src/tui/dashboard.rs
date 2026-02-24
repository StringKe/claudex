use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::config::ClaudexConfig;
use crate::proxy::health::HealthMap;

use super::{App, LogLevel};

pub fn render(f: &mut Frame, app: &App, config: &ClaudexConfig, health: &HealthMap) {
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

    render_profiles(f, app, config, health, main_chunks[0]);
    render_logs(f, app, main_chunks[1]);
    render_metrics(f, app, chunks[1]);
    render_status_bar(f, chunks[2]);
}

fn render_profiles(
    f: &mut Frame,
    app: &App,
    config: &ClaudexConfig,
    health: &HealthMap,
    area: Rect,
) {
    let items: Vec<ListItem> = config
        .profiles
        .iter()
        .enumerate()
        .map(|(i, profile)| {
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

            let style = if i == app.selected_profile {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if !profile.enabled {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            let prefix = if i == app.selected_profile {
                "►"
            } else {
                " "
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{prefix} {:<12} {indicator} {latency_str:>6}", profile.name),
                    style,
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Profiles ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(list, area);
}

fn render_logs(f: &mut Frame, app: &App, area: Rect) {
    let max_logs = area.height.saturating_sub(2) as usize;
    let start = app.logs.len().saturating_sub(max_logs);

    let items: Vec<ListItem> = app.logs[start..]
        .iter()
        .map(|log| {
            let time = log.timestamp.format("%H:%M:%S");
            let (color, prefix) = match log.level {
                LogLevel::Info => (Color::Green, "INFO"),
                LogLevel::Warn => (Color::Yellow, "WARN"),
                LogLevel::Error => (Color::Red, "ERR "),
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("[{time}] "), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{prefix} "), Style::default().fg(color)),
                Span::raw(&log.message),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Logs ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(list, area);
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
        let latencies: Vec<_> = snapshot
            .values()
            .filter_map(|m| m.avg_latency())
            .collect();
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
            format!("{:.0}%", (total_success as f64 / total_requests as f64) * 100.0)
        }
    };

    let token_display = if total_tokens > 1000 {
        format!("{:.1}K", total_tokens as f64 / 1000.0)
    } else {
        total_tokens.to_string()
    };

    let text = format!(
        " Requests: {total_requests}  |  Tokens: {token_display}  |  Avg Latency: {avg_latency}  |  Success: {success_rate}"
    );

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .title(" Metrics ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(paragraph, area);
}

fn render_status_bar(f: &mut Frame, area: Rect) {
    let text = " [Enter] Run  [e] Edit  [d] Delete  [p] Proxy  [/] Search  [a] Add  [t] Test  [q] Quit  [?] Help";
    let paragraph = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(paragraph, area);
}
