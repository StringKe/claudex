use std::io::IsTerminal;
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

    let resume_hint = consume_resume_hint(&mut cmd, std::env::var("CLAUDEX_RESUME_HINT").ok());

    // PTY mode (Unix only): 非交互模式跳过 PTY
    #[cfg(unix)]
    let use_pty = !is_noninteractive
        && should_use_pty(
            &config.hyperlinks,
            hyperlinks_override,
            resume_hint.is_some(),
            std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
        );
    #[cfg(not(unix))]
    let use_pty = false;

    let mut resume_session_id: Option<String> = None;
    let mut managed_resume_hint_used = false;

    if use_pty {
        #[cfg(unix)]
        {
            tracing::info!("using PTY proxy mode");
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
            (resume_session_id, managed_resume_hint_used) =
                terminal::pty::spawn_with_pty(cmd, cwd, resume_hint.as_deref())?;
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
    if !managed_resume_hint_used {
        if let Some(session_id) = resume_session_id {
            print_claudex_resume_hint(&profile.name, &session_id, extra_args);
        }
    }

    Ok(())
}

/// 在 Claude Code 退出后追加 claudex resume 命令提示
fn print_claudex_resume_hint(profile_name: &str, session_id: &str, extra_args: &[String]) {
    let hint = build_resume_hint(profile_name, session_id, extra_args);
    eprintln!("\nResume this session with claudex:\n  {hint}");
}

/// 构造 claudex resume 命令字符串（纯函数，便于测试）
fn build_resume_hint(profile_name: &str, session_id: &str, extra_args: &[String]) -> String {
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

fn valid_resume_hint(value: Option<String>) -> Option<String> {
    value.filter(|hint| !hint.trim().is_empty() && !hint.chars().any(char::is_control))
}

fn consume_resume_hint(cmd: &mut Command, value: Option<String>) -> Option<String> {
    cmd.env_remove("CLAUDEX_RESUME_HINT");
    valid_resume_hint(value)
}

/// Decide whether to use PTY mode based on config + CLI flag.
#[cfg(unix)]
fn should_use_pty(
    config_hyperlinks: &HyperlinksConfig,
    cli_override: bool,
    managed_resume_hint: bool,
    interactive_terminal: bool,
) -> bool {
    if cli_override || (managed_resume_hint && interactive_terminal) {
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
        let hint = build_resume_hint("codex-sub", "abc-123", &[]);
        assert_eq!(hint, "claudex run codex-sub --resume abc-123");
    }

    #[test]
    fn test_build_resume_hint_with_extra_args() {
        let args = vec![
            "--dangerously-skip-permissions".to_string(),
            "--verbose".to_string(),
        ];
        let hint = build_resume_hint("codex-sub", "abc-123", &args);
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
        let hint = build_resume_hint("codex-sub", "new-session-id", &args);
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
        let hint = build_resume_hint("my-profile", "new-id", &args);
        assert_eq!(hint, "claudex run my-profile --resume new-id --verbose");
    }

    #[test]
    fn test_build_resume_hint_resume_only() {
        let args = vec!["--resume".to_string(), "old-id".to_string()];
        let hint = build_resume_hint("p", "new-id", &args);
        assert_eq!(hint, "claudex run p --resume new-id");
    }

    #[test]
    fn test_valid_resume_hint_accepts_one_line_command() {
        assert_eq!(
            valid_resume_hint(Some("orichum resume oc-s-0123456789abcdef".to_string())),
            Some("orichum resume oc-s-0123456789abcdef".to_string())
        );
    }

    #[test]
    fn test_valid_resume_hint_rejects_control_characters() {
        assert_eq!(
            valid_resume_hint(Some("orichum resume safe\nunsafe".to_string())),
            None
        );
        assert_eq!(
            valid_resume_hint(Some("orichum resume safe\x1b[2J".to_string())),
            None
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_managed_resume_hint_requires_pty() {
        assert!(should_use_pty(
            &HyperlinksConfig::Disabled,
            false,
            true,
            true
        ));
        assert!(!should_use_pty(
            &HyperlinksConfig::Disabled,
            false,
            true,
            false
        ));
    }

    #[test]
    fn test_consume_resume_hint_removes_child_environment() {
        let mut command = Command::new("claude");
        command.env("CLAUDEX_RESUME_HINT", "inherited");

        let hint = consume_resume_hint(
            &mut command,
            Some("orichum resume oc-s-0123456789abcdef".to_string()),
        );

        assert_eq!(
            hint.as_deref(),
            Some("orichum resume oc-s-0123456789abcdef")
        );
        assert!(command.get_envs().any(|(name, value)| {
            name == std::ffi::OsStr::new("CLAUDEX_RESUME_HINT") && value.is_none()
        }));
    }
}
