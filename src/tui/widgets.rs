use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::{FieldKind, Notification, NotificationLevel, ProfileForm};

pub fn render_help_popup(f: &mut Frame) {
    let area = centered_rect(60, 70, f.area());
    f.render_widget(Clear, area);

    let help_text = vec![
        Line::from("Claudex Dashboard Help"),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  j/k       Move down/up"),
        Line::from("  Enter     Run selected profile"),
        Line::from("  Space     Toggle Detail/Logs panel"),
        Line::from(""),
        Line::from("Actions:"),
        Line::from("  a         Add new profile"),
        Line::from("  e         Edit selected profile"),
        Line::from("  d         Delete selected profile"),
        Line::from("  t         Test selected profile"),
        Line::from("  p         Start/Stop proxy"),
        Line::from("  /         Search profiles"),
        Line::from(""),
        Line::from("Form:"),
        Line::from("  Tab/Down  Next field"),
        Line::from("  S-Tab/Up  Previous field"),
        Line::from("  Ctrl+S    Save"),
        Line::from("  Esc       Cancel"),
        Line::from(""),
        Line::from("General:"),
        Line::from("  q/Esc     Quit"),
        Line::from("  ?         Toggle this help"),
    ];

    let paragraph = Paragraph::new(help_text).block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(paragraph, area);
}

pub fn render_form_popup(f: &mut Frame, form: &ProfileForm) {
    let height = (form.fields.len() as u16 * 2) + 4; // 2 lines per field + border + title
    let area = centered_rect_fixed(50, height, f.area());
    f.render_widget(Clear, area);

    let title = if form.is_edit {
        " Edit Profile "
    } else {
        " Add Profile "
    };

    let mut lines: Vec<Line> = Vec::new();
    for (i, field) in form.fields.iter().enumerate() {
        let is_focused = i == form.focused_field;
        let label_style = if is_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let indicator = if is_focused { "> " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(indicator, label_style),
            Span::styled(format!("{}: ", field.label), label_style),
        ]));

        let value_display = match field.kind {
            FieldKind::Password => {
                if field.value.is_empty() {
                    "(empty)".to_string()
                } else {
                    "*".repeat(field.value.len())
                }
            }
            FieldKind::Bool => if field.value == "true" {
                "[x] Yes"
            } else {
                "[ ] No"
            }
            .to_string(),
            FieldKind::Select(_) => {
                format!("< {} >", field.value)
            }
            _ => {
                if is_focused {
                    let mut display = field.value.clone();
                    // Show cursor position with underscore
                    if field.cursor_pos <= display.len() {
                        display.insert(field.cursor_pos, '|');
                    }
                    display
                } else if field.value.is_empty() {
                    "(empty)".to_string()
                } else {
                    field.value.clone()
                }
            }
        };

        let value_style = if is_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };

        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(value_display, value_style),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(paragraph, area);
}

pub fn render_confirm_dialog(f: &mut Frame, target: &str) {
    let area = centered_rect_fixed(40, 5, f.area());
    f.render_widget(Clear, area);

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Delete profile '"),
            Span::styled(
                target,
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("'?"),
        ]),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .title(" Confirm Delete ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red)),
    );

    f.render_widget(paragraph, area);
}

pub fn render_notification(f: &mut Frame, notif: &Notification) {
    let color = match notif.level {
        NotificationLevel::Info => Color::Blue,
        NotificationLevel::Success => Color::Green,
        NotificationLevel::Error => Color::Red,
    };

    let width = (notif.message.len() as u16 + 4).min(f.area().width);
    let x = f.area().width.saturating_sub(width);
    let area = Rect::new(x, 0, width, 1);

    let paragraph = Paragraph::new(Line::from(vec![Span::styled(
        format!(" {} ", notif.message),
        Style::default().fg(Color::White).bg(color),
    )]));

    f.render_widget(paragraph, area);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let width = area.width * percent_x / 100;
    let height = area.height * percent_y / 100;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

pub fn centered_rect_fixed(percent_x: u16, height: u16, area: Rect) -> Rect {
    let width = area.width * percent_x / 100;
    let height = height.min(area.height);
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
