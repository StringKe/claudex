use anyhow::{Context, Result};

use super::{OAuthProvider, OAuthToken};

const KEYRING_SERVICE: &str = "claudex";

/// 生成 keyring entry 名称：`{profile}-oauth-token`
fn keyring_entry_name(profile_name: &str) -> String {
    format!("{profile_name}-oauth-token")
}

/// 存储 OAuth token 到 keyring
pub fn store_token(profile_name: &str, token: &OAuthToken) -> Result<()> {
    let entry_name = keyring_entry_name(profile_name);
    let json = serde_json::to_string(token).context("failed to serialize token")?;
    let entry = keyring::Entry::new(KEYRING_SERVICE, &entry_name)
        .context("failed to create keyring entry")?;
    entry
        .set_password(&json)
        .context("failed to store token in keyring")?;
    Ok(())
}

/// 从 keyring 加载 OAuth token
pub fn load_token(profile_name: &str) -> Result<OAuthToken> {
    let entry_name = keyring_entry_name(profile_name);
    let entry = keyring::Entry::new(KEYRING_SERVICE, &entry_name)
        .context("failed to create keyring entry")?;
    let json = entry
        .get_password()
        .context("no OAuth token found in keyring")?;
    let token: OAuthToken = serde_json::from_str(&json).context("failed to parse stored token")?;
    Ok(token)
}

/// 从 keyring 删除 OAuth token
pub fn delete_token(profile_name: &str) -> Result<()> {
    let entry_name = keyring_entry_name(profile_name);
    let entry = keyring::Entry::new(KEYRING_SERVICE, &entry_name)
        .context("failed to create keyring entry")?;
    entry
        .delete_credential()
        .context("failed to delete token from keyring")?;
    Ok(())
}

/// 读取 Claude CLI 的 credentials（~/.claude/.credentials.json）
pub fn read_claude_credentials() -> Result<OAuthToken> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    let cred_path = home.join(".claude").join(".credentials.json");
    let content = std::fs::read_to_string(&cred_path)
        .with_context(|| format!("cannot read {}", cred_path.display()))?;
    let json: serde_json::Value =
        serde_json::from_str(&content).context("invalid JSON in credentials file")?;

    // Claude credentials format: { "claudeAiOauth": { "accessToken": "...", ... } }
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

    Ok(OAuthToken {
        access_token,
        refresh_token: oauth_obj
            .get("refreshToken")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        expires_at,
        token_type: Some("Bearer".to_string()),
        scopes: None,
        extra: None,
    })
}

/// 读取 Codex CLI 的 credentials（~/.codex/auth.json）
///
/// Codex auth.json 格式:
/// ```json
/// {
///   "auth_mode": "chatgpt",
///   "tokens": {
///     "access_token": "eyJ...",
///     "refresh_token": "rt_...",
///     "id_token": "eyJ...",
///     "account_id": "..."
///   },
///   "last_refresh": "2026-02-19T12:54:57Z"
/// }
/// ```
pub fn read_codex_credentials() -> Result<OAuthToken> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    let cred_path = home.join(".codex").join("auth.json");
    let content = std::fs::read_to_string(&cred_path)
        .with_context(|| format!("cannot read {}", cred_path.display()))?;
    let json: serde_json::Value =
        serde_json::from_str(&content).context("invalid JSON in auth file")?;

    // Codex 嵌套格式: tokens.access_token
    let tokens = json.get("tokens");

    let access_token = tokens
        .and_then(|t| t.get("access_token"))
        .and_then(|v| v.as_str())
        // fallback: 顶层 access_token 或 OPENAI_API_KEY
        .or_else(|| json.get("access_token").and_then(|v| v.as_str()))
        .or_else(|| json.get("OPENAI_API_KEY").and_then(|v| v.as_str()))
        .context("no access_token found in codex auth file")?
        .to_string();

    let refresh_token = tokens
        .and_then(|t| t.get("refresh_token"))
        .and_then(|v| v.as_str())
        .or_else(|| json.get("refresh_token").and_then(|v| v.as_str()))
        .map(|s| s.to_string());

    // 从 JWT exp 字段提取过期时间
    let expires_at = extract_jwt_exp(&access_token);

    let auth_mode = json
        .get("auth_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("api-key");

    // 提取 account_id：优先从 tokens.account_id，其次从 id_token JWT claims
    let account_id = tokens
        .and_then(|t| t.get("account_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            // 从 id_token 的 JWT claims 中提取 chatgpt_account_id
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

    Ok(OAuthToken {
        access_token,
        refresh_token,
        expires_at,
        token_type: Some("Bearer".to_string()),
        scopes: None,
        extra: Some(extra),
    })
}

/// 从 JWT payload 的嵌套 namespace 中提取字段（pub 版本，供 providers.rs 调用）
pub fn extract_jwt_claim_pub(token: &str, namespace: &str, field: &str) -> Option<String> {
    extract_jwt_claim(token, namespace, field)
}

/// 从 JWT access_token 的 payload 提取 exp 字段（pub 版本，供 providers.rs 调用）
pub fn extract_jwt_exp_pub(token: &str) -> Option<i64> {
    extract_jwt_exp(token)
}

/// 从 JWT payload 的嵌套 namespace 中提取字段
/// e.g. extract_jwt_claim(token, "https://api.openai.com/auth", "chatgpt_account_id")
fn extract_jwt_claim(token: &str, namespace: &str, field: &str) -> Option<String> {
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

/// 从 JWT access_token 的 payload 提取 exp 字段（秒 → 毫秒）
fn extract_jwt_exp(token: &str) -> Option<i64> {
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

/// 将刷新后的 token 回写到 ~/.codex/auth.json，保持与 Codex CLI 兼容
pub fn write_codex_credentials(token: &OAuthToken) -> Result<()> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    let cred_path = home.join(".codex").join("auth.json");

    // 读取现有文件保留 auth_mode 等字段
    let mut json: serde_json::Value = if let Ok(content) = std::fs::read_to_string(&cred_path) {
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // 确保 tokens 对象存在
    if json.get("tokens").is_none() {
        json["tokens"] = serde_json::json!({});
    }

    let tokens = json.get_mut("tokens").unwrap();
    tokens["access_token"] = serde_json::json!(token.access_token);
    if let Some(ref rt) = token.refresh_token {
        tokens["refresh_token"] = serde_json::json!(rt);
    }

    // 更新 last_refresh 时间戳
    json["last_refresh"] = serde_json::json!(chrono::Utc::now().to_rfc3339());

    std::fs::write(&cred_path, serde_json::to_string_pretty(&json)?)?;
    tracing::info!("wrote refreshed token to {}", cred_path.display());
    Ok(())
}

/// 读取外部 CLI 的 token 文件（按 provider 分发）
pub fn read_external_token(provider: &OAuthProvider) -> Result<OAuthToken> {
    match provider {
        OAuthProvider::Claude => read_claude_credentials(),
        OAuthProvider::Openai => read_codex_credentials(),
        OAuthProvider::Google => read_gemini_credentials(),
        OAuthProvider::Kimi => read_kimi_credentials(),
        _ => anyhow::bail!("no external token reader for provider {:?}", provider),
    }
}

/// 读取 Gemini CLI 的 OAuth 缓存
fn read_gemini_credentials() -> Result<OAuthToken> {
    let home = dirs::home_dir().context("cannot determine home directory")?;

    // Gemini CLI stores credentials in ~/.gemini/oauth_creds.json or similar
    let candidates = [
        home.join(".gemini").join("oauth_creds.json"),
        home.join(".config").join("gemini").join("oauth_creds.json"),
    ];

    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let access_token = json
                    .get("access_token")
                    .or_else(|| json.get("token"))
                    .and_then(|v| v.as_str());

                if let Some(token) = access_token {
                    return Ok(OAuthToken {
                        access_token: token.to_string(),
                        refresh_token: json
                            .get("refresh_token")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        expires_at: json.get("expires_at").and_then(|v| v.as_i64()),
                        token_type: Some("Bearer".to_string()),
                        scopes: None,
                        extra: None,
                    });
                }
            }
        }
    }

    anyhow::bail!("no Gemini CLI credentials found")
}

/// 读取 Kimi CLI 的 token
fn read_kimi_credentials() -> Result<OAuthToken> {
    let home = dirs::home_dir().context("cannot determine home directory")?;

    let candidates = [
        home.join(".kimi").join("auth.json"),
        home.join(".config").join("kimi").join("auth.json"),
    ];

    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let access_token = json
                    .get("access_token")
                    .or_else(|| json.get("token"))
                    .and_then(|v| v.as_str());

                if let Some(token) = access_token {
                    return Ok(OAuthToken {
                        access_token: token.to_string(),
                        refresh_token: json
                            .get("refresh_token")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        expires_at: json.get("expires_at").and_then(|v| v.as_i64()),
                        token_type: Some("Bearer".to_string()),
                        scopes: None,
                        extra: None,
                    });
                }
            }
        }
    }

    anyhow::bail!("no Kimi CLI credentials found")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyring_entry_name() {
        assert_eq!(keyring_entry_name("chatgpt-pro"), "chatgpt-pro-oauth-token");
        assert_eq!(keyring_entry_name("claude-max"), "claude-max-oauth-token");
    }

    #[test]
    fn test_parse_claude_credentials_json() {
        let json: serde_json::Value = serde_json::json!({
            "claudeAiOauth": {
                "accessToken": "test-access-token",
                "refreshToken": "test-refresh-token",
                "expiresAt": 1700000000000_i64
            }
        });

        let oauth_obj = json.get("claudeAiOauth").unwrap();
        let access_token = oauth_obj
            .get("accessToken")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(access_token, "test-access-token");
    }

    #[test]
    fn test_parse_codex_credentials_nested_tokens() {
        let json: serde_json::Value = serde_json::json!({
            "auth_mode": "chatgpt",
            "tokens": {
                "access_token": "codex-nested-token",
                "refresh_token": "codex-refresh-456",
                "id_token": "id-token-789",
                "account_id": "acc-123"
            },
            "last_refresh": "2026-02-19T12:54:57Z"
        });

        let tokens = json.get("tokens").unwrap();
        let access_token = tokens.get("access_token").and_then(|v| v.as_str()).unwrap();
        assert_eq!(access_token, "codex-nested-token");
        assert_eq!(
            json.get("auth_mode").and_then(|v| v.as_str()),
            Some("chatgpt")
        );
    }

    #[test]
    fn test_parse_codex_credentials_flat_fallback() {
        let json: serde_json::Value = serde_json::json!({
            "access_token": "flat-token-123"
        });

        let access_token = json
            .get("tokens")
            .and_then(|t| t.get("access_token"))
            .and_then(|v| v.as_str())
            .or_else(|| json.get("access_token").and_then(|v| v.as_str()))
            .unwrap();
        assert_eq!(access_token, "flat-token-123");
    }

    #[test]
    fn test_extract_jwt_exp() {
        // 构造一个简单的 JWT: header.payload.signature
        // payload = {"exp": 1700000000}
        use base64::Engine;
        let payload = serde_json::json!({"exp": 1700000000_i64});
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&payload).unwrap());
        let fake_jwt = format!("eyJhbGciOiJub25lIn0.{payload_b64}.sig");
        let result = super::extract_jwt_exp(&fake_jwt);
        assert_eq!(result, Some(1700000000000_i64)); // 秒 → 毫秒
    }

    // ── Claude credentials 完整解析 ───────────────────────────

    #[test]
    fn test_parse_claude_credentials_with_refresh_token() {
        let json: serde_json::Value = serde_json::json!({
            "claudeAiOauth": {
                "accessToken": "access-tok",
                "refreshToken": "refresh-tok",
                "expiresAt": 1700000000000_i64
            }
        });
        let oauth_obj = json.get("claudeAiOauth").unwrap();
        assert_eq!(
            oauth_obj.get("refreshToken").and_then(|v| v.as_str()),
            Some("refresh-tok")
        );
        assert_eq!(
            oauth_obj.get("expiresAt").and_then(|v| v.as_i64()),
            Some(1700000000000)
        );
    }

    #[test]
    fn test_parse_claude_credentials_string_expires_at() {
        // expiresAt 作为字符串（有些版本的 credentials 是字符串）
        let json: serde_json::Value = serde_json::json!({
            "claudeAiOauth": {
                "accessToken": "access-tok",
                "expiresAt": "1700000000000"
            }
        });
        let oauth_obj = json.get("claudeAiOauth").unwrap();
        // i64 直接取会失败
        let expires_at = oauth_obj
            .get("expiresAt")
            .and_then(|v| v.as_i64())
            .or_else(|| {
                oauth_obj
                    .get("expiresAt")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<i64>().ok())
            });
        assert_eq!(expires_at, Some(1700000000000));
    }

    #[test]
    fn test_parse_claude_credentials_missing_oauth_key() {
        let json: serde_json::Value = serde_json::json!({
            "someOtherKey": {}
        });
        assert!(json.get("claudeAiOauth").is_none());
    }

    #[test]
    fn test_parse_claude_credentials_missing_access_token() {
        let json: serde_json::Value = serde_json::json!({
            "claudeAiOauth": {
                "refreshToken": "ref"
            }
        });
        let oauth_obj = json.get("claudeAiOauth").unwrap();
        let access_token = oauth_obj.get("accessToken").and_then(|v| v.as_str());
        assert!(access_token.is_none());
    }

    // ── Codex credentials 兜底字段 ────────────────────────────

    #[test]
    fn test_parse_codex_credentials_alt_field_name() {
        // Codex 有时用 "token" 而不是 "access_token"
        let json: serde_json::Value = serde_json::json!({
            "token": "alt-token-789"
        });
        let access_token = json
            .get("access_token")
            .or_else(|| json.get("token"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(access_token, "alt-token-789");
    }

    #[test]
    fn test_parse_codex_credentials_no_token_field() {
        let json: serde_json::Value = serde_json::json!({
            "unrelated": "data"
        });
        let access_token = json
            .get("access_token")
            .or_else(|| json.get("token"))
            .and_then(|v| v.as_str());
        assert!(access_token.is_none());
    }

    // ── read_external_token dispatch ──────────────────────────

    #[test]
    fn test_read_external_token_unsupported_provider_qwen() {
        // Qwen 没有外部 CLI token 读取
        let result = read_external_token(&OAuthProvider::Qwen);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("no external token reader"));
    }

    #[test]
    fn test_read_external_token_unsupported_provider_github() {
        let result = read_external_token(&OAuthProvider::Github);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("no external token reader"));
    }

    // ── Keyring entry name 边界 ───────────────────────────────

    #[test]
    fn test_keyring_entry_name_special_chars() {
        assert_eq!(
            keyring_entry_name("my-profile_123"),
            "my-profile_123-oauth-token"
        );
    }

    #[test]
    fn test_keyring_entry_name_empty() {
        assert_eq!(keyring_entry_name(""), "-oauth-token");
    }
}
