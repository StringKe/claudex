pub mod dashboard;
pub mod input;
pub mod widgets;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::ListState;
use ratatui::Terminal;
use tokio::sync::RwLock;

use crate::config::{ClaudexConfig, ProfileConfig, ProviderType};
use crate::metrics::MetricsStore;
use crate::oauth::AuthType;
use crate::proxy::health::HealthMap;

// ── State Machine ──

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Search,
    AddProfile,
    EditProfile,
    Confirm,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RightPanel {
    Logs,
    Detail,
}

#[derive(Debug, Clone)]
pub enum AsyncAction {
    SaveProfile(ProfileForm),
    DeleteProfile(String),
    StartProxy,
    StopProxy,
    TestProfile(String),
}

// ── Notification ──

#[derive(Debug, Clone)]
pub enum NotificationLevel {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    pub created_at: Instant,
}

impl Notification {
    pub fn info(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            level: NotificationLevel::Info,
            created_at: Instant::now(),
        }
    }
    pub fn success(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            level: NotificationLevel::Success,
            created_at: Instant::now(),
        }
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            level: NotificationLevel::Error,
            created_at: Instant::now(),
        }
    }
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() >= 3
    }
}

// ── Profile Snapshot ──

#[derive(Debug, Clone)]
pub struct ProfileSnapshot {
    pub name: String,
    pub enabled: bool,
    pub provider_type: String,
    pub base_url: String,
    pub default_model: String,
    pub priority: u32,
    pub auth_type: String,
    pub has_api_key: bool,
}

impl ProfileSnapshot {
    fn from_profile(p: &ProfileConfig) -> Self {
        Self {
            name: p.name.clone(),
            enabled: p.enabled,
            provider_type: match p.provider_type {
                ProviderType::DirectAnthropic => "DirectAnthropic".to_string(),
                ProviderType::OpenAICompatible => "OpenAICompatible".to_string(),
                ProviderType::OpenAIResponses => "OpenAIResponses".to_string(),
            },
            base_url: p.base_url.clone(),
            default_model: p.default_model.clone(),
            priority: p.priority,
            auth_type: match p.auth_type {
                AuthType::ApiKey => "ApiKey".to_string(),
                AuthType::OAuth => "OAuth".to_string(),
            },
            has_api_key: !p.api_key.is_empty() || p.api_key_keyring.is_some(),
        }
    }
}

// ── Form ──

#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    Text,
    Password,
    Select(Vec<String>),
    Bool,
    Number,
}

#[derive(Debug, Clone)]
pub struct FormField {
    pub label: &'static str,
    pub kind: FieldKind,
    pub value: String,
    pub cursor_pos: usize,
}

impl FormField {
    fn new(label: &'static str, kind: FieldKind, value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor_pos = value.len();
        Self {
            label,
            kind,
            value,
            cursor_pos,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        match self.kind {
            FieldKind::Number => {
                if c.is_ascii_digit() {
                    self.value.insert(self.cursor_pos, c);
                    self.cursor_pos += 1;
                }
            }
            FieldKind::Bool => {
                // toggle on any char
                self.value = if self.value == "true" {
                    "false"
                } else {
                    "true"
                }
                .to_string();
            }
            FieldKind::Select(ref options) => {
                // cycle to next
                if let Some(idx) = options.iter().position(|o| o == &self.value) {
                    let next = (idx + 1) % options.len();
                    self.value = options[next].clone();
                }
            }
            _ => {
                self.value.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
        }
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.value.remove(self.cursor_pos);
        }
    }

    pub fn move_cursor_left(&mut self) {
        self.cursor_pos = self.cursor_pos.saturating_sub(1);
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.value.len() {
            self.cursor_pos += 1;
        }
    }

    pub fn cycle_select(&mut self, forward: bool) {
        if let FieldKind::Select(ref options) = self.kind {
            if let Some(idx) = options.iter().position(|o| o == &self.value) {
                let next = if forward {
                    (idx + 1) % options.len()
                } else {
                    (idx + options.len() - 1) % options.len()
                };
                self.value = options[next].clone();
            }
        } else if self.kind == FieldKind::Bool {
            self.value = if self.value == "true" {
                "false"
            } else {
                "true"
            }
            .to_string();
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProfileForm {
    pub fields: Vec<FormField>,
    pub focused_field: usize,
    pub is_edit: bool,
    pub original_name: Option<String>,
}

impl ProfileForm {
    pub fn new_blank() -> Self {
        Self {
            fields: vec![
                FormField::new("Name", FieldKind::Text, ""),
                FormField::new(
                    "Provider Type",
                    FieldKind::Select(vec![
                        "DirectAnthropic".to_string(),
                        "OpenAICompatible".to_string(),
                        "OpenAIResponses".to_string(),
                    ]),
                    "OpenAICompatible",
                ),
                FormField::new("Base URL", FieldKind::Text, ""),
                FormField::new("API Key", FieldKind::Password, ""),
                FormField::new("Default Model", FieldKind::Text, ""),
                FormField::new("Enabled", FieldKind::Bool, "true"),
                FormField::new("Priority", FieldKind::Number, "100"),
            ],
            focused_field: 0,
            is_edit: false,
            original_name: None,
        }
    }

    pub fn from_profile(p: &ProfileConfig) -> Self {
        let provider_type = match p.provider_type {
            ProviderType::DirectAnthropic => "DirectAnthropic",
            ProviderType::OpenAICompatible => "OpenAICompatible",
            ProviderType::OpenAIResponses => "OpenAIResponses",
        };
        Self {
            fields: vec![
                FormField::new("Name", FieldKind::Text, &p.name),
                FormField::new(
                    "Provider Type",
                    FieldKind::Select(vec![
                        "DirectAnthropic".to_string(),
                        "OpenAICompatible".to_string(),
                        "OpenAIResponses".to_string(),
                    ]),
                    provider_type,
                ),
                FormField::new("Base URL", FieldKind::Text, &p.base_url),
                FormField::new("API Key", FieldKind::Password, &p.api_key),
                FormField::new("Default Model", FieldKind::Text, &p.default_model),
                FormField::new(
                    "Enabled",
                    FieldKind::Bool,
                    if p.enabled { "true" } else { "false" },
                ),
                FormField::new("Priority", FieldKind::Number, p.priority.to_string()),
            ],
            focused_field: 0,
            is_edit: true,
            original_name: Some(p.name.clone()),
        }
    }

    pub fn to_profile_config(&self) -> ProfileConfig {
        let provider_type = match self.fields[1].value.as_str() {
            "DirectAnthropic" => ProviderType::DirectAnthropic,
            "OpenAIResponses" => ProviderType::OpenAIResponses,
            _ => ProviderType::OpenAICompatible,
        };
        ProfileConfig {
            name: self.fields[0].value.clone(),
            provider_type,
            base_url: self.fields[2].value.clone(),
            api_key: self.fields[3].value.clone(),
            api_key_keyring: None,
            default_model: self.fields[4].value.clone(),
            backup_providers: Vec::new(),
            custom_headers: HashMap::new(),
            extra_env: HashMap::new(),
            priority: self.fields[6].value.parse().unwrap_or(100),
            enabled: self.fields[5].value == "true",
            auth_type: AuthType::ApiKey,
            oauth_provider: None,
            models: crate::config::ProfileModels::default(),
            max_tokens: None,
            strip_params: crate::config::StripParams::default(),
        }
    }

    pub fn focus_next(&mut self) {
        if self.focused_field < self.fields.len() - 1 {
            self.focused_field += 1;
        }
    }

    pub fn focus_prev(&mut self) {
        self.focused_field = self.focused_field.saturating_sub(1);
    }
}

// ── App ──

pub struct App {
    pub config: Arc<RwLock<ClaudexConfig>>,
    pub metrics: MetricsStore,
    pub health_status: Arc<RwLock<HealthMap>>,
    pub should_quit: bool,
    pub mode: AppMode,
    pub right_panel: RightPanel,
    pub search_query: String,
    pub proxy_running: bool,
    pub show_help: bool,
    pub launch_profile: Option<String>,
    pub notification: Option<Notification>,
    pub pending_action: Option<AsyncAction>,
    pub confirm_target: Option<String>,
    pub form: ProfileForm,

    /// Cached profile list for sync access
    pub profile_list: Vec<ProfileSnapshot>,
    /// Indices into profile_list filtered by search
    pub filtered_indices: Vec<usize>,
    /// ratatui ListState for profile selection (over filtered_indices)
    pub profile_state: ListState,
    /// tui-logger state
    pub log_state: tui_logger::TuiWidgetState,
}

impl App {
    pub fn new(
        config: Arc<RwLock<ClaudexConfig>>,
        metrics: MetricsStore,
        health_status: Arc<RwLock<HealthMap>>,
    ) -> Self {
        Self {
            config,
            metrics,
            health_status,
            should_quit: false,
            mode: AppMode::Normal,
            right_panel: RightPanel::Logs,
            search_query: String::new(),
            proxy_running: false,
            show_help: false,
            launch_profile: None,
            notification: None,
            pending_action: None,
            confirm_target: None,
            form: ProfileForm::new_blank(),
            profile_list: Vec::new(),
            filtered_indices: Vec::new(),
            profile_state: ListState::default(),
            log_state: tui_logger::TuiWidgetState::new(),
        }
    }

    /// Refresh the cached profile list from config
    pub async fn refresh_profiles(&mut self) {
        let config = self.config.read().await;
        self.profile_list = config
            .profiles
            .iter()
            .map(ProfileSnapshot::from_profile)
            .collect();
        drop(config);
        self.apply_search_filter();
    }

    /// Rebuild filtered_indices from search_query
    pub fn apply_search_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            self.filtered_indices = (0..self.profile_list.len()).collect();
        } else {
            self.filtered_indices = self
                .profile_list
                .iter()
                .enumerate()
                .filter(|(_, p)| p.name.to_lowercase().contains(&query))
                .map(|(i, _)| i)
                .collect();
        }
        // Clamp selection
        if self.filtered_indices.is_empty() {
            self.profile_state.select(None);
        } else {
            match self.profile_state.selected() {
                None => self.profile_state.select(Some(0)),
                Some(sel) if sel >= self.filtered_indices.len() => {
                    self.profile_state
                        .select(Some(self.filtered_indices.len() - 1));
                }
                _ => {}
            }
        }
    }

    /// Get the currently selected profile name (from filtered view)
    pub fn selected_profile_name(&self) -> Option<String> {
        let sel = self.profile_state.selected()?;
        let orig_idx = *self.filtered_indices.get(sel)?;
        self.profile_list.get(orig_idx).map(|p| p.name.clone())
    }

    /// Get the currently selected profile snapshot
    pub fn selected_profile(&self) -> Option<&ProfileSnapshot> {
        let sel = self.profile_state.selected()?;
        let orig_idx = *self.filtered_indices.get(sel)?;
        self.profile_list.get(orig_idx)
    }

    pub fn select_next(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let i = match self.profile_state.selected() {
            Some(i) => (i + 1).min(self.filtered_indices.len() - 1),
            None => 0,
        };
        self.profile_state.select(Some(i));
    }

    pub fn select_previous(&mut self) {
        let i = match self.profile_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.profile_state.select(Some(i));
    }
}

pub async fn run_tui(
    config: Arc<RwLock<ClaudexConfig>>,
    metrics: MetricsStore,
    health_status: Arc<RwLock<HealthMap>>,
) -> Result<()> {
    // Initialize tui-logger
    tui_logger::init_logger(log::LevelFilter::Info).ok();
    tui_logger::set_default_level(log::LevelFilter::Info);

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config, metrics, health_status);
    app.proxy_running = crate::daemon::is_proxy_running().unwrap_or(false);
    app.refresh_profiles().await;

    log::info!("Claudex dashboard started");
    if app.proxy_running {
        log::info!("Proxy is running");
    }

    let mut event_stream = EventStream::new();
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(250));

    loop {
        // Handle pending async actions first
        handle_async_actions(&mut app).await?;

        if app.should_quit {
            break;
        }

        // Check if we should exit TUI for launch
        if let Some(profile_name) = app.launch_profile.take() {
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;

            let config = app.config.read().await;
            if let Some(profile) = config.find_profile(&profile_name) {
                let profile = profile.clone();
                let config_snapshot = config.clone();
                drop(config);

                if !crate::daemon::is_proxy_running().unwrap_or(false) {
                    println!("Starting proxy in background...");
                    let bg_config = config_snapshot.clone();
                    tokio::spawn(async move {
                        if let Err(e) = crate::proxy::start_proxy(bg_config, None).await {
                            tracing::error!("proxy failed: {e}");
                        }
                    });
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }

                crate::launch::launch_claude(&config_snapshot, &profile, None, &[], false)?;
            }
            return Ok(());
        }

        // Render
        let config_snap = app.config.read().await.clone();
        let health_snap = app.health_status.read().await.clone();
        terminal.draw(|f| {
            dashboard::render(f, &mut app, &config_snap, &health_snap);
            // Overlay layers
            match app.mode {
                AppMode::AddProfile | AppMode::EditProfile => {
                    widgets::render_form_popup(f, &app.form);
                }
                AppMode::Confirm => {
                    if let Some(ref target) = app.confirm_target {
                        widgets::render_confirm_dialog(f, target);
                    }
                }
                _ => {}
            }
            if app.show_help {
                widgets::render_help_popup(f);
            }
            if let Some(ref notif) = app.notification {
                widgets::render_notification(f, notif);
            }
        })?;

        // Async event handling with tokio::select!
        tokio::select! {
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                        input::handle_key_event(&mut app, key);
                    }
                    Some(Ok(Event::Resize(_, _))) => {
                        // Terminal will auto-redraw on next loop iteration
                    }
                    Some(Err(_)) => {
                        app.should_quit = true;
                    }
                    _ => {}
                }
            }
            _ = tick.tick() => {
                // Periodic refresh
                app.refresh_profiles().await;
                app.proxy_running = crate::daemon::is_proxy_running().unwrap_or(false);
                // Clear expired notifications
                if let Some(ref notif) = app.notification {
                    if notif.is_expired() {
                        app.notification = None;
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

async fn handle_async_actions(app: &mut App) -> Result<()> {
    let action = match app.pending_action.take() {
        Some(a) => a,
        None => return Ok(()),
    };

    match action {
        AsyncAction::TestProfile(profile_name) => {
            let config = app.config.read().await;
            if let Some(profile) = config.find_profile(&profile_name) {
                let profile = profile.clone();
                drop(config);
                log::info!("Testing {}...", profile_name);
                match crate::profile::test_connectivity(&profile).await {
                    Ok(latency) => {
                        let msg = format!("{}: OK ({latency}ms)", profile_name);
                        log::info!("{msg}");
                        app.notification = Some(Notification::success(msg));
                    }
                    Err(e) => {
                        let msg = format!("{}: FAIL - {e}", profile_name);
                        log::error!("{msg}");
                        app.notification = Some(Notification::error(msg));
                    }
                }
            }
        }
        AsyncAction::SaveProfile(form) => {
            let profile_config = form.to_profile_config();
            let name = profile_config.name.clone();
            let mut config = app.config.write().await;

            if form.is_edit {
                // Remove original
                if let Some(orig) = &form.original_name {
                    config.profiles.retain(|p| p.name != *orig);
                }
            } else if config.find_profile(&name).is_some() {
                app.notification = Some(Notification::error(format!(
                    "Profile '{name}' already exists"
                )));
                return Ok(());
            }

            config.profiles.push(profile_config);
            if let Err(e) = config.save() {
                app.notification = Some(Notification::error(format!("Save failed: {e}")));
            } else {
                let verb = if form.is_edit { "Updated" } else { "Added" };
                app.notification = Some(Notification::success(format!("{verb} profile '{name}'")));
                log::info!("{verb} profile '{name}'");
            }
            drop(config);
            app.refresh_profiles().await;
        }
        AsyncAction::DeleteProfile(name) => {
            let mut config = app.config.write().await;
            config.profiles.retain(|p| p.name != name);
            if let Err(e) = config.save() {
                app.notification = Some(Notification::error(format!("Delete failed: {e}")));
            } else {
                app.notification = Some(Notification::success(format!("Deleted profile '{name}'")));
                log::info!("Deleted profile '{name}'");
            }
            drop(config);
            app.refresh_profiles().await;
        }
        AsyncAction::StartProxy => {
            let config = app.config.read().await.clone();
            tokio::spawn(async move {
                if let Err(e) = crate::proxy::start_proxy(config, None).await {
                    tracing::error!("proxy failed: {e}");
                }
            });
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            app.proxy_running = crate::daemon::is_proxy_running().unwrap_or(false);
            if app.proxy_running {
                app.notification = Some(Notification::success("Proxy started"));
                log::info!("Proxy started");
            } else {
                app.notification = Some(Notification::error("Proxy failed to start"));
                log::error!("Proxy failed to start");
            }
        }
        AsyncAction::StopProxy => match crate::daemon::stop_proxy() {
            Ok(()) => {
                app.proxy_running = false;
                app.notification = Some(Notification::success("Proxy stopped"));
                log::info!("Proxy stopped");
            }
            Err(e) => {
                app.notification = Some(Notification::error(format!("Stop failed: {e}")));
                log::error!("Stop proxy failed: {e}");
            }
        },
    }

    Ok(())
}
