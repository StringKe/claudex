#![allow(dead_code)]

mod cli;
mod config;
mod context;
mod daemon;
mod launch;
mod metrics;
mod profile;
mod proxy;
mod router;
mod tui;
mod update;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use cli::{Cli, Commands, ProfileAction, ProxyAction};
use config::ClaudexConfig;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut config = ClaudexConfig::load()?;

    // Init tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .init();

    match cli.command {
        Some(Commands::Run {
            profile: profile_name,
            model,
            args,
        }) => {
            // Ensure proxy is running
            if !daemon::is_proxy_running()? {
                tracing::info!("proxy not running, starting in background...");
                start_proxy_background(&config).await?;
                // Brief wait for proxy to be ready
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }

            let profile = config
                .find_profile(&profile_name)
                .ok_or_else(|| anyhow::anyhow!("profile '{}' not found", profile_name))?
                .clone();

            launch::launch_claude(&config, &profile, model.as_deref(), &args)?;
        }

        Some(Commands::Profile { action }) => match action {
            ProfileAction::List => {
                profile::list_profiles(&config).await;
            }
            ProfileAction::Show { name } => {
                profile::show_profile(&config, &name).await?;
            }
            ProfileAction::Test { name } => {
                profile::test_profile(&config, &name).await?;
            }
            ProfileAction::Add => {
                profile::interactive_add(&mut config).await?;
            }
            ProfileAction::Remove { name } => {
                profile::remove_profile(&mut config, &name)?;
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
                daemon::stop_proxy()?;
            }
            ProxyAction::Status => {
                daemon::proxy_status()?;
            }
        },

        Some(Commands::Dashboard) => {
            let config_arc = std::sync::Arc::new(tokio::sync::RwLock::new(config));
            let metrics_store = metrics::MetricsStore::new();
            let health =
                std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
            tui::run_tui(config_arc, metrics_store, health).await?;
        }

        Some(Commands::Config { init }) => {
            if init {
                ClaudexConfig::init_local()?;
            } else {
                let source_display = config
                    .config_source
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "(default)".to_string());
                println!("Config loaded from: {}", source_display);
                println!("Profiles: {}", config.profiles.len());
                println!("Proxy: {}:{}", config.proxy_host, config.proxy_port);
                println!(
                    "Router: {}",
                    if config.router.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                println!("Context engine:");
                println!(
                    "  Compression: {}",
                    if config.context.compression.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                println!(
                    "  Sharing: {}",
                    if config.context.sharing.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                println!(
                    "  RAG: {}",
                    if config.context.rag.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
            }
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
                let metrics_store = metrics::MetricsStore::new();
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
