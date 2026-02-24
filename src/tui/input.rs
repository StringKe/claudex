use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{App, LogLevel};

pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    if app.search_mode {
        handle_search_input(app, key);
        return;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.selected_profile > 0 {
                app.selected_profile -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.selected_profile += 1;
            // Will be clamped during render
        }
        KeyCode::Char('/') => {
            app.search_mode = true;
            app.search_query.clear();
        }
        KeyCode::Char('t') => {
            app.add_log(LogLevel::Info, "Testing profile...".to_string());
        }
        KeyCode::Enter => {
            app.add_log(LogLevel::Info, "Launching claude...".to_string());
        }
        KeyCode::Char('a') => {
            app.add_log(LogLevel::Info, "Add profile: use CLI `claudex profile add`".to_string());
        }
        KeyCode::Char('d') => {
            app.add_log(LogLevel::Warn, "Delete: confirm with CLI `claudex profile remove`".to_string());
        }
        _ => {}
    }
}

fn handle_search_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.search_mode = false;
            app.search_query.clear();
        }
        KeyCode::Enter => {
            app.search_mode = false;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
        }
        _ => {}
    }
}
