pub mod dashboard;
pub mod input;
pub mod widgets;

use std::sync::Arc;

use anyhow::Result;
use crossterm::event::{self, Event};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::RwLock;

use crate::config::ClaudexConfig;
use crate::metrics::MetricsStore;
use crate::proxy::health::HealthMap;

pub struct App {
    pub config: Arc<RwLock<ClaudexConfig>>,
    pub metrics: MetricsStore,
    pub health_status: Arc<RwLock<HealthMap>>,
    pub selected_profile: usize,
    pub logs: Vec<LogEntry>,
    pub should_quit: bool,
    pub search_mode: bool,
    pub search_query: String,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
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
            selected_profile: 0,
            logs: Vec::new(),
            should_quit: false,
            search_mode: false,
            search_query: String::new(),
        }
    }

    pub fn add_log(&mut self, level: LogLevel, message: String) {
        self.logs.push(LogEntry {
            timestamp: chrono::Local::now(),
            level,
            message,
        });
        if self.logs.len() > 1000 {
            self.logs.drain(..500);
        }
    }
}

pub async fn run_tui(
    config: Arc<RwLock<ClaudexConfig>>,
    metrics: MetricsStore,
    health_status: Arc<RwLock<HealthMap>>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config, metrics, health_status);
    app.add_log(LogLevel::Info, "Claudex dashboard started".to_string());

    loop {
        let config = app.config.read().await;
        let health = app.health_status.read().await;
        terminal.draw(|f| {
            dashboard::render(f, &app, &config, &health);
        })?;
        drop(config);
        drop(health);

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                input::handle_key_event(&mut app, key);
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
