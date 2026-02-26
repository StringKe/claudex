#![allow(dead_code)]

mod cli;
mod config;
mod context;
mod oauth;
mod process;
mod proxy;
mod router;
mod sets;
mod terminal;
mod tui;
mod update;
mod util;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use cli::{AuthAction, Cli, Commands, ProfileAction, ProxyAction, SetsAction};
use config::ClaudexConfig;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut config = ClaudexConfig::load(cli.config.as_deref())?;

    // `claudex run` 时 proxy 日志只写文件，不污染 Claude Code 终端输出
    let is_run_command = matches!(&cli.command, Some(Commands::Run { .. }));

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // 日志文件（所有模式都写）
    let file_layer = proxy::proxy_log_path().and_then(|log_path| {
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok()
            .map(|file| {
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_writer(std::sync::Mutex::new(file))
            })
    });

    // stderr（run 模式不输出）
    let stderr_layer = if is_run_command {
        None
    } else {
        Some(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer)
        .with(file_layer)
        .init();

    match cli.command {
        Some(Commands::Run {
            profile: profile_name,
            model,
            hyperlinks,
            args,
        }) => {
            // Ensure proxy is running
            if !process::daemon::is_proxy_running()? {
                tracing::info!("proxy not running, starting in background...");
                start_proxy_background(&config).await?;
                // Brief wait for proxy to be ready
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }

            let profile = config
                .find_profile(&profile_name)
                .ok_or_else(|| anyhow::anyhow!("profile '{}' not found", profile_name))?
                .clone();

            process::launch::launch_claude(&config, &profile, model.as_deref(), &args, hyperlinks)?;

            // Claude 退出后，输出日志文件路径
            if let Some(log_path) = proxy::proxy_log_path() {
                if log_path.exists() {
                    eprintln!("\nClaudex proxy log: {}", log_path.display());
                }
            }
        }

        Some(Commands::Profile { action }) => match action {
            ProfileAction::List => {
                config::profile::list_profiles(&config).await;
            }
            ProfileAction::Show { name } => {
                config::profile::show_profile(&config, &name).await?;
            }
            ProfileAction::Test { name } => {
                config::profile::test_profile(&config, &name).await?;
            }
            ProfileAction::Add => {
                config::profile::interactive_add(&mut config).await?;
            }
            ProfileAction::Remove { name } => {
                config::profile::remove_profile(&mut config, &name)?;
            }
        },

        Some(Commands::Proxy { action }) => match action {
            ProxyAction::Start {
                port,
                daemon: as_daemon,
            } => {
                if as_daemon {
                    start_proxy_background(&config).await?;
                } else {
                    proxy::start_proxy(config, port).await?;
                }
            }
            ProxyAction::Stop => {
                process::daemon::stop_proxy()?;
            }
            ProxyAction::Status => {
                process::daemon::proxy_status()?;
            }
        },

        Some(Commands::Dashboard) => {
            let config_arc = std::sync::Arc::new(tokio::sync::RwLock::new(config));
            let metrics_store = proxy::metrics::MetricsStore::new();
            let health =
                std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
            tui::run_tui(config_arc, metrics_store, health).await?;
        }

        Some(Commands::Config { action }) => {
            config::cmd::dispatch(action, &mut config).await?;
        }

        Some(Commands::Update { check }) => {
            if check {
                match update::check_update().await? {
                    Some(version) => println!("New version available: {version}"),
                    None => println!("Already up to date (v{})", env!("CARGO_PKG_VERSION")),
                }
            } else {
                update::self_update().await?;
            }
        }

        Some(Commands::Sets { action }) => match action {
            SetsAction::Add {
                source,
                global,
                r#ref,
            } => {
                sets::add(&source, global, r#ref.as_deref()).await?;
            }
            SetsAction::Remove { name, global } => {
                sets::remove(&name, global).await?;
            }
            SetsAction::List { global } => {
                sets::list(global)?;
            }
            SetsAction::Update { name, global } => {
                sets::update(name.as_deref(), global).await?;
            }
            SetsAction::Show { name, global } => {
                sets::show(&name, global)?;
            }
        },

        Some(Commands::Auth { action }) => match action {
            AuthAction::Login {
                provider,
                profile,
                force,
                headless,
                enterprise_url,
            } => {
                let profile_name = profile.unwrap_or_else(|| provider.clone());
                oauth::providers::login(
                    &mut config,
                    &provider,
                    &profile_name,
                    force,
                    headless,
                    enterprise_url.as_deref(),
                )
                .await?;
            }
            AuthAction::Status { profile } => {
                oauth::providers::status(&config, profile.as_deref()).await?;
            }
            AuthAction::Logout { profile } => {
                oauth::providers::logout(&config, &profile).await?;
            }
            AuthAction::Refresh { profile } => {
                oauth::providers::refresh(&config, &profile).await?;
            }
        },

        None => {
            // Default: launch TUI if profiles exist, else show help
            if config.profiles.is_empty() {
                println!("Welcome to Claudex!");
                println!();
                println!("Get started:");
                println!("  1. Create config: claudex config");
                println!(
                    "  2. Add a profile: edit {:?}",
                    ClaudexConfig::config_path()?
                );
                println!("  3. Run claude:    claudex run <profile>");
                println!();
                println!("Use --help for more options.");
            } else {
                let config_arc = std::sync::Arc::new(tokio::sync::RwLock::new(config));
                let metrics_store = proxy::metrics::MetricsStore::new();
                let health =
                    std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
                tui::run_tui(config_arc, metrics_store, health).await?;
            }
        }
    }

    Ok(())
}

async fn start_proxy_background(config: &ClaudexConfig) -> Result<()> {
    let port = config.proxy_port;
    let host = config.proxy_host.clone();

    // Spawn proxy in a background task
    let config_clone = config.clone();
    tokio::spawn(async move {
        if let Err(e) = proxy::start_proxy(config_clone, None).await {
            tracing::error!("proxy failed: {e}");
        }
    });

    // Wait for it to be ready
    let client = reqwest::Client::new();
    let health_url = format!("http://{host}:{port}/health");
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if client.get(&health_url).send().await.is_ok() {
            tracing::info!("proxy is ready");
            return Ok(());
        }
    }

    anyhow::bail!("proxy failed to start within 2 seconds")
}
