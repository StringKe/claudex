pub mod dashboard;
pub mod input;
pub mod widgets;

use std::sync::Arc;

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

use crate::config::ClaudexConfig;
use crate::metrics::MetricsStore;
use crate::proxy::health::HealthMap;

/// Cached profile info for sync access in key handlers
#[derive(Debug, Clone)]
pub struct ProfileSnapshot {
    pub name: String,
    pub enabled: bool,
}

pub struct App {
    pub config: Arc<RwLock<ClaudexConfig>>,
    pub metrics: MetricsStore,
    pub health_status: Arc<RwLock<HealthMap>>,
    pub should_quit: bool,
    pub search_mode: bool,
    pub search_query: String,
    pub proxy_running: bool,
    pub show_help: bool,
    pub launch_profile: Option<String>,
    pub test_profile: Option<String>,

    /// Cached profile list for sync access
    pub profile_list: Vec<ProfileSnapshot>,
    /// ratatui ListState for profile selection (handles bounds + scroll)
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
            search_mode: false,
            search_query: String::new(),
            proxy_running: false,
            show_help: false,
            launch_profile: None,
            test_profile: None,
            profile_list: Vec::new(),
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
            .map(|p| ProfileSnapshot {
                name: p.name.clone(),
                enabled: p.enabled,
            })
            .collect();
        // Clamp selection
        if !self.profile_list.is_empty() {
            if self.profile_state.selected().is_none() {
                self.profile_state.select(Some(0));
            } else if let Some(sel) = self.profile_state.selected() {
                if sel >= self.profile_list.len() {
                    self.profile_state.select(Some(self.profile_list.len() - 1));
                }
            }
        }
    }

    /// Get the currently selected profile name (sync, from cache)
    pub fn selected_profile_name(&self) -> Option<String> {
        let idx = self.profile_state.selected()?;
        self.profile_list.get(idx).map(|p| p.name.clone())
    }

    pub fn select_next(&mut self) {
        if self.profile_list.is_empty() {
            return;
        }
        let i = match self.profile_state.selected() {
            Some(i) => (i + 1).min(self.profile_list.len() - 1),
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
        handle_async_actions(&mut app, &mut terminal).await?;

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

                crate::launch::launch_claude(&config_snapshot, &profile, None, &[])?;
            }
            return Ok(());
        }

        // Render — clone config snapshot to avoid borrow conflict with &mut app
        let config_snap = app.config.read().await.clone();
        let health_snap = app.health_status.read().await.clone();
        terminal.draw(|f| {
            dashboard::render(f, &mut app, &config_snap, &health_snap);
            if app.show_help {
                widgets::render_help_popup(f);
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
                // Periodic refresh (metrics update, etc.)
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

async fn handle_async_actions(
    app: &mut App,
    _terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> Result<()> {
    // Handle test profile
    if let Some(profile_name) = app.test_profile.take() {
        let config = app.config.read().await;
        if let Some(profile) = config.find_profile(&profile_name) {
            let profile = profile.clone();
            drop(config);
            log::info!("Testing {}...", profile_name);
            match crate::profile::test_connectivity(&profile).await {
                Ok(latency) => {
                    log::info!("{}: OK ({latency}ms)", profile_name);
                }
                Err(e) => {
                    log::error!("{}: FAIL - {e}", profile_name);
                }
            }
        }
    }

    Ok(())
}
