use anyhow::{Context, Result};

use crate::config::{ClaudexConfig, ProfileConfig, ProfileModels, ProviderType};
use crate::oauth::{AuthType, OAuthProvider, OAuthToken};

/// Provider 默认配置（开箱即用）
struct ProviderDefaults {
    provider_type: ProviderType,
    base_url: &'static str,
    default_model: &'static str,
    models: ProfileModels,
    max_tokens: Option<u64>,
}

fn provider_defaults(provider: &OAuthProvider) -> ProviderDefaults {
    match provider {
        OAuthProvider::Claude => ProviderDefaults {
            provider_type: ProviderType::DirectAnthropic,
            base_url: "https://api.claude.ai",
            default_model: "claude-sonnet-4-20250514",
            models: ProfileModels {
                haiku: Some("claude-haiku-4-5-20251001".to_string()),
                sonnet: Some("claude-sonnet-4-20250514".to_string()),
                opus: Some("claude-opus-4-6-20250610".to_string()),
            },
            max_tokens: None,
        },
        OAuthProvider::Chatgpt | OAuthProvider::Openai => ProviderDefaults {
            provider_type: ProviderType::OpenAIResponses,
            base_url: "https://chatgpt.com/backend-api/codex",
            default_model: "gpt-5.3-codex",
            models: ProfileModels {
                haiku: Some("gpt-5.3-codex".to_string()),
                sonnet: Some("gpt-5.3-codex".to_string()),
                opus: Some("gpt-5.3-codex".to_string()),
            },
            max_tokens: None,
        },
        OAuthProvider::Google => ProviderDefaults {
            provider_type: ProviderType::OpenAICompatible,
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
            default_model: "gemini-2.5-pro-preview",
            models: ProfileModels {
                haiku: Some("gemini-2.5-flash-preview".to_string()),
                sonnet: Some("gemini-2.5-pro-preview".to_string()),
                opus: Some("gemini-2.5-pro-preview".to_string()),
            },
            max_tokens: None,
        },
        OAuthProvider::Qwen => ProviderDefaults {
            provider_type: ProviderType::OpenAICompatible,
            base_url: "https://chat.qwen.ai/api",
            default_model: "qwen3-235b-a22b",
            models: ProfileModels {
                haiku: Some("qwen3-30b-a3b".to_string()),
                sonnet: Some("qwen3-235b-a22b".to_string()),
                opus: Some("qwen3-235b-a22b".to_string()),
            },
            max_tokens: None,
        },
        OAuthProvider::Kimi => ProviderDefaults {
            provider_type: ProviderType::OpenAICompatible,
            base_url: "https://api.moonshot.cn/v1",
            default_model: "kimi-k2-0905-preview",
            models: ProfileModels {
                haiku: Some("moonshot-v1-8k".to_string()),
                sonnet: Some("kimi-k2-0905-preview".to_string()),
                opus: Some("kimi-k2-0905-preview".to_string()),
            },
            max_tokens: None,
        },
        OAuthProvider::Github => ProviderDefaults {
            provider_type: ProviderType::OpenAICompatible,
            base_url: "https://api.githubcopilot.com",
            default_model: "gpt-4o",
            models: ProfileModels {
                haiku: Some("gpt-4o-mini".to_string()),
                sonnet: Some("gpt-4o".to_string()),
                opus: Some("o3".to_string()),
            },
            max_tokens: None,
        },
    }
}

/// 确保 OAuth profile 存在于配置中，不存在则自动创建
fn ensure_oauth_profile(
    config: &mut ClaudexConfig,
    profile_name: &str,
    provider: &OAuthProvider,
) -> Result<()> {
    if config.find_profile(profile_name).is_some() {
        // 更新现有 profile 的 auth_type 和 oauth_provider
        if let Some(p) = config.find_profile_mut(profile_name) {
            p.auth_type = AuthType::OAuth;
            p.oauth_provider = Some(provider.clone());
        }
        return Ok(());
    }

    let defaults = provider_defaults(provider);

    let profile = ProfileConfig {
        name: profile_name.to_string(),
        provider_type: defaults.provider_type,
        base_url: defaults.base_url.to_string(),
        default_model: defaults.default_model.to_string(),
        auth_type: AuthType::OAuth,
        oauth_provider: Some(provider.clone()),
        models: defaults.models,
        max_tokens: defaults.max_tokens,
        ..Default::default()
    };

    config.profiles.push(profile);
    config.save().context("failed to save config")?;
    println!(
        "Created OAuth profile '{profile_name}' for {}",
        provider.display_name()
    );
    Ok(())
}

// ── OAuth client IDs ─────────────────────────────────────────────────────
// ChatGPT + GitHub Copilot IDs 已移至 exchange.rs
// Qwen 保留在此，因为仅在 providers.rs 的 device code flow 中使用

const QWEN_CLIENT_ID: &str = "claudex-qwen";

// ── Login ───────────────────────────────────────────────────────────────

pub async fn login(
    config: &mut ClaudexConfig,
    provider_str: &str,
    profile_name: &str,
    force: bool,
    headless: bool,
) -> Result<()> {
    let provider = OAuthProvider::from_str(provider_str).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown provider '{}'. Supported: claude, chatgpt/openai, google, qwen, kimi, github/copilot",
            provider_str
        )
    })?;

    ensure_oauth_profile(config, profile_name, &provider)?;

    match provider {
        OAuthProvider::Claude => login_claude(profile_name).await,
        OAuthProvider::Chatgpt | OAuthProvider::Openai => {
            login_chatgpt(profile_name, force, headless).await
        }
        OAuthProvider::Google => login_google(profile_name).await,
        OAuthProvider::Qwen => login_device_code(profile_name, &OAuthProvider::Qwen).await,
        OAuthProvider::Kimi => login_kimi(profile_name).await,
        OAuthProvider::Github => login_github(profile_name, force).await,
    }
}

/// Claude: 只读外部 credentials，不自建 OAuth
async fn login_claude(profile_name: &str) -> Result<()> {
    println!("Reading Claude credentials from ~/.claude/.credentials.json...");

    let cred = super::source::read_claude_credentials()
        .context("Failed to read Claude credentials. Make sure Claude Code is installed and you have logged in with `claude` first.")?;
    let token = cred.into_oauth_token();

    super::source::store_keyring(profile_name, &token)?;
    println!("Claude OAuth token stored for profile '{profile_name}'.");
    println!(
        "Note: Claude subscription profiles bypass the proxy (Claude Code uses its own OAuth)."
    );
    Ok(())
}

/// ChatGPT 订阅 login (别名: openai, codex)
/// 支持三种方式: 读取已有 Codex CLI 凭证、Browser PKCE、Headless Device Code
async fn login_chatgpt(profile_name: &str, force: bool, headless: bool) -> Result<()> {
    // 非 force 模式: 优先读取已有 Codex CLI credentials
    if !force {
        match super::source::read_codex_credentials() {
            Ok(cred) => {
                let auth_mode = cred
                    .extra
                    .as_ref()
                    .and_then(|e| e.get("auth_mode"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                println!("Found Codex CLI credentials (auth_mode: {auth_mode})");
                let token = cred.into_oauth_token();
                super::source::store_keyring(profile_name, &token)?;
                println!("ChatGPT OAuth token stored for profile '{profile_name}'.");
                println!("Token will be refreshed automatically.");
                return Ok(());
            }
            Err(e) => {
                tracing::debug!("Codex credentials not available: {e}");
            }
        }
    }

    if !force {
        println!("No Codex CLI credentials found at ~/.codex/auth.json");
    }

    let use_headless = headless || std::env::var("CLAUDEX_HEADLESS").is_ok() || !atty_check();

    if use_headless {
        println!("Using headless device code flow...");
        login_chatgpt_headless(profile_name).await
    } else {
        println!("Using browser PKCE flow...");
        login_chatgpt_browser(profile_name).await
    }
}

/// Codex CLI 默认回调端口（OAuth app 注册的 redirect_uri 使用此端口）
const CHATGPT_CALLBACK_PORT: u16 = 1455;

/// ChatGPT Browser PKCE login
async fn login_chatgpt_browser(profile_name: &str) -> Result<()> {
    let pkce = super::server::PkceChallenge::generate();
    let port = CHATGPT_CALLBACK_PORT;
    let state = uuid::Uuid::new_v4().to_string();

    let authorize_url = super::exchange::build_chatgpt_authorize_url(port, &pkce, &state);

    println!("Opening browser for ChatGPT login...");
    println!("If the browser doesn't open, visit:");
    println!("  {authorize_url}");
    println!();

    let _ = open_browser(&authorize_url);

    // 启动回调服务器等待 authorization code
    let code = super::server::start_callback_server(port)
        .await
        .context("failed to receive authorization callback")?;

    println!("Authorization code received, exchanging for token...");

    let client = reqwest::Client::new();
    let redirect_uri = format!("http://localhost:{port}/auth/callback");
    let token =
        super::exchange::exchange_chatgpt_code(&client, &code, &redirect_uri, &pkce.code_verifier)
            .await?;

    // 存储到 keyring 并回写 ~/.codex/auth.json
    super::source::store_keyring(profile_name, &token)?;
    super::source::write_codex_credentials_atomic(&token)?;

    println!("ChatGPT OAuth token stored for profile '{profile_name}'.");
    Ok(())
}

/// ChatGPT Headless Device Code login
async fn login_chatgpt_headless(profile_name: &str) -> Result<()> {
    let client = reqwest::Client::new();

    let device_resp = super::exchange::chatgpt_device_auth_request(&client).await?;

    println!();
    println!("  Open: https://auth.openai.com/codex/device");
    println!("  Enter code: {}", device_resp.user_code);
    println!();
    println!("Waiting for authorization...");

    let _ = open_browser("https://auth.openai.com/codex/device");

    let token = super::exchange::chatgpt_device_auth_poll(
        &client,
        &device_resp.device_auth_id,
        &device_resp.user_code,
    )
    .await?;

    super::source::store_keyring(profile_name, &token)?;
    super::source::write_codex_credentials_atomic(&token)?;

    println!("ChatGPT OAuth token stored for profile '{profile_name}'.");
    Ok(())
}

/// GitHub Copilot login
async fn login_github(profile_name: &str, force: bool) -> Result<()> {
    // 非 force 模式: 优先从 ~/.config/github-copilot/ 读取已有 token
    if !force {
        match super::source::read_copilot_config() {
            Ok(cred) => {
                println!("Found existing GitHub Copilot credentials, verifying...");

                let client = reqwest::Client::new();
                match super::exchange::exchange_github_for_copilot(&client, &cred.access_token)
                    .await
                {
                    Ok(copilot) => {
                        let token = OAuthToken {
                            access_token: copilot.token,
                            refresh_token: None,
                            expires_at: Some(copilot.expires_at * 1000),
                            token_type: Some("Bearer".to_string()),
                            scopes: None,
                            extra: Some(serde_json::json!({
                                "provider": "copilot",
                                "github_token": cred.access_token,
                            })),
                        };
                        super::source::store_keyring(profile_name, &token)?;
                        println!("GitHub Copilot token stored for profile '{profile_name}'.");
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::warn!(
                            "existing Copilot token invalid: {e}, falling back to device code"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::debug!("no existing Copilot config: {e}");
            }
        }
    }

    // Device code flow
    println!("Starting GitHub device code flow...");

    let client = reqwest::Client::new();
    let resp = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", super::exchange::GITHUB_COPILOT_CLIENT_ID),
            ("scope", "read:user"),
        ])
        .send()
        .await
        .context("failed to request GitHub device code")?;

    let body: serde_json::Value = resp.json().await.context("invalid device code response")?;

    let user_code = body
        .get("user_code")
        .and_then(|v| v.as_str())
        .context("missing user_code")?;
    let verification_uri = body
        .get("verification_uri")
        .and_then(|v| v.as_str())
        .context("missing verification_uri")?;
    let device_code = body
        .get("device_code")
        .and_then(|v| v.as_str())
        .context("missing device_code")?;
    let interval = body.get("interval").and_then(|v| v.as_u64()).unwrap_or(5);

    println!();
    println!("  Open: {verification_uri}");
    println!("  Enter code: {user_code}");
    println!();
    println!("Waiting for authorization...");

    let _ = open_browser(verification_uri);

    let token_resp = super::server::poll_device_code(
        &client,
        "https://github.com/login/oauth/access_token",
        device_code,
        super::exchange::GITHUB_COPILOT_CLIENT_ID,
        interval,
        "urn:ietf:params:oauth:grant-type:device_code",
    )
    .await?;

    let github_token = token_resp
        .get("access_token")
        .and_then(|v| v.as_str())
        .context("missing access_token in GitHub response")?;

    // 交换为 Copilot bearer token
    let copilot = super::exchange::exchange_github_for_copilot(&client, github_token).await?;

    let token = OAuthToken {
        access_token: copilot.token,
        refresh_token: None,
        expires_at: Some(copilot.expires_at * 1000),
        token_type: Some("Bearer".to_string()),
        scopes: None,
        extra: Some(serde_json::json!({
            "provider": "copilot",
            "github_token": github_token,
        })),
    };

    super::source::store_keyring(profile_name, &token)?;
    println!("GitHub Copilot token stored for profile '{profile_name}'.");
    Ok(())
}

/// 检测是否有 tty
fn atty_check() -> bool {
    std::io::IsTerminal::is_terminal(&std::io::stdin())
}

/// Google: 读取 Gemini CLI 外部 credentials
async fn login_google(profile_name: &str) -> Result<()> {
    println!("Reading Google/Gemini credentials from external CLI...");
    println!(
        "Note: Google OAuth requires a registered Client ID. Using external CLI token instead."
    );

    let cred = super::source::read_gemini_credentials()
        .context("Failed to read Gemini CLI credentials. Make sure Gemini CLI is installed and authenticated.")?;
    let token = cred.into_oauth_token();

    super::source::store_keyring(profile_name, &token)?;
    println!("Google OAuth token stored for profile '{profile_name}'.");
    Ok(())
}

/// Kimi: 读取外部 CLI credentials
async fn login_kimi(profile_name: &str) -> Result<()> {
    println!("Reading Kimi credentials from external CLI...");

    let cred = super::source::read_kimi_credentials().context(
        "Failed to read Kimi CLI credentials. Make sure Kimi CLI is installed and authenticated.",
    )?;
    let token = cred.into_oauth_token();

    super::source::store_keyring(profile_name, &token)?;
    println!("Kimi OAuth token stored for profile '{profile_name}'.");
    Ok(())
}

/// Device Code Flow (Qwen only; GitHub uses login_github)
async fn login_device_code(profile_name: &str, provider: &OAuthProvider) -> Result<()> {
    let (device_url, token_url, client_id, scope, grant_type) = match provider {
        OAuthProvider::Qwen => (
            "https://chat.qwen.ai/api/oauth/device/code",
            "https://chat.qwen.ai/api/oauth/token",
            QWEN_CLIENT_ID,
            "",
            "urn:ietf:params:oauth:grant-type:device_code",
        ),
        _ => anyhow::bail!("device code flow not supported for {:?}", provider),
    };

    println!("Starting {} device code flow...", provider.display_name());

    let client = reqwest::Client::new();

    let mut form = vec![("client_id", client_id)];
    if !scope.is_empty() {
        form.push(("scope", scope));
    }

    let resp = client
        .post(device_url)
        .header("Accept", "application/json")
        .form(&form)
        .send()
        .await
        .context("failed to request device code")?;

    let body: serde_json::Value = resp.json().await.context("invalid device code response")?;

    let user_code = body
        .get("user_code")
        .and_then(|v| v.as_str())
        .context("missing user_code in response")?;
    let verification_uri = body
        .get("verification_uri")
        .or_else(|| body.get("verification_url"))
        .and_then(|v| v.as_str())
        .context("missing verification_uri in response")?;
    let device_code = body
        .get("device_code")
        .and_then(|v| v.as_str())
        .context("missing device_code in response")?;
    let interval = body.get("interval").and_then(|v| v.as_u64()).unwrap_or(5);

    println!();
    println!("  Open: {verification_uri}");
    println!("  Enter code: {user_code}");
    println!();
    println!("Waiting for authorization...");

    let _ = open_browser(verification_uri);

    let token_resp = super::server::poll_device_code(
        &client,
        token_url,
        device_code,
        client_id,
        interval,
        grant_type,
    )
    .await?;

    let token =
        OAuthToken::from_token_response(&token_resp).context("failed to parse token response")?;

    super::source::store_keyring(profile_name, &token)?;
    println!(
        "{} OAuth token stored for profile '{profile_name}'.",
        provider.display_name()
    );
    Ok(())
}

// ── Status ──────────────────────────────────────────────────────────────

pub async fn status(config: &ClaudexConfig, profile_name: Option<&str>) -> Result<()> {
    let profiles: Vec<&ProfileConfig> = if let Some(name) = profile_name {
        config
            .find_profile(name)
            .map(|p| vec![p])
            .unwrap_or_default()
    } else {
        config
            .profiles
            .iter()
            .filter(|p| p.auth_type == AuthType::OAuth)
            .collect()
    };

    if profiles.is_empty() {
        println!("No OAuth profiles found.");
        return Ok(());
    }

    println!(
        "{:<20} {:<10} {:<10} EXPIRES",
        "PROFILE", "PROVIDER", "STATUS"
    );
    println!("{}", "-".repeat(60));

    for profile in profiles {
        let provider_name = profile
            .oauth_provider
            .as_ref()
            .map(|p| p.display_name())
            .unwrap_or("?");

        let (status_str, expires_str) = match super::source::load_keyring(&profile.name) {
            Ok(token) => {
                if token.is_expired(0) {
                    ("expired".to_string(), format_expires(token.expires_at))
                } else if token.is_expired(300) {
                    ("expiring".to_string(), format_expires(token.expires_at))
                } else {
                    ("valid".to_string(), format_expires(token.expires_at))
                }
            }
            Err(_) => ("no token".to_string(), "-".to_string()),
        };

        println!(
            "{:<20} {:<10} {:<10} {}",
            profile.name, provider_name, status_str, expires_str
        );
    }

    Ok(())
}

fn format_expires(expires_at: Option<i64>) -> String {
    match expires_at {
        Some(ms) => {
            let dt = chrono::DateTime::from_timestamp_millis(ms);
            match dt {
                Some(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
                None => "invalid".to_string(),
            }
        }
        None => "no expiry".to_string(),
    }
}

// ── Logout ──────────────────────────────────────────────────────────────

pub async fn logout(_config: &ClaudexConfig, profile_name: &str) -> Result<()> {
    match super::source::delete_keyring(profile_name) {
        Ok(()) => println!("OAuth token removed for profile '{profile_name}'."),
        Err(e) => println!("No token to remove for '{profile_name}': {e}"),
    }
    Ok(())
}

// ── Refresh ─────────────────────────────────────────────────────────────

pub async fn refresh(config: &ClaudexConfig, profile_name: &str) -> Result<()> {
    let profile = config
        .find_profile(profile_name)
        .ok_or_else(|| anyhow::anyhow!("profile '{}' not found", profile_name))?;

    let provider = profile
        .oauth_provider
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("profile '{}' has no oauth_provider", profile_name))?;

    match provider {
        OAuthProvider::Claude => {
            let cred = super::source::read_claude_credentials()?;
            let token = cred.into_oauth_token();
            super::source::store_keyring(profile_name, &token)?;
            println!("Refreshed Claude token from ~/.claude/.credentials.json");
        }
        OAuthProvider::Google | OAuthProvider::Kimi => {
            let cred = super::source::load_credential_chain(provider)?;
            let token = cred.into_oauth_token();
            super::source::store_keyring(profile_name, &token)?;
            println!(
                "Refreshed {} token from external CLI",
                provider.display_name()
            );
        }
        OAuthProvider::Chatgpt | OAuthProvider::Openai => {
            let cred =
                super::source::read_codex_credentials().context("cannot read Codex credentials")?;
            let token = cred.into_oauth_token();
            let refresh_tok = token.refresh_token.as_ref().ok_or_else(|| {
                anyhow::anyhow!("no refresh_token in Codex credentials, please re-login")
            })?;

            let client = reqwest::Client::new();
            let new_token = super::exchange::refresh_chatgpt_token(&client, refresh_tok).await?;
            super::source::store_keyring(profile_name, &new_token)?;
            println!("Token refreshed for profile '{profile_name}'.");
        }
        OAuthProvider::Github => {
            // GitHub: 重新交换 Copilot bearer token
            let cred = super::source::load_credential_chain(&OAuthProvider::Github)
                .context("no GitHub credentials available")?;
            let client = reqwest::Client::new();
            let copilot =
                super::exchange::exchange_github_for_copilot(&client, &cred.access_token).await?;
            let token = OAuthToken {
                access_token: copilot.token,
                refresh_token: None,
                expires_at: Some(copilot.expires_at * 1000),
                token_type: Some("Bearer".to_string()),
                scopes: None,
                extra: Some(serde_json::json!({
                    "provider": "copilot",
                    "github_token": cred.access_token,
                })),
            };
            super::source::store_keyring(profile_name, &token)?;
            println!("GitHub Copilot token refreshed for profile '{profile_name}'.");
        }
        OAuthProvider::Qwen => {
            let token = super::source::load_keyring(profile_name)
                .context("no existing token to refresh")?;
            let refresh_token = token
                .refresh_token
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("no refresh_token available, please re-login"))?;

            let client = reqwest::Client::new();
            let resp = super::server::refresh_access_token(
                &client,
                "https://chat.qwen.ai/api/oauth/token",
                refresh_token,
                QWEN_CLIENT_ID,
            )
            .await?;

            let mut new_token = OAuthToken::from_token_response(&resp)
                .context("failed to parse refreshed token")?;

            if new_token.refresh_token.is_none() {
                new_token.refresh_token = token.refresh_token;
            }

            super::source::store_keyring(profile_name, &new_token)?;
            println!("Token refreshed for profile '{profile_name}'.");
        }
    }

    Ok(())
}

// ── Token refresh for proxy (called from handler) ───────────────────────

/// 确保 profile 的 OAuth token 有效（旧接口，保留向后兼容）
/// 新代码应使用 TokenManager::get_token()
pub async fn ensure_valid_token(profile: &mut ProfileConfig) -> Result<()> {
    if profile.auth_type != AuthType::OAuth {
        return Ok(());
    }

    if !profile.api_key.is_empty() {
        return Ok(());
    }

    let provider = match profile.oauth_provider.as_ref() {
        Some(p) => p.normalize(),
        None => anyhow::bail!("no oauth_provider for profile '{}'", profile.name),
    };

    let cred = super::source::load_credential_chain(&provider).with_context(|| {
        format!(
            "OAuth token not available for '{}'. Run `claudex auth login {} --profile {}`",
            profile.name,
            provider.display_name().to_lowercase(),
            profile.name
        )
    })?;
    let token = cred.into_oauth_token();

    if token.is_expired(60) {
        if matches!(provider, OAuthProvider::Chatgpt | OAuthProvider::Openai) {
            if let Some(ref refresh_tok) = token.refresh_token {
                tracing::info!(
                    "ChatGPT token expired for profile '{}', refreshing...",
                    profile.name
                );
                let client = reqwest::Client::new();
                let new_token =
                    super::exchange::refresh_chatgpt_token(&client, refresh_tok).await?;
                super::manager::apply_token_to_profile(profile, &new_token);
                return Ok(());
            }
        }
        anyhow::bail!(
            "OAuth token expired for '{}' and cannot auto-refresh. Run `claudex auth refresh {}`",
            profile.name,
            profile.name
        );
    }

    super::manager::apply_token_to_profile(profile, &token);
    Ok(())
}

// ── Public APIs (kept for backward compat with handler.rs) ───────────────

/// Refresh ChatGPT token (public entry point)
pub async fn refresh_chatgpt_token_pub(refresh_token: &str) -> Result<OAuthToken> {
    let client = reqwest::Client::new();
    super::exchange::refresh_chatgpt_token(&client, refresh_token).await
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn urlencoded(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn open_browser(url: &str) -> Result<()> {
    open::that(url).context("failed to open browser")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_defaults_claude() {
        let defaults = provider_defaults(&OAuthProvider::Claude);
        assert_eq!(defaults.base_url, "https://api.claude.ai");
        assert!(matches!(
            defaults.provider_type,
            ProviderType::DirectAnthropic
        ));
    }

    #[test]
    fn test_provider_defaults_openai() {
        let defaults = provider_defaults(&OAuthProvider::Openai);
        assert_eq!(defaults.base_url, "https://chatgpt.com/backend-api/codex");
        assert_eq!(defaults.default_model, "gpt-5.3-codex");
        assert!(matches!(
            defaults.provider_type,
            ProviderType::OpenAIResponses
        ));
        assert!(defaults.models.haiku.is_some());
        assert!(defaults.models.sonnet.is_some());
        assert!(defaults.models.opus.is_some());
    }

    #[test]
    fn test_provider_defaults_github() {
        let defaults = provider_defaults(&OAuthProvider::Github);
        assert_eq!(defaults.base_url, "https://api.githubcopilot.com");
        assert_eq!(defaults.default_model, "gpt-4o");
    }

    #[test]
    fn test_urlencoded() {
        assert_eq!(
            urlencoded("http://127.0.0.1:8080/callback"),
            "http%3A%2F%2F127.0.0.1%3A8080%2Fcallback"
        );
    }

    #[test]
    fn test_format_expires() {
        assert_eq!(format_expires(None), "no expiry");
        // A known timestamp
        let ms = 1700000000000_i64;
        let result = format_expires(Some(ms));
        assert!(!result.is_empty());
        assert_ne!(result, "invalid");
    }

    // ── provider_defaults 全覆盖 ──────────────────────────────

    #[test]
    fn test_provider_defaults_google() {
        let defaults = provider_defaults(&OAuthProvider::Google);
        assert_eq!(
            defaults.base_url,
            "https://generativelanguage.googleapis.com/v1beta/openai"
        );
        assert_eq!(defaults.default_model, "gemini-2.5-pro-preview");
        assert!(matches!(
            defaults.provider_type,
            ProviderType::OpenAICompatible
        ));
    }

    #[test]
    fn test_provider_defaults_qwen() {
        let defaults = provider_defaults(&OAuthProvider::Qwen);
        assert_eq!(defaults.base_url, "https://chat.qwen.ai/api");
        assert_eq!(defaults.default_model, "qwen3-235b-a22b");
        assert!(matches!(
            defaults.provider_type,
            ProviderType::OpenAICompatible
        ));
    }

    #[test]
    fn test_provider_defaults_kimi() {
        let defaults = provider_defaults(&OAuthProvider::Kimi);
        assert_eq!(defaults.base_url, "https://api.moonshot.cn/v1");
        assert_eq!(defaults.default_model, "kimi-k2-0905-preview");
        assert!(matches!(
            defaults.provider_type,
            ProviderType::OpenAICompatible
        ));
    }

    #[test]
    fn test_all_providers_have_model_slots() {
        let providers = [
            OAuthProvider::Claude,
            OAuthProvider::Openai,
            OAuthProvider::Google,
            OAuthProvider::Qwen,
            OAuthProvider::Kimi,
            OAuthProvider::Github,
        ];
        for provider in &providers {
            let defaults = provider_defaults(provider);
            assert!(
                defaults.models.haiku.is_some(),
                "{:?} missing haiku model",
                provider
            );
            assert!(
                defaults.models.sonnet.is_some(),
                "{:?} missing sonnet model",
                provider
            );
            assert!(
                defaults.models.opus.is_some(),
                "{:?} missing opus model",
                provider
            );
        }
    }

    // ── urlencoded 边界 ───────────────────────────────────────

    #[test]
    fn test_urlencoded_special_chars() {
        assert_eq!(urlencoded("a=b&c=d"), "a%3Db%26c%3Dd");
        assert_eq!(urlencoded("foo?bar"), "foo%3Fbar");
    }

    #[test]
    fn test_urlencoded_empty() {
        assert_eq!(urlencoded(""), "");
    }

    #[test]
    fn test_urlencoded_no_special_chars() {
        assert_eq!(urlencoded("hello-world"), "hello-world");
    }

    #[test]
    fn test_urlencoded_hash_and_plus() {
        // form_urlencoded encodes '#' as %23, '+' as %2B
        assert!(urlencoded("a#b").contains("%23"));
        assert!(urlencoded("a+b").contains("%2B"));
    }

    #[test]
    fn test_urlencoded_space() {
        // form_urlencoded encodes space as '+'
        assert_eq!(urlencoded("hello world"), "hello+world");
    }

    #[test]
    fn test_urlencoded_at_sign() {
        assert_eq!(urlencoded("user@host"), "user%40host");
    }

    // ── format_expires 边界 ───────────────────────────────────

    #[test]
    fn test_format_expires_zero_timestamp() {
        let result = format_expires(Some(0));
        // Unix epoch: 1970-01-01 00:00
        assert!(result.contains("1970"));
    }

    #[test]
    fn test_format_expires_future_timestamp() {
        // 2030-01-01 00:00:00 UTC in ms
        let ms = 1893456000000_i64;
        let result = format_expires(Some(ms));
        assert!(result.contains("2030"));
    }

    // ── provider_defaults: Claude 是 DirectAnthropic ──────────

    #[test]
    fn test_provider_type_classification() {
        assert!(matches!(
            provider_defaults(&OAuthProvider::Claude).provider_type,
            ProviderType::DirectAnthropic
        ));
        assert!(matches!(
            provider_defaults(&OAuthProvider::Openai).provider_type,
            ProviderType::OpenAIResponses
        ));
        for provider in &[
            OAuthProvider::Google,
            OAuthProvider::Qwen,
            OAuthProvider::Kimi,
            OAuthProvider::Github,
        ] {
            assert!(
                matches!(
                    provider_defaults(provider).provider_type,
                    ProviderType::OpenAICompatible
                ),
                "{:?} should be OpenAICompatible",
                provider
            );
        }
    }

    // ── account_id 注入逻辑 ──────────────────────────────────

    #[test]
    fn test_account_id_injection_from_token_extra() {
        // 模拟 ensure_valid_token 的 account_id 注入逻辑
        let token = OAuthToken {
            access_token: "test-token".to_string(),
            refresh_token: None,
            expires_at: None,
            token_type: Some("Bearer".to_string()),
            scopes: None,
            extra: Some(serde_json::json!({"auth_mode": "chatgpt", "account_id": "acc-123"})),
        };

        let mut extra_env = std::collections::HashMap::new();

        // 注入逻辑（与 ensure_valid_token 中相同）
        if let Some(account_id) = token
            .extra
            .as_ref()
            .and_then(|e| e.get("account_id"))
            .and_then(|v| v.as_str())
        {
            extra_env
                .entry("CHATGPT_ACCOUNT_ID".to_string())
                .or_insert_with(|| account_id.to_string());
        }

        assert_eq!(extra_env.get("CHATGPT_ACCOUNT_ID").unwrap(), "acc-123");
    }

    #[test]
    fn test_account_id_no_override_existing() {
        // 用户手动配置的 CHATGPT_ACCOUNT_ID 不应被覆盖
        let token = OAuthToken {
            access_token: "test-token".to_string(),
            refresh_token: None,
            expires_at: None,
            token_type: Some("Bearer".to_string()),
            scopes: None,
            extra: Some(serde_json::json!({"account_id": "new-acc"})),
        };

        let mut extra_env = std::collections::HashMap::new();
        extra_env.insert("CHATGPT_ACCOUNT_ID".to_string(), "existing-acc".to_string());

        if let Some(account_id) = token
            .extra
            .as_ref()
            .and_then(|e| e.get("account_id"))
            .and_then(|v| v.as_str())
        {
            extra_env
                .entry("CHATGPT_ACCOUNT_ID".to_string())
                .or_insert_with(|| account_id.to_string());
        }

        assert_eq!(extra_env.get("CHATGPT_ACCOUNT_ID").unwrap(), "existing-acc");
    }

    #[test]
    fn test_account_id_missing_in_token_extra() {
        let token = OAuthToken {
            access_token: "test-token".to_string(),
            refresh_token: None,
            expires_at: None,
            token_type: Some("Bearer".to_string()),
            scopes: None,
            extra: Some(serde_json::json!({"auth_mode": "api-key"})),
        };

        let mut extra_env = std::collections::HashMap::new();

        if let Some(account_id) = token
            .extra
            .as_ref()
            .and_then(|e| e.get("account_id"))
            .and_then(|v| v.as_str())
        {
            extra_env
                .entry("CHATGPT_ACCOUNT_ID".to_string())
                .or_insert_with(|| account_id.to_string());
        }

        assert!(extra_env.get("CHATGPT_ACCOUNT_ID").is_none());
    }

    #[test]
    fn test_account_id_no_extra_field() {
        let token = OAuthToken {
            access_token: "test-token".to_string(),
            refresh_token: None,
            expires_at: None,
            token_type: Some("Bearer".to_string()),
            scopes: None,
            extra: None,
        };

        let mut extra_env = std::collections::HashMap::new();

        if let Some(account_id) = token
            .extra
            .as_ref()
            .and_then(|e| e.get("account_id"))
            .and_then(|v| v.as_str())
        {
            extra_env
                .entry("CHATGPT_ACCOUNT_ID".to_string())
                .or_insert_with(|| account_id.to_string());
        }

        assert!(extra_env.get("CHATGPT_ACCOUNT_ID").is_none());
    }
}
