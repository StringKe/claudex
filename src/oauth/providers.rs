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
        OAuthProvider::Openai => ProviderDefaults {
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

// ── OAuth client IDs (public, non-secret) ──────────────────────────────
// These are public client IDs used for OAuth PKCE flows (no client secret needed)

const OPENAI_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const OPENAI_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const GITHUB_CLIENT_ID: &str = "Iv1.claudex_github";
const QWEN_CLIENT_ID: &str = "claudex-qwen";

// ── Login ───────────────────────────────────────────────────────────────

pub async fn login(
    config: &mut ClaudexConfig,
    provider_str: &str,
    profile_name: &str,
) -> Result<()> {
    let provider = OAuthProvider::from_str(provider_str).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown provider '{}'. Supported: claude, openai, google, qwen, kimi, github",
            provider_str
        )
    })?;

    ensure_oauth_profile(config, profile_name, &provider)?;

    match provider {
        OAuthProvider::Claude => login_claude(profile_name).await,
        OAuthProvider::Openai => login_openai(profile_name).await,
        OAuthProvider::Google => login_google(profile_name).await,
        OAuthProvider::Qwen => login_device_code(profile_name, &OAuthProvider::Qwen).await,
        OAuthProvider::Kimi => login_kimi(profile_name).await,
        OAuthProvider::Github => login_device_code(profile_name, &OAuthProvider::Github).await,
    }
}

/// Claude: 只读外部 credentials，不自建 OAuth
async fn login_claude(profile_name: &str) -> Result<()> {
    println!("Reading Claude credentials from ~/.claude/.credentials.json...");

    let token = super::token::read_claude_credentials()
        .context("Failed to read Claude credentials. Make sure Claude Code is installed and you have logged in with `claude` first.")?;

    super::token::store_token(profile_name, &token)?;
    println!("Claude OAuth token stored for profile '{profile_name}'.");
    println!(
        "Note: Claude subscription profiles bypass the proxy (Claude Code uses its own OAuth)."
    );
    Ok(())
}

/// OpenAI: 优先读取 Codex CLI 已有 credentials，否则提示手动设置
async fn login_openai(profile_name: &str) -> Result<()> {
    // 优先读取 Codex CLI 的 auth.json（已通过 `codex` 登录的 ChatGPT 订阅）
    match super::token::read_codex_credentials() {
        Ok(token) => {
            let auth_mode = token
                .extra
                .as_ref()
                .and_then(|e| e.get("auth_mode"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            println!("Found Codex CLI credentials (auth_mode: {auth_mode})");
            super::token::store_token(profile_name, &token)?;
            println!("OpenAI OAuth token stored for profile '{profile_name}'.");
            println!("Token will be refreshed automatically from ~/.codex/auth.json");
            return Ok(());
        }
        Err(e) => {
            tracing::debug!("Codex credentials not available: {e}");
        }
    }

    // 没有 Codex credentials，提示用户
    println!("No Codex CLI credentials found at ~/.codex/auth.json");
    println!();
    println!("To use your ChatGPT subscription with Claudex:");
    println!("  1. Install Codex CLI: npm install -g @openai/codex");
    println!("  2. Login: codex --login");
    println!("  3. Re-run: claudex auth login openai --profile {profile_name}");
    println!();
    println!("Or set OPENAI_API_KEY in your profile's extra_env for API key mode.");

    anyhow::bail!("no OpenAI credentials available")
}

/// Google: 读取 Gemini CLI 外部 credentials
async fn login_google(profile_name: &str) -> Result<()> {
    println!("Reading Google/Gemini credentials from external CLI...");
    println!(
        "Note: Google OAuth requires a registered Client ID. Using external CLI token instead."
    );

    let token = super::token::read_external_token(&OAuthProvider::Google)
        .context("Failed to read Gemini CLI credentials. Make sure Gemini CLI is installed and authenticated.")?;

    super::token::store_token(profile_name, &token)?;
    println!("Google OAuth token stored for profile '{profile_name}'.");
    Ok(())
}

/// Kimi: 读取外部 CLI credentials
async fn login_kimi(profile_name: &str) -> Result<()> {
    println!("Reading Kimi credentials from external CLI...");

    let token = super::token::read_external_token(&OAuthProvider::Kimi).context(
        "Failed to read Kimi CLI credentials. Make sure Kimi CLI is installed and authenticated.",
    )?;

    super::token::store_token(profile_name, &token)?;
    println!("Kimi OAuth token stored for profile '{profile_name}'.");
    Ok(())
}

/// Device Code Flow (GitHub, Qwen)
async fn login_device_code(profile_name: &str, provider: &OAuthProvider) -> Result<()> {
    let (device_url, token_url, client_id, scope, grant_type) = match provider {
        OAuthProvider::Github => (
            "https://github.com/login/device/code",
            "https://github.com/login/oauth/access_token",
            GITHUB_CLIENT_ID,
            "copilot",
            "urn:ietf:params:oauth:grant-type:device_code",
        ),
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

    super::token::store_token(profile_name, &token)?;
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

        let (status_str, expires_str) = match super::token::load_token(&profile.name) {
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
    match super::token::delete_token(profile_name) {
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
            // Re-read external credentials
            let token = super::token::read_claude_credentials()?;
            super::token::store_token(profile_name, &token)?;
            println!("Refreshed Claude token from ~/.claude/.credentials.json");
        }
        OAuthProvider::Google | OAuthProvider::Kimi => {
            // Re-read external credentials
            let token = super::token::read_external_token(provider)?;
            super::token::store_token(profile_name, &token)?;
            println!(
                "Refreshed {} token from external CLI",
                provider.display_name()
            );
        }
        OAuthProvider::Openai => {
            // OpenAI: 用 refresh_token 刷新，回写 auth.json
            let token = super::token::read_external_token(provider)
                .context("cannot read Codex credentials")?;
            let refresh_tok = token.refresh_token.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "no refresh_token in Codex credentials, please re-login with `codex --login`"
                )
            })?;

            let new_token = refresh_openai_token(refresh_tok, token.refresh_token.clone()).await?;
            super::token::store_token(profile_name, &new_token)?;
            println!("Token refreshed for profile '{profile_name}'.");
        }
        OAuthProvider::Qwen | OAuthProvider::Github => {
            let token =
                super::token::load_token(profile_name).context("no existing token to refresh")?;
            let refresh_token = token
                .refresh_token
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("no refresh_token available, please re-login"))?;

            let (token_url, client_id) = match provider {
                OAuthProvider::Github => (
                    "https://github.com/login/oauth/access_token",
                    GITHUB_CLIENT_ID,
                ),
                OAuthProvider::Qwen => ("https://chat.qwen.ai/api/oauth/token", QWEN_CLIENT_ID),
                _ => unreachable!(),
            };

            let client = reqwest::Client::new();
            let resp =
                super::server::refresh_access_token(&client, token_url, refresh_token, client_id)
                    .await?;

            let mut new_token = OAuthToken::from_token_response(&resp)
                .context("failed to parse refreshed token")?;

            // Preserve refresh_token if the response didn't include a new one
            if new_token.refresh_token.is_none() {
                new_token.refresh_token = token.refresh_token;
            }

            super::token::store_token(profile_name, &new_token)?;
            println!("Token refreshed for profile '{profile_name}'.");
        }
    }

    Ok(())
}

// ── Token refresh for proxy (called from handler) ───────────────────────

/// 确保 profile 的 OAuth token 有效，必要时从外部 CLI 文件重读或自动刷新。
/// 不自动访问 keyring（避免 macOS Keychain 弹窗），只读文件。
pub async fn ensure_valid_token(profile: &mut ProfileConfig) -> Result<()> {
    if profile.auth_type != AuthType::OAuth {
        return Ok(());
    }

    // api_key 已有值（可能是上次 `claudex auth login` 后写入 config 的）
    if !profile.api_key.is_empty() {
        return Ok(());
    }

    // 从外部 CLI 文件读取（无 keyring 弹窗）
    let provider = match profile.oauth_provider.as_ref() {
        Some(p) => p.clone(),
        None => anyhow::bail!("no oauth_provider for profile '{}'", profile.name),
    };

    let token = super::token::read_external_token(&provider).with_context(|| {
        format!(
            "OAuth token not available for '{}'. Run `claudex auth login {} --profile {}`",
            profile.name,
            provider.display_name().to_lowercase(),
            profile.name
        )
    })?;

    // 检查 token 是否过期（提前 60 秒刷新）
    if token.is_expired(60) {
        if provider == OAuthProvider::Openai {
            if let Some(ref refresh_tok) = token.refresh_token {
                tracing::info!(
                    "OpenAI token expired for profile '{}', refreshing...",
                    profile.name
                );
                let new_token =
                    refresh_openai_token(refresh_tok, token.refresh_token.clone()).await?;
                apply_token_to_profile(profile, &new_token);
                return Ok(());
            }
        }
        // 其他 provider 或无 refresh_token：报错
        anyhow::bail!(
            "OAuth token expired for '{}' and cannot auto-refresh. Run `claudex auth refresh {}`",
            profile.name,
            profile.name
        );
    }

    apply_token_to_profile(profile, &token);
    Ok(())
}

/// 将 token 信息注入到 profile 的 api_key 和 extra_env 中
fn apply_token_to_profile(profile: &mut ProfileConfig, token: &OAuthToken) {
    profile.api_key = token.access_token.clone();

    // 从 token extra 中注入 CHATGPT_ACCOUNT_ID（Codex 订阅需要此 header）
    if let Some(account_id) = token
        .extra
        .as_ref()
        .and_then(|e| e.get("account_id"))
        .and_then(|v| v.as_str())
    {
        profile
            .extra_env
            .entry("CHATGPT_ACCOUNT_ID".to_string())
            .or_insert_with(|| account_id.to_string());
    }
}

/// 使用 refresh_token 刷新 OpenAI access_token，并回写 ~/.codex/auth.json
async fn refresh_openai_token(
    refresh_token: &str,
    original_refresh_token: Option<String>,
) -> Result<OAuthToken> {
    let client = reqwest::Client::new();
    let resp = super::server::refresh_access_token(
        &client,
        OPENAI_TOKEN_URL,
        refresh_token,
        OPENAI_CLIENT_ID,
    )
    .await
    .context("OpenAI token refresh failed")?;

    let mut new_token =
        OAuthToken::from_token_response(&resp).context("failed to parse refreshed token")?;

    // 保留原 refresh_token（如果响应没有返回新的）
    if new_token.refresh_token.is_none() {
        new_token.refresh_token = original_refresh_token;
    }

    // 从新 access_token 的 JWT 提取 expires_at
    if new_token.expires_at.is_none() {
        new_token.expires_at = super::token::extract_jwt_exp_pub(&new_token.access_token);
    }

    // 从 id_token 提取 account_id
    let account_id = resp
        .get("id_token")
        .and_then(|v| v.as_str())
        .and_then(|id_tok| {
            super::token::extract_jwt_claim_pub(
                id_tok,
                "https://api.openai.com/auth",
                "chatgpt_account_id",
            )
        });

    let mut extra = serde_json::json!({"auth_mode": "chatgpt"});
    if let Some(ref aid) = account_id {
        extra["account_id"] = serde_json::json!(aid);
    }
    new_token.extra = Some(extra);

    // 回写 ~/.codex/auth.json
    super::token::write_codex_credentials(&new_token)?;

    tracing::info!("OpenAI token refreshed successfully");
    Ok(new_token)
}

// ── Public APIs for handler.rs ───────────────────────────────────────────

/// Device code login flow (public entry point for handler.rs)
pub async fn login_device_code_pub(provider: &OAuthProvider) -> Result<OAuthToken> {
    let (device_url, token_url, client_id, scope, grant_type) = match provider {
        OAuthProvider::Github => (
            "https://github.com/login/device/code",
            "https://github.com/login/oauth/access_token",
            GITHUB_CLIENT_ID,
            "copilot",
            "urn:ietf:params:oauth:grant-type:device_code",
        ),
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

    Ok(token)
}

/// Refresh device code token (public entry point for handler.rs)
pub async fn refresh_device_code_pub(
    provider: &OAuthProvider,
    profile_name: &str,
) -> Result<OAuthToken> {
    let token = super::token::load_token(profile_name).context("no existing token to refresh")?;
    let refresh_token = token
        .refresh_token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no refresh_token available, please re-login"))?;

    let (token_url, client_id) = match provider {
        OAuthProvider::Github => (
            "https://github.com/login/oauth/access_token",
            GITHUB_CLIENT_ID,
        ),
        OAuthProvider::Qwen => ("https://chat.qwen.ai/api/oauth/token", QWEN_CLIENT_ID),
        _ => anyhow::bail!("device code refresh not supported for {:?}", provider),
    };

    let client = reqwest::Client::new();
    let resp =
        super::server::refresh_access_token(&client, token_url, refresh_token, client_id).await?;

    let mut new_token =
        OAuthToken::from_token_response(&resp).context("failed to parse refreshed token")?;

    if new_token.refresh_token.is_none() {
        new_token.refresh_token = token.refresh_token;
    }

    Ok(new_token)
}

/// Refresh OpenAI token (public entry point for handler.rs)
pub async fn refresh_openai_token_pub(
    refresh_token: &str,
    original_refresh_token: Option<String>,
) -> Result<OAuthToken> {
    refresh_openai_token(refresh_token, original_refresh_token).await
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
