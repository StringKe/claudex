use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::config::{ClaudexConfig, ProfileConfig};

pub fn launch_claude(
    config: &ClaudexConfig,
    profile: &ProfileConfig,
    model_override: Option<&str>,
    extra_args: &[String],
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

    for (k, v) in &profile.custom_headers {
        let header_val = format!("{}:{}", k, v);
        cmd.env("ANTHROPIC_CUSTOM_HEADERS", &header_val);
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

    let status = cmd.status().context("failed to execute claude binary")?;

    if !status.success() {
        bail!("claude exited with status: {}", status);
    }

    Ok(())
}
