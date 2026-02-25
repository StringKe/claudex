use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::config::{ClaudexConfig, HyperlinksConfig, ProfileConfig};
use crate::terminal;

pub fn launch_claude(
    config: &ClaudexConfig,
    profile: &ProfileConfig,
    model_override: Option<&str>,
    extra_args: &[String],
    hyperlinks_override: bool,
) -> Result<()> {
    let proxy_base = format!(
        "http://{}:{}/proxy/{}",
        config.proxy_host, config.proxy_port, profile.name
    );

    let model = model_override
        .map(|m| config.resolve_model(m))
        .unwrap_or_else(|| config.resolve_model(&profile.default_model));

    let config_dir = dirs::config_dir()
        .context("cannot determine config dir")?
        .join(format!("claude-{}", profile.name));

    std::fs::create_dir_all(&config_dir)?;

    let mut cmd = Command::new(&config.claude_binary);

    cmd.env("CLAUDE_CONFIG_DIR", &config_dir)
        .env("ANTHROPIC_BASE_URL", &proxy_base)
        .env("ANTHROPIC_API_KEY", "claudex-passthrough")
        .env("ANTHROPIC_MODEL", &model);

    if !profile.custom_headers.is_empty() {
        let headers: Vec<String> = profile
            .custom_headers
            .iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect();
        cmd.env("ANTHROPIC_CUSTOM_HEADERS", headers.join(","));
    }

    for (k, v) in &profile.extra_env {
        cmd.env(k, v);
    }

    cmd.args(extra_args);

    tracing::info!(
        profile = %profile.name,
        model = %model,
        proxy = %proxy_base,
        "launching claude"
    );

    // Determine whether to use PTY mode for hyperlinks
    let use_pty = should_use_pty(&config.hyperlinks, hyperlinks_override);

    if use_pty {
        tracing::info!("hyperlinks enabled, using PTY proxy mode");
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
        terminal::pty::spawn_with_pty(cmd, cwd)?;
    } else {
        let status = cmd.status().context("failed to execute claude binary")?;
        if !status.success() {
            bail!("claude exited with status: {}", status);
        }
    }

    Ok(())
}

/// Decide whether to use PTY mode based on config + CLI flag.
fn should_use_pty(config_hyperlinks: &HyperlinksConfig, cli_override: bool) -> bool {
    // CLI --hyperlinks flag forces enable
    if cli_override {
        return true;
    }

    match config_hyperlinks {
        HyperlinksConfig::Enabled => true,
        HyperlinksConfig::Disabled => false,
        HyperlinksConfig::Auto => terminal::detect::terminal_supports_hyperlinks(),
    }
}
