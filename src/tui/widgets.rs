use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render_help_popup(f: &mut Frame) {
    let area = centered_rect(60, 70, f.area());

    let help_text = vec![
        Line::from("Claudex Dashboard Help"),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  ↑/k     Move up"),
        Line::from("  ↓/j     Move down"),
        Line::from("  Enter   Run selected profile"),
        Line::from(""),
        Line::from("Actions:"),
        Line::from("  a       Add new profile"),
        Line::from("  e       Edit selected profile"),
        Line::from("  d       Delete selected profile"),
        Line::from("  t       Test selected profile"),
        Line::from("  p       Toggle proxy"),
        Line::from("  /       Search profiles"),
        Line::from(""),
        Line::from("General:"),
        Line::from("  q/Esc   Quit"),
        Line::from("  ?       Toggle this help"),
    ];

    let paragraph = Paragraph::new(help_text).block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(paragraph, area);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let width = area.width * percent_x / 100;
    let height = area.height * percent_y / 100;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
