pub mod providers;
pub mod server;
pub mod token;

use serde::{Deserialize, Serialize};

/// 认证方式
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum AuthType {
    #[default]
    ApiKey,
    #[serde(rename = "oauth")]
    OAuth,
}

/// OAuth 提供商
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum OAuthProvider {
    Claude,
    Openai,
    Google,
    Qwen,
    Kimi,
    Github,
}

impl OAuthProvider {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "claude" => Some(Self::Claude),
            "openai" => Some(Self::Openai),
            "google" | "gemini" => Some(Self::Google),
            "qwen" => Some(Self::Qwen),
            "kimi" | "moonshot" => Some(Self::Kimi),
            "github" | "copilot" => Some(Self::Github),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Openai => "OpenAI",
            Self::Google => "Google",
            Self::Qwen => "Qwen",
            Self::Kimi => "Kimi",
            Self::Github => "GitHub",
        }
    }
}

/// 存储在 keyring 中的 token（JSON 序列化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix 毫秒时间戳
    pub expires_at: Option<i64>,
    pub token_type: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub extra: Option<serde_json::Value>,
}

impl OAuthToken {
    /// 检查 token 是否已过期（含提前 buffer 秒数）
    pub fn is_expired(&self, buffer_secs: i64) -> bool {
        match self.expires_at {
            Some(expires_at) => {
                let now_ms = chrono::Utc::now().timestamp_millis();
                now_ms >= (expires_at - buffer_secs * 1000)
            }
            None => false, // 无过期时间视为不过期
        }
    }

    /// 从标准 OAuth token response JSON 解析
    pub fn from_token_response(json: &serde_json::Value) -> Option<Self> {
        let access_token = json.get("access_token")?.as_str()?.to_string();
        let refresh_token = json
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let expires_at = json.get("expires_in").and_then(|v| v.as_i64()).map(|secs| {
            chrono::Utc::now().timestamp_millis() + secs * 1000
        });

        let token_type = json
            .get("token_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let scopes = json.get("scope").and_then(|v| v.as_str()).map(|s| {
            s.split_whitespace().map(|s| s.to_string()).collect()
        });

        Some(Self {
            access_token,
            refresh_token,
            expires_at,
            token_type,
            scopes,
            extra: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_token_not_expired() {
        let token = OAuthToken {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: Some(chrono::Utc::now().timestamp_millis() + 3600_000),
            token_type: None,
            scopes: None,
            extra: None,
        };
        assert!(!token.is_expired(0));
        assert!(!token.is_expired(300));
    }

    #[test]
    fn test_oauth_token_expired() {
        let token = OAuthToken {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: Some(chrono::Utc::now().timestamp_millis() - 1000),
            token_type: None,
            scopes: None,
            extra: None,
        };
        assert!(token.is_expired(0));
    }

    #[test]
    fn test_oauth_token_expired_with_buffer() {
        let token = OAuthToken {
            access_token: "test".to_string(),
            refresh_token: None,
            // Expires in 200 seconds
            expires_at: Some(chrono::Utc::now().timestamp_millis() + 200_000),
            token_type: None,
            scopes: None,
            extra: None,
        };
        // Not expired with no buffer
        assert!(!token.is_expired(0));
        // Expired with 5-minute buffer (300s > 200s remaining)
        assert!(token.is_expired(300));
    }

    #[test]
    fn test_oauth_token_no_expiry() {
        let token = OAuthToken {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: None,
            token_type: None,
            scopes: None,
            extra: None,
        };
        assert!(!token.is_expired(0));
        assert!(!token.is_expired(9999));
    }

    #[test]
    fn test_parse_token_response() {
        let json = serde_json::json!({
            "access_token": "abc123",
            "refresh_token": "ref456",
            "expires_in": 3600,
            "token_type": "Bearer",
            "scope": "openid profile"
        });
        let token = OAuthToken::from_token_response(&json).unwrap();
        assert_eq!(token.access_token, "abc123");
        assert_eq!(token.refresh_token.as_deref(), Some("ref456"));
        assert!(token.expires_at.is_some());
        assert_eq!(token.token_type.as_deref(), Some("Bearer"));
        assert_eq!(
            token.scopes,
            Some(vec!["openid".to_string(), "profile".to_string()])
        );
    }

    #[test]
    fn test_parse_token_response_minimal() {
        let json = serde_json::json!({
            "access_token": "abc123"
        });
        let token = OAuthToken::from_token_response(&json).unwrap();
        assert_eq!(token.access_token, "abc123");
        assert!(token.refresh_token.is_none());
        assert!(token.expires_at.is_none());
    }

    #[test]
    fn test_parse_provider_str() {
        assert_eq!(OAuthProvider::from_str("claude"), Some(OAuthProvider::Claude));
        assert_eq!(OAuthProvider::from_str("openai"), Some(OAuthProvider::Openai));
        assert_eq!(OAuthProvider::from_str("google"), Some(OAuthProvider::Google));
        assert_eq!(OAuthProvider::from_str("gemini"), Some(OAuthProvider::Google));
        assert_eq!(OAuthProvider::from_str("qwen"), Some(OAuthProvider::Qwen));
        assert_eq!(OAuthProvider::from_str("kimi"), Some(OAuthProvider::Kimi));
        assert_eq!(OAuthProvider::from_str("moonshot"), Some(OAuthProvider::Kimi));
        assert_eq!(OAuthProvider::from_str("github"), Some(OAuthProvider::Github));
        assert_eq!(OAuthProvider::from_str("copilot"), Some(OAuthProvider::Github));
        assert_eq!(OAuthProvider::from_str("unknown"), None);
    }

    #[test]
    fn test_auth_type_default_is_api_key() {
        assert_eq!(AuthType::default(), AuthType::ApiKey);
    }

    #[test]
    fn test_auth_type_serde_roundtrip() {
        let oauth = AuthType::OAuth;
        let json = serde_json::to_string(&oauth).unwrap();
        assert_eq!(json, r#""oauth""#);
        let parsed: AuthType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, AuthType::OAuth);

        let api_key = AuthType::ApiKey;
        let json = serde_json::to_string(&api_key).unwrap();
        assert_eq!(json, r#""api-key""#);
        let parsed: AuthType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, AuthType::ApiKey);
    }

    // ── from_token_response 边界 ──────────────────────────────

    #[test]
    fn test_parse_token_response_missing_access_token() {
        let json = serde_json::json!({
            "refresh_token": "ref456",
            "expires_in": 3600
        });
        assert!(OAuthToken::from_token_response(&json).is_none());
    }

    #[test]
    fn test_parse_token_response_access_token_not_string() {
        let json = serde_json::json!({
            "access_token": 12345
        });
        assert!(OAuthToken::from_token_response(&json).is_none());
    }

    #[test]
    fn test_parse_token_response_empty_object() {
        let json = serde_json::json!({});
        assert!(OAuthToken::from_token_response(&json).is_none());
    }

    #[test]
    fn test_parse_token_response_expires_in_computed_correctly() {
        let before = chrono::Utc::now().timestamp_millis();
        let json = serde_json::json!({
            "access_token": "tok",
            "expires_in": 7200
        });
        let token = OAuthToken::from_token_response(&json).unwrap();
        let after = chrono::Utc::now().timestamp_millis();

        let expires = token.expires_at.unwrap();
        // expires_at should be ~now + 7200*1000
        assert!(expires >= before + 7200_000);
        assert!(expires <= after + 7200_000);
    }

    #[test]
    fn test_parse_token_response_single_scope() {
        let json = serde_json::json!({
            "access_token": "tok",
            "scope": "read"
        });
        let token = OAuthToken::from_token_response(&json).unwrap();
        assert_eq!(token.scopes, Some(vec!["read".to_string()]));
    }

    // ── OAuthToken serde roundtrip ────────────────────────────

    #[test]
    fn test_oauth_token_json_roundtrip() {
        let token = OAuthToken {
            access_token: "access-123".to_string(),
            refresh_token: Some("refresh-456".to_string()),
            expires_at: Some(1700000000000),
            token_type: Some("Bearer".to_string()),
            scopes: Some(vec!["openid".to_string(), "profile".to_string()]),
            extra: Some(serde_json::json!({"foo": "bar"})),
        };
        let json = serde_json::to_string(&token).unwrap();
        let parsed: OAuthToken = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, "access-123");
        assert_eq!(parsed.refresh_token.as_deref(), Some("refresh-456"));
        assert_eq!(parsed.expires_at, Some(1700000000000));
        assert_eq!(parsed.token_type.as_deref(), Some("Bearer"));
        assert_eq!(parsed.scopes.as_ref().unwrap().len(), 2);
        assert!(parsed.extra.is_some());
    }

    #[test]
    fn test_oauth_token_json_roundtrip_minimal() {
        let token = OAuthToken {
            access_token: "tok".to_string(),
            refresh_token: None,
            expires_at: None,
            token_type: None,
            scopes: None,
            extra: None,
        };
        let json = serde_json::to_string(&token).unwrap();
        let parsed: OAuthToken = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, "tok");
        assert!(parsed.refresh_token.is_none());
    }

    // ── OAuthProvider serde ───────────────────────────────────

    #[test]
    fn test_oauth_provider_serde_roundtrip() {
        let cases = vec![
            (OAuthProvider::Claude, "\"claude\""),
            (OAuthProvider::Openai, "\"openai\""),
            (OAuthProvider::Google, "\"google\""),
            (OAuthProvider::Qwen, "\"qwen\""),
            (OAuthProvider::Kimi, "\"kimi\""),
            (OAuthProvider::Github, "\"github\""),
        ];
        for (variant, expected_json) in cases {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected_json, "serialize {:?}", variant);
            let parsed: OAuthProvider = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, variant, "deserialize {expected_json}");
        }
    }

    // ── OAuthProvider::display_name ───────────────────────────

    #[test]
    fn test_oauth_provider_display_names() {
        assert_eq!(OAuthProvider::Claude.display_name(), "Claude");
        assert_eq!(OAuthProvider::Openai.display_name(), "OpenAI");
        assert_eq!(OAuthProvider::Google.display_name(), "Google");
        assert_eq!(OAuthProvider::Qwen.display_name(), "Qwen");
        assert_eq!(OAuthProvider::Kimi.display_name(), "Kimi");
        assert_eq!(OAuthProvider::Github.display_name(), "GitHub");
    }

    // ── OAuthProvider::from_str 大小写 ────────────────────────

    #[test]
    fn test_parse_provider_str_case_insensitive() {
        assert_eq!(OAuthProvider::from_str("Claude"), Some(OAuthProvider::Claude));
        assert_eq!(OAuthProvider::from_str("OPENAI"), Some(OAuthProvider::Openai));
        assert_eq!(OAuthProvider::from_str("GitHub"), Some(OAuthProvider::Github));
        assert_eq!(OAuthProvider::from_str("GEMINI"), Some(OAuthProvider::Google));
        assert_eq!(OAuthProvider::from_str("Moonshot"), Some(OAuthProvider::Kimi));
        assert_eq!(OAuthProvider::from_str("Copilot"), Some(OAuthProvider::Github));
    }

    #[test]
    fn test_parse_provider_str_empty() {
        assert_eq!(OAuthProvider::from_str(""), None);
    }

    // ── is_expired 边界条件 ───────────────────────────────────

    #[test]
    fn test_oauth_token_expired_exactly_at_boundary() {
        let now = chrono::Utc::now().timestamp_millis();
        let token = OAuthToken {
            access_token: "test".to_string(),
            refresh_token: None,
            // Expires exactly now
            expires_at: Some(now),
            token_type: None,
            scopes: None,
            extra: None,
        };
        // now >= now → expired
        assert!(token.is_expired(0));
    }

    #[test]
    fn test_oauth_token_negative_buffer_extends_window() {
        let token = OAuthToken {
            access_token: "test".to_string(),
            refresh_token: None,
            // Expired 10 seconds ago
            expires_at: Some(chrono::Utc::now().timestamp_millis() - 10_000),
            token_type: None,
            scopes: None,
            extra: None,
        };
        // With -60s buffer, it effectively adds 60s to the expiry → not expired
        assert!(!token.is_expired(-60));
    }
}
