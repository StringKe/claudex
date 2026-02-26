//! Layer 1: Token Sources
//!
//! 统一凭证读取层，支持多种来源: 环境变量、config、外部 CLI 文件、keyring、Copilot config。

use anyhow::{Context, Result};

use super::{OAuthProvider, OAuthToken};

// ── Types ────────────────────────────────────────────────────────────────

/// 凭证来源标识
#[derive(Debug, Clone)]
pub enum CredentialSource {
    EnvVar(String),
    ConfigApiKey,
    ExternalCli(String),
    Keyring,
    CopilotConfig,
}

/// 原始凭证（从某来源读取、尚未经过 exchange 处理）
#[derive(Debug, Clone)]
pub struct RawCredential {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub token_type: Option<String>,
    pub extra: Option<serde_json::Value>,
    pub source: CredentialSource,
}

impl RawCredential {
    pub fn into_oauth_token(self) -> OAuthToken {
        OAuthToken {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            expires_at: self.expires_at,
            token_type: self.token_type,
            scopes: None,
            extra: self.extra,
        }
    }
}

// ── Keyring ──────────────────────────────────────────────────────────────

const KEYRING_SERVICE: &str = "claudex";

fn keyring_entry_name(profile_name: &str) -> String {
    format!("{profile_name}-oauth-token")
}

pub fn store_keyring(profile_name: &str, token: &OAuthToken) -> Result<()> {
    let entry_name = keyring_entry_name(profile_name);
    let json = serde_json::to_string(token).context("failed to serialize token")?;
    let entry = keyring::Entry::new(KEYRING_SERVICE, &entry_name)
        .context("failed to create keyring entry")?;
    entry
        .set_password(&json)
        .context("failed to store token in keyring")?;
    Ok(())
}

pub fn load_keyring(profile_name: &str) -> Result<OAuthToken> {
    let entry_name = keyring_entry_name(profile_name);
    let entry = keyring::Entry::new(KEYRING_SERVICE, &entry_name)
        .context("failed to create keyring entry")?;
    let json = entry
        .get_password()
        .context("no OAuth token found in keyring")?;
    let token: OAuthToken = serde_json::from_str(&json).context("failed to parse stored token")?;
    Ok(token)
}

pub fn delete_keyring(profile_name: &str) -> Result<()> {
    let entry_name = keyring_entry_name(profile_name);
    let entry = keyring::Entry::new(KEYRING_SERVICE, &entry_name)
        .context("failed to create keyring entry")?;
    entry
        .delete_credential()
        .context("failed to delete token from keyring")?;
    Ok(())
}

// ── External CLI Readers ─────────────────────────────────────────────────

/// 读取 Claude CLI 的 credentials（~/.claude/.credentials.json）
pub fn read_claude_credentials() -> Result<RawCredential> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    let cred_path = home.join(".claude").join(".credentials.json");
    let content = std::fs::read_to_string(&cred_path)
        .with_context(|| format!("cannot read {}", cred_path.display()))?;
    let json: serde_json::Value =
        serde_json::from_str(&content).context("invalid JSON in credentials file")?;

    let oauth_obj = json
        .get("claudeAiOauth")
        .context("missing 'claudeAiOauth' in credentials")?;

    let access_token = oauth_obj
        .get("accessToken")
        .and_then(|v| v.as_str())
        .context("missing 'accessToken' in claudeAiOauth")?
        .to_string();

    let expires_at = oauth_obj
        .get("expiresAt")
        .and_then(|v| v.as_i64())
        .or_else(|| {
            oauth_obj
                .get("expiresAt")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
        });

    Ok(RawCredential {
        access_token,
        refresh_token: oauth_obj
            .get("refreshToken")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        expires_at,
        token_type: Some("Bearer".to_string()),
        extra: None,
        source: CredentialSource::ExternalCli("~/.claude/.credentials.json".to_string()),
    })
}

/// 读取 Codex CLI 的 credentials（~/.codex/auth.json）
pub fn read_codex_credentials() -> Result<RawCredential> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    let cred_path = home.join(".codex").join("auth.json");
    let content = std::fs::read_to_string(&cred_path)
        .with_context(|| format!("cannot read {}", cred_path.display()))?;
    let json: serde_json::Value =
        serde_json::from_str(&content).context("invalid JSON in auth file")?;

    let tokens = json.get("tokens");

    let access_token = tokens
        .and_then(|t| t.get("access_token"))
        .and_then(|v| v.as_str())
        .or_else(|| json.get("access_token").and_then(|v| v.as_str()))
        .or_else(|| json.get("OPENAI_API_KEY").and_then(|v| v.as_str()))
        .context("no access_token found in codex auth file")?
        .to_string();

    let refresh_token = tokens
        .and_then(|t| t.get("refresh_token"))
        .and_then(|v| v.as_str())
        .or_else(|| json.get("refresh_token").and_then(|v| v.as_str()))
        .map(|s| s.to_string());

    let expires_at = extract_jwt_exp(&access_token);

    let auth_mode = json
        .get("auth_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("api-key");

    // 提取 account_id
    let account_id = tokens
        .and_then(|t| t.get("account_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            let id_token = tokens
                .and_then(|t| t.get("id_token"))
                .and_then(|v| v.as_str())?;
            extract_jwt_claim(
                id_token,
                "https://api.openai.com/auth",
                "chatgpt_account_id",
            )
        });

    let mut extra = serde_json::json!({ "auth_mode": auth_mode });
    if let Some(ref aid) = account_id {
        extra["account_id"] = serde_json::json!(aid);
    }

    Ok(RawCredential {
        access_token,
        refresh_token,
        expires_at,
        token_type: Some("Bearer".to_string()),
        extra: Some(extra),
        source: CredentialSource::ExternalCli("~/.codex/auth.json".to_string()),
    })
}

/// 读取 GitHub Copilot 的已有配置
/// 支持 ~/.config/github-copilot/hosts.json 和 apps.json
/// enterprise_host: 可选企业版 host (如 "company.ghe.com")，用于搜索 apps.json
pub fn read_copilot_config() -> Result<RawCredential> {
    read_copilot_config_with_host(None)
}

pub fn read_copilot_config_with_host(enterprise_host: Option<&str>) -> Result<RawCredential> {
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".config"));
    let copilot_dir = config_dir.join("github-copilot");

    let host_pattern = enterprise_host.unwrap_or("github.com");

    // 优先尝试 apps.json (key 格式: "github.com:CLIENT_ID" 或 "enterprise.ghe.com:CLIENT_ID")
    let apps_path = copilot_dir.join("apps.json");
    if let Ok(content) = std::fs::read_to_string(&apps_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json.as_object() {
                for (key, value) in obj {
                    if key.contains(host_pattern) {
                        if let Some(token) = value.get("oauth_token").and_then(|v| v.as_str()) {
                            return Ok(RawCredential {
                                access_token: token.to_string(),
                                refresh_token: None,
                                expires_at: None,
                                token_type: Some("token".to_string()),
                                extra: Some(serde_json::json!({"source_key": key})),
                                source: CredentialSource::CopilotConfig,
                            });
                        }
                    }
                }
            }
        }
    }

    // 回退到 hosts.json (格式: {"github.com": {"oauth_token": "gho_xxx"}})
    let hosts_path = copilot_dir.join("hosts.json");
    if let Ok(content) = std::fs::read_to_string(&hosts_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json.as_object() {
                for (key, value) in obj {
                    if key.contains(host_pattern) {
                        if let Some(token) = value.get("oauth_token").and_then(|v| v.as_str()) {
                            return Ok(RawCredential {
                                access_token: token.to_string(),
                                refresh_token: None,
                                expires_at: None,
                                token_type: Some("token".to_string()),
                                extra: None,
                                source: CredentialSource::CopilotConfig,
                            });
                        }
                    }
                }
            }
        }
    }

    anyhow::bail!(
        "no GitHub Copilot credentials found in {}",
        copilot_dir.display()
    )
}

/// 读取 Gemini CLI 的 credentials
pub fn read_gemini_credentials() -> Result<RawCredential> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    let candidates = [
        home.join(".gemini").join("oauth_creds.json"),
        home.join(".config").join("gemini").join("oauth_creds.json"),
    ];
    read_cli_credentials(&candidates, "Gemini")
}

/// 读取 Kimi CLI 的 credentials
pub fn read_kimi_credentials() -> Result<RawCredential> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    let candidates = [
        home.join(".kimi").join("auth.json"),
        home.join(".config").join("kimi").join("auth.json"),
    ];
    read_cli_credentials(&candidates, "Kimi")
}

/// 通用 CLI credentials 读取器
fn read_cli_credentials(
    candidates: &[std::path::PathBuf],
    provider: &str,
) -> Result<RawCredential> {
    for path in candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let access_token = json
                    .get("access_token")
                    .or_else(|| json.get("token"))
                    .and_then(|v| v.as_str());

                if let Some(token) = access_token {
                    return Ok(RawCredential {
                        access_token: token.to_string(),
                        refresh_token: json
                            .get("refresh_token")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        expires_at: json.get("expires_at").and_then(|v| v.as_i64()),
                        token_type: Some("Bearer".to_string()),
                        extra: None,
                        source: CredentialSource::ExternalCli(path.display().to_string()),
                    });
                }
            }
        }
    }

    anyhow::bail!("no {provider} CLI credentials found")
}

// ── Credential Chain ─────────────────────────────────────────────────────

/// 多源 fallback 链: 按优先级尝试不同来源加载凭证
pub fn load_credential_chain(provider: &OAuthProvider) -> Result<RawCredential> {
    // normalize: Openai -> Chatgpt
    let provider = provider.normalize();

    match provider {
        OAuthProvider::Claude => {
            // env ANTHROPIC_API_KEY > ~/.claude/.credentials.json > keyring
            if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
                if !key.is_empty() {
                    return Ok(RawCredential {
                        access_token: key,
                        refresh_token: None,
                        expires_at: None,
                        token_type: Some("Bearer".to_string()),
                        extra: None,
                        source: CredentialSource::EnvVar("ANTHROPIC_API_KEY".to_string()),
                    });
                }
            }
            read_claude_credentials()
        }
        OAuthProvider::Chatgpt => {
            // env CODEX_API_KEY > ~/.codex/auth.json > keyring
            if let Ok(key) = std::env::var("CODEX_API_KEY") {
                if !key.is_empty() {
                    return Ok(RawCredential {
                        access_token: key,
                        refresh_token: None,
                        expires_at: None,
                        token_type: Some("Bearer".to_string()),
                        extra: None,
                        source: CredentialSource::EnvVar("CODEX_API_KEY".to_string()),
                    });
                }
            }
            read_codex_credentials()
        }
        OAuthProvider::Google => {
            // env GEMINI_API_KEY > ~/.gemini/oauth_creds.json
            if let Ok(key) = std::env::var("GEMINI_API_KEY") {
                if !key.is_empty() {
                    return Ok(RawCredential {
                        access_token: key,
                        refresh_token: None,
                        expires_at: None,
                        token_type: Some("Bearer".to_string()),
                        extra: None,
                        source: CredentialSource::EnvVar("GEMINI_API_KEY".to_string()),
                    });
                }
            }
            read_gemini_credentials()
        }
        OAuthProvider::Kimi => {
            // env KIMI_API_KEY > ~/.kimi/auth.json
            if let Ok(key) = std::env::var("KIMI_API_KEY") {
                if !key.is_empty() {
                    return Ok(RawCredential {
                        access_token: key,
                        refresh_token: None,
                        expires_at: None,
                        token_type: Some("Bearer".to_string()),
                        extra: None,
                        source: CredentialSource::EnvVar("KIMI_API_KEY".to_string()),
                    });
                }
            }
            read_kimi_credentials()
        }
        OAuthProvider::Github => {
            // env GITHUB_TOKEN > ~/.config/github-copilot/apps.json > hosts.json
            if let Ok(key) = std::env::var("GITHUB_TOKEN") {
                if !key.is_empty() {
                    return Ok(RawCredential {
                        access_token: key,
                        refresh_token: None,
                        expires_at: None,
                        token_type: Some("token".to_string()),
                        extra: None,
                        source: CredentialSource::EnvVar("GITHUB_TOKEN".to_string()),
                    });
                }
            }
            read_copilot_config()
        }
        OAuthProvider::Gitlab => {
            // env GITLAB_TOKEN > GL_TOKEN
            for var in &["GITLAB_TOKEN", "GL_TOKEN"] {
                if let Ok(key) = std::env::var(var) {
                    if !key.is_empty() {
                        return Ok(RawCredential {
                            access_token: key,
                            refresh_token: None,
                            expires_at: None,
                            token_type: Some("Bearer".to_string()),
                            extra: None,
                            source: CredentialSource::EnvVar(var.to_string()),
                        });
                    }
                }
            }
            anyhow::bail!("no GitLab token found. Set GITLAB_TOKEN or GL_TOKEN environment variable, or run `claudex auth login gitlab`")
        }
        OAuthProvider::Qwen => {
            anyhow::bail!("Qwen does not support credential chain loading; use config api_key or device code login")
        }
        // Openai 已被 normalize() 映射到 Chatgpt，此处不可达
        OAuthProvider::Openai => unreachable!("Openai normalized to Chatgpt"),
    }
}

// ── JWT Utilities ────────────────────────────────────────────────────────

/// 从 JWT payload 提取 exp 字段（秒 -> 毫秒）
pub fn extract_jwt_exp(token: &str) -> Option<i64> {
    use base64::Engine;

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    let json: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    json.get("exp").and_then(|v| v.as_i64()).map(|s| s * 1000)
}

/// 从 JWT payload 的嵌套 namespace 中提取字段
pub fn extract_jwt_claim(token: &str, namespace: &str, field: &str) -> Option<String> {
    use base64::Engine;

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    let json: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    json.get(namespace)
        .and_then(|ns| ns.get(field))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// 从 id_token 或 access_token 中提取 ChatGPT account_id
pub fn extract_account_id(token_response: &serde_json::Value) -> Option<String> {
    // 优先从 id_token 提取
    if let Some(id_token) = token_response.get("id_token").and_then(|v| v.as_str()) {
        if let Some(aid) = extract_jwt_claim(
            id_token,
            "https://api.openai.com/auth",
            "chatgpt_account_id",
        ) {
            return Some(aid);
        }
    }
    // 回退到 access_token
    if let Some(access_token) = token_response.get("access_token").and_then(|v| v.as_str()) {
        if let Some(aid) = extract_jwt_claim(
            access_token,
            "https://api.openai.com/auth",
            "chatgpt_account_id",
        ) {
            return Some(aid);
        }
    }
    None
}

// ── Codex credentials atomic write ───────────────────────────────────────

/// 将刷新后的 token 原子写入 ~/.codex/auth.json
pub fn write_codex_credentials_atomic(token: &OAuthToken) -> Result<()> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    let codex_dir = home.join(".codex");
    let cred_path = codex_dir.join("auth.json");

    // 读取现有文件保留 auth_mode 等字段
    let mut json: serde_json::Value = if let Ok(content) = std::fs::read_to_string(&cred_path) {
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if json.get("tokens").is_none() {
        json["tokens"] = serde_json::json!({});
    }

    let tokens = json.get_mut("tokens").unwrap();
    tokens["access_token"] = serde_json::json!(token.access_token);
    if let Some(ref rt) = token.refresh_token {
        tokens["refresh_token"] = serde_json::json!(rt);
    }

    json["last_refresh"] = serde_json::json!(chrono::Utc::now().to_rfc3339());

    // 原子写入: tmp 文件 + rename
    std::fs::create_dir_all(&codex_dir)?;
    let tmp_path = cred_path.with_extension("tmp");
    std::fs::write(&tmp_path, serde_json::to_string_pretty(&json)?)?;
    std::fs::rename(&tmp_path, &cred_path)?;

    tracing::info!("wrote refreshed token to {}", cred_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_jwt_exp() {
        use base64::Engine;
        let payload = serde_json::json!({"exp": 1700000000_i64});
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&payload).unwrap());
        let fake_jwt = format!("eyJhbGciOiJub25lIn0.{payload_b64}.sig");
        assert_eq!(extract_jwt_exp(&fake_jwt), Some(1700000000000_i64));
    }

    #[test]
    fn test_extract_jwt_exp_invalid_token() {
        assert_eq!(extract_jwt_exp("not-a-jwt"), None);
        assert_eq!(extract_jwt_exp("a.b"), None);
    }

    #[test]
    fn test_extract_jwt_claim() {
        use base64::Engine;
        let payload = serde_json::json!({
            "https://api.openai.com/auth": {
                "chatgpt_account_id": "acc-123"
            }
        });
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&payload).unwrap());
        let fake_jwt = format!("eyJhbGciOiJub25lIn0.{payload_b64}.sig");
        assert_eq!(
            extract_jwt_claim(
                &fake_jwt,
                "https://api.openai.com/auth",
                "chatgpt_account_id"
            ),
            Some("acc-123".to_string())
        );
    }

    #[test]
    fn test_extract_account_id_from_id_token() {
        use base64::Engine;
        let payload = serde_json::json!({
            "https://api.openai.com/auth": {
                "chatgpt_account_id": "id-tok-acc"
            }
        });
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&payload).unwrap());
        let fake_jwt = format!("eyJhbGciOiJub25lIn0.{payload_b64}.sig");
        let resp = serde_json::json!({
            "access_token": "opaque",
            "id_token": fake_jwt,
        });
        assert_eq!(extract_account_id(&resp), Some("id-tok-acc".to_string()));
    }

    #[test]
    fn test_credential_chain_env_var() {
        // 测试 env var 优先级
        std::env::set_var("ANTHROPIC_API_KEY", "test-key-123");
        let cred = load_credential_chain(&OAuthProvider::Claude).unwrap();
        assert_eq!(cred.access_token, "test-key-123");
        assert!(matches!(cred.source, CredentialSource::EnvVar(_)));
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_credential_chain_empty_env_skipped() {
        std::env::set_var("ANTHROPIC_API_KEY", "");
        // 空值应被跳过，如果文件也不存在则报错
        let result = load_credential_chain(&OAuthProvider::Claude);
        // 在 CI 中文件不存在，应该报错
        // 关键是不会因为空 env var 返回空 token
        if let Ok(cred) = &result {
            assert!(!cred.access_token.is_empty());
        }
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_normalize_openai_to_chatgpt() {
        assert_eq!(OAuthProvider::Openai.normalize(), OAuthProvider::Chatgpt);
        assert_eq!(OAuthProvider::Claude.normalize(), OAuthProvider::Claude);
        assert_eq!(OAuthProvider::Github.normalize(), OAuthProvider::Github);
        assert_eq!(OAuthProvider::Chatgpt.normalize(), OAuthProvider::Chatgpt);
    }

    #[test]
    fn test_raw_credential_into_oauth_token() {
        let cred = RawCredential {
            access_token: "tok".to_string(),
            refresh_token: Some("ref".to_string()),
            expires_at: Some(1700000000000),
            token_type: Some("Bearer".to_string()),
            extra: None,
            source: CredentialSource::EnvVar("TEST".to_string()),
        };
        let token = cred.into_oauth_token();
        assert_eq!(token.access_token, "tok");
        assert_eq!(token.refresh_token.as_deref(), Some("ref"));
        assert_eq!(token.expires_at, Some(1700000000000));
    }

    #[test]
    fn test_keyring_entry_name() {
        assert_eq!(keyring_entry_name("chatgpt-pro"), "chatgpt-pro-oauth-token");
    }
}
