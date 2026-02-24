use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::App;

pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    if app.show_help {
        app.show_help = false;
        return;
    }

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
            app.select_previous();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.select_next();
        }
        KeyCode::Char('/') => {
            app.search_mode = true;
            app.search_query.clear();
        }
        KeyCode::Char('?') => {
            app.show_help = true;
        }
        KeyCode::Char('t') => {
            if let Some(name) = app.selected_profile_name() {
                app.test_profile = Some(name);
            }
        }
        KeyCode::Enter => {
            if let Some(name) = app.selected_profile_name() {
                log::info!("Launching claude with profile '{name}'...");
                app.launch_profile = Some(name);
            }
        }
        KeyCode::Char('a') => {
            log::info!("Add profile: use CLI `claudex profile add`");
        }
        KeyCode::Char('d') => {
            log::warn!("Delete: confirm with CLI `claudex profile remove`");
        }
        KeyCode::Char('p') => {
            let status = if app.proxy_running {
                "running"
            } else {
                "stopped"
            };
            log::info!("Proxy is {status}. Use CLI to start/stop.");
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
