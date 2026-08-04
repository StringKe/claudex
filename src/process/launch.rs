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

    // 自动禁用 Chrome 集成（除非用户显式传了 --chrome）
    if !extra_args.iter().any(|a| a == "--chrome") {
        cmd.arg("--no-chrome");
    }

    cmd.args(extra_args);

    tracing::info!(
        profile = %profile.name,
        model = %model,
        proxy = %proxy_base,
        noninteractive = %is_noninteractive,
        "launching claude"
    );

    let resume_hint_override = resume_hint_override();

    // PTY mode (Unix only): 非交互模式跳过 PTY
    #[cfg(unix)]
    let use_pty = should_launch_with_pty(
        is_noninteractive,
        should_use_pty(&config.hyperlinks, hyperlinks_override),
        resume_hint_override.is_some(),
    );
    #[cfg(not(unix))]
    let use_pty = false;

    let mut resume_session_id: Option<String> = None;

    if use_pty {
        #[cfg(unix)]
        {
            tracing::info!("hyperlinks enabled, using PTY proxy mode");
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
            resume_session_id =
                terminal::pty::spawn_with_pty(cmd, cwd, resume_hint_override.is_some())?;
        }
    } else {
        let mut child = cmd.spawn().context("failed to execute claude binary")?;

        // 转发 SIGINT/SIGTERM 到子进程
        #[cfg(unix)]
        unsafe {
            libc::signal(libc::SIGINT, libc::SIG_IGN);
        }

        let status = child.wait().context("failed to wait for claude")?;

        #[cfg(unix)]
        unsafe {
            libc::signal(libc::SIGINT, libc::SIG_DFL);
        }

        if !status.success() {
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

    // 追加 claudex resume 命令提示
    if let Some(session_id) = resume_session_id {
        print_claudex_resume_hint(
            &profile.name,
            &session_id,
            extra_args,
            resume_hint_override.as_deref(),
        );
    }

    Ok(())
}

/// 在 Claude Code 退出后追加 claudex resume 命令提示
fn print_claudex_resume_hint(
    profile_name: &str,
    session_id: &str,
    extra_args: &[String],
    override_hint: Option<&str>,
) {
    let hint = build_resume_hint(profile_name, session_id, extra_args, override_hint);
    let owner = if override_hint.is_some() {
        ""
    } else {
        " claudex"
    };
    eprintln!("\nResume this session with{owner}:\n  {hint}");
}

/// 构造 claudex resume 命令字符串（纯函数，便于测试）
fn build_resume_hint(
    profile_name: &str,
    session_id: &str,
    extra_args: &[String],
    override_hint: Option<&str>,
) -> String {
    if let Some(hint) = override_hint {
        return hint.to_string();
    }

    // 过滤掉原始 extra_args 中的 --resume 及其值参数
    let mut args_clean: Vec<&str> = Vec::new();
    let mut skip_next = false;
    for arg in extra_args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--resume" {
            skip_next = true;
            continue;
        }
        args_clean.push(arg);
    }

    let args_str = if args_clean.is_empty() {
        String::new()
    } else {
        format!(" {}", args_clean.join(" "))
    };

    format!("claudex run {profile_name} --resume {session_id}{args_str}")
}

fn resume_hint_override() -> Option<String> {
    let value = std::env::var("CLAUDEX_RESUME_HINT").ok()?;
    let hint = value.trim();
    if hint.is_empty() || hint.len() > 1024 || hint.chars().any(char::is_control) {
        return None;
    }
    Some(hint.to_string())
}

/// Decide whether to use PTY mode based on config + CLI flag.
#[cfg(unix)]
fn should_launch_with_pty(
    is_noninteractive: bool,
    hyperlinks_enabled: bool,
    has_owned_resume_hint: bool,
) -> bool {
    !is_noninteractive && (hyperlinks_enabled || has_owned_resume_hint)
}

/// Decide whether hyperlinks require PTY mode based on config + CLI flag.
#[cfg(unix)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_resume_hint_no_extra_args() {
        let hint = build_resume_hint("codex-sub", "abc-123", &[], None);
        assert_eq!(hint, "claudex run codex-sub --resume abc-123");
    }

    #[test]
    fn test_build_resume_hint_with_extra_args() {
        let args = vec![
            "--dangerously-skip-permissions".to_string(),
            "--verbose".to_string(),
        ];
        let hint = build_resume_hint("codex-sub", "abc-123", &args, None);
        assert_eq!(
            hint,
            "claudex run codex-sub --resume abc-123 --dangerously-skip-permissions --verbose"
        );
    }

    #[test]
    fn test_build_resume_hint_filters_existing_resume() {
        let args = vec![
            "--resume".to_string(),
            "old-session-id".to_string(),
            "--dangerously-skip-permissions".to_string(),
        ];
        let hint = build_resume_hint("codex-sub", "new-session-id", &args, None);
        assert_eq!(
            hint,
            "claudex run codex-sub --resume new-session-id --dangerously-skip-permissions"
        );
    }

    #[test]
    fn test_build_resume_hint_resume_at_end() {
        let args = vec![
            "--verbose".to_string(),
            "--resume".to_string(),
            "old-id".to_string(),
        ];
        let hint = build_resume_hint("my-profile", "new-id", &args, None);
        assert_eq!(hint, "claudex run my-profile --resume new-id --verbose");
    }

    #[test]
    fn test_build_resume_hint_resume_only() {
        let args = vec!["--resume".to_string(), "old-id".to_string()];
        let hint = build_resume_hint("p", "new-id", &args, None);
        assert_eq!(hint, "claudex run p --resume new-id");
    }

    #[test]
    fn test_build_resume_hint_prefers_owned_override() {
        let hint = build_resume_hint(
            "codex-sub",
            "abc-123",
            &[],
            Some("orichum resume oc-s-0000000000000001"),
        );
        assert_eq!(hint, "orichum resume oc-s-0000000000000001");
    }

    #[cfg(unix)]
    #[test]
    fn test_owned_resume_hint_forces_pty_without_hyperlinks() {
        assert!(should_launch_with_pty(false, false, true));
        assert!(!should_launch_with_pty(true, false, true));
    }
}
