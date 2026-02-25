use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::config::{ClaudexConfig, HyperlinksConfig, ProfileConfig};
use crate::oauth::{AuthType, OAuthProvider};
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

    // 非交互模式检测：含 -p / --print，或首个 arg 不是 flag（裸 prompt）
    let is_noninteractive = extra_args.iter().any(|arg| arg == "-p" || arg == "--print")
        || extra_args.first().is_some_and(|arg| !arg.starts_with('-'));

    let mut cmd = Command::new(&config.claude_binary);

    // 不设 CLAUDE_CONFIG_DIR — 使用全局 ~/.claude，保留用户已有认证和设置。
    // Profile 差异化完全通过环境变量实现。

    let is_claude_subscription = profile.auth_type == AuthType::OAuth
        && profile.oauth_provider == Some(OAuthProvider::Claude);

    if is_claude_subscription {
        // Claude subscription：Claude Code 直接使用自身 OAuth
        // 不设 ANTHROPIC_BASE_URL / ANTHROPIC_API_KEY
        if model != profile.default_model {
            cmd.env("ANTHROPIC_MODEL", &model);
        }
    } else {
        // 标准代理流程（Gateway 模式）
        // 用 ANTHROPIC_AUTH_TOKEN（发 Authorization: Bearer header）而非 ANTHROPIC_API_KEY（发 X-Api-Key header）
        // 避免与 claude.ai OAuth token 产生 "Auth conflict"
        cmd.env("ANTHROPIC_BASE_URL", &proxy_base)
            .env("ANTHROPIC_AUTH_TOKEN", "claudex-passthrough")
            .env("ANTHROPIC_MODEL", &model);
    }

    if !profile.custom_headers.is_empty() {
        let headers: Vec<String> = profile
            .custom_headers
            .iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect();
        cmd.env("ANTHROPIC_CUSTOM_HEADERS", headers.join(","));
    }

    // 模型 slot 映射 → Claude Code 的 /model 切换
    if let Some(ref h) = profile.models.haiku {
        cmd.env("ANTHROPIC_DEFAULT_HAIKU_MODEL", h);
    }
    if let Some(ref s) = profile.models.sonnet {
        cmd.env("ANTHROPIC_DEFAULT_SONNET_MODEL", s);
    }
    if let Some(ref o) = profile.models.opus {
        cmd.env("ANTHROPIC_DEFAULT_OPUS_MODEL", o);
    }

    for (k, v) in &profile.extra_env {
        cmd.env(k, v);
    }

    cmd.args(extra_args);

    tracing::info!(
        profile = %profile.name,
        model = %model,
        proxy = %proxy_base,
        noninteractive = %is_noninteractive,
        "launching claude"
    );

    // 非交互模式跳过 PTY
    let use_pty = !is_noninteractive && should_use_pty(&config.hyperlinks, hyperlinks_override);

    if use_pty {
        tracing::info!("hyperlinks enabled, using PTY proxy mode");
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
        terminal::pty::spawn_with_pty(cmd, cwd)?;
    } else {
        let mut child = cmd.spawn().context("failed to execute claude binary")?;

        // 转发 SIGINT/SIGTERM 到子进程
        let _child_pid = child.id() as i32;
        unsafe {
            // 忽略父进程的 SIGINT，让子进程处理
            libc::signal(libc::SIGINT, libc::SIG_IGN);
        }

        let status = child.wait().context("failed to wait for claude")?;

        // 恢复 SIGINT 处理
        unsafe {
            libc::signal(libc::SIGINT, libc::SIG_DFL);
        }

        if !status.success() {
            // 被信号终止时（如 Ctrl+C）静默退出，不报错
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                if status.signal().is_some() {
                    std::process::exit(128 + status.signal().unwrap());
                }
            }
            bail!("claude exited with status: {}", status);
        }
    }

    Ok(())
}

/// Decide whether to use PTY mode based on config + CLI flag.
fn should_use_pty(config_hyperlinks: &HyperlinksConfig, cli_override: bool) -> bool {
    if cli_override {
        return true;
    }

    match config_hyperlinks {
        HyperlinksConfig::Enabled => true,
        HyperlinksConfig::Disabled => false,
        HyperlinksConfig::Auto => terminal::detect::terminal_supports_hyperlinks(),
    }
}
