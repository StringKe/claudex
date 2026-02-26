use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    App, AppMode, AsyncAction, ProfileForm, RightPanel, FIELD_BASE_URL, FIELD_ENABLED, FIELD_MODEL,
    FIELD_NAME, FIELD_PRIORITY, FIELD_PROVIDER_TYPE,
};

pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    // Help overlay dismisses on any key
    if app.show_help {
        app.show_help = false;
        return;
    }

    match app.mode {
        AppMode::Normal => handle_normal(app, key),
        AppMode::Search => handle_search(app, key),
        AppMode::AddProfile | AppMode::EditProfile => handle_form(app, key),
        AppMode::Confirm => handle_confirm(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
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
            app.mode = AppMode::Search;
            app.search_query.clear();
            app.apply_search_filter();
        }
        KeyCode::Char('?') => {
            app.show_help = true;
        }
        KeyCode::Char(' ') => {
            app.right_panel = match app.right_panel {
                RightPanel::Logs => RightPanel::Detail,
                RightPanel::Detail => RightPanel::Logs,
            };
        }
        KeyCode::Char('t') => {
            if let Some(name) = app.selected_profile_name() {
                app.pending_action = Some(AsyncAction::TestProfile(name));
            }
        }
        KeyCode::Enter => {
            if let Some(name) = app.selected_profile_name() {
                log::info!("Launching claude with profile '{name}'...");
                app.launch_profile = Some(name);
            }
        }
        KeyCode::Char('a') => {
            app.form = ProfileForm::new_blank();
            app.mode = AppMode::AddProfile;
        }
        KeyCode::Char('e') => {
            if let Some(name) = app.selected_profile_name() {
                // We need to fill form from full config. Use snapshot data for now.
                // The pending_action for EditProfile will fill from config in the event loop.
                if let Some(profile) = app.selected_profile() {
                    // Build form from snapshot (limited data, but api_key is masked anyway)
                    let mut form = ProfileForm::new_blank();
                    form.is_edit = true;
                    form.original_name = Some(name.clone());
                    form.fields[FIELD_NAME].value = profile.name.clone();
                    form.fields[FIELD_NAME].cursor_pos = profile.name.len();
                    form.fields[FIELD_PROVIDER_TYPE].value = profile.provider_type.clone();
                    form.fields[FIELD_BASE_URL].value = profile.base_url.clone();
                    form.fields[FIELD_BASE_URL].cursor_pos = profile.base_url.len();
                    // api_key left blank (cannot read from snapshot)
                    form.fields[FIELD_MODEL].value = profile.default_model.clone();
                    form.fields[FIELD_MODEL].cursor_pos = profile.default_model.len();
                    form.fields[FIELD_ENABLED].value =
                        if profile.enabled { "true" } else { "false" }.to_string();
                    form.fields[FIELD_PRIORITY].value = profile.priority.to_string();
                    form.fields[FIELD_PRIORITY].cursor_pos =
                        form.fields[FIELD_PRIORITY].value.len();
                    app.form = form;
                    app.mode = AppMode::EditProfile;
                }
            }
        }
        KeyCode::Char('d') => {
            if let Some(name) = app.selected_profile_name() {
                app.confirm_target = Some(name);
                app.mode = AppMode::Confirm;
            }
        }
        KeyCode::Char('p') => {
            if app.proxy_running {
                app.pending_action = Some(AsyncAction::StopProxy);
            } else {
                app.pending_action = Some(AsyncAction::StartProxy);
            }
        }
        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.search_query.clear();
            app.apply_search_filter();
        }
        KeyCode::Enter => {
            app.mode = AppMode::Normal;
            // Keep filter active
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.apply_search_filter();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.apply_search_filter();
        }
        _ => {}
    }
}

fn handle_form(app: &mut App, key: KeyEvent) {
    // Ctrl+S: save
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        let form = app.form.clone();
        if form.fields[FIELD_NAME].value.is_empty() {
            log::warn!("Name cannot be empty");
            return;
        }
        app.pending_action = Some(AsyncAction::SaveProfile(form));
        app.mode = AppMode::Normal;
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Tab | KeyCode::Down => {
            app.form.focus_next();
        }
        KeyCode::BackTab | KeyCode::Up => {
            app.form.focus_prev();
        }
        KeyCode::Enter => {
            // If on last field, submit
            if app.form.focused_field == app.form.fields.len() - 1 {
                let form = app.form.clone();
                if form.fields[FIELD_NAME].value.is_empty() {
                    log::warn!("Name cannot be empty");
                    return;
                }
                app.pending_action = Some(AsyncAction::SaveProfile(form));
                app.mode = AppMode::Normal;
            } else {
                app.form.focus_next();
            }
        }
        KeyCode::Left => {
            let field = &mut app.form.fields[app.form.focused_field];
            match field.kind {
                super::FieldKind::Select(_) | super::FieldKind::Bool => {
                    field.cycle_select(false);
                }
                _ => field.move_cursor_left(),
            }
        }
        KeyCode::Right => {
            let field = &mut app.form.fields[app.form.focused_field];
            match field.kind {
                super::FieldKind::Select(_) | super::FieldKind::Bool => {
                    field.cycle_select(true);
                }
                _ => field.move_cursor_right(),
            }
        }
        KeyCode::Backspace => {
            app.form.fields[app.form.focused_field].delete_char();
        }
        KeyCode::Char(' ') => {
            let field = &mut app.form.fields[app.form.focused_field];
            if field.kind == super::FieldKind::Bool {
                field.cycle_select(true);
            } else {
                field.insert_char(' ');
            }
        }
        KeyCode::Char(c) => {
            app.form.fields[app.form.focused_field].insert_char(c);
        }
        _ => {}
    }
}

fn handle_confirm(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => {
            if let Some(name) = app.confirm_target.take() {
                app.pending_action = Some(AsyncAction::DeleteProfile(name));
            }
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.confirm_target = None;
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}
