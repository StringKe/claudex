//! Layer 3: Token Manager
//!
//! proxy handler 的唯一 token 入口。提供缓存、并发刷新去重、401 重试触发。

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::Mutex;

use crate::config::ProfileConfig;
use crate::oauth::{AuthType, OAuthProvider, OAuthToken};

/// 缓存中的 token
struct CachedToken {
    token: OAuthToken,
    /// 缓存写入时间 (用于 invalidate)
    cached_at: i64,
}

pub struct TokenManager {
    cache: Arc<Mutex<HashMap<String, CachedToken>>>,
    /// per-profile 刷新锁，防止并发刷新
    refresh_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    http_client: reqwest::Client,
}

impl TokenManager {
    pub fn new(http_client: reqwest::Client) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            refresh_locks: Arc::new(Mutex::new(HashMap::new())),
            http_client,
        }
    }

    /// 获取 token，优先从缓存返回，过期时自动刷新
    pub async fn get_token(&self, profile: &ProfileConfig) -> Result<OAuthToken> {
        if profile.auth_type != AuthType::OAuth {
            anyhow::bail!("profile '{}' is not OAuth", profile.name);
        }

        let provider = profile
            .oauth_provider
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no oauth_provider for profile '{}'", profile.name))?
            .normalize();

        // 快速路径: 缓存命中且未过期
        {
            let cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&profile.name) {
                if !cached.token.is_expired(60) {
                    return Ok(cached.token.clone());
                }
            }
        }

        // 慢速路径: 获取 per-profile 刷新锁
        let refresh_lock = {
            let mut locks = self.refresh_locks.lock().await;
            locks
                .entry(profile.name.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };

        let _guard = refresh_lock.lock().await;

        // Double-check: 另一个并发请求可能已经刷新了
        {
            let cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&profile.name) {
                if !cached.token.is_expired(60) {
                    return Ok(cached.token.clone());
                }
            }
        }

        // 执行实际的 load + exchange
        let token = self.load_and_exchange(profile, &provider).await?;

        // 写入缓存
        {
            let mut cache = self.cache.lock().await;
            cache.insert(
                profile.name.clone(),
                CachedToken {
                    token: token.clone(),
                    cached_at: chrono::Utc::now().timestamp_millis(),
                },
            );
        }

        Ok(token)
    }

    /// 401 后调用: 清除缓存并重新获取 token
    pub async fn invalidate_and_retry(&self, profile: &ProfileConfig) -> Result<OAuthToken> {
        {
            let mut cache = self.cache.lock().await;
            cache.remove(&profile.name);
        }
        self.get_token(profile).await
    }

    /// CLI logout 时清除缓存
    pub async fn invalidate(&self, profile_name: &str) {
        let mut cache = self.cache.lock().await;
        cache.remove(profile_name);
    }

    /// 根据 provider 执行凭证加载和必要的 token 交换
    async fn load_and_exchange(
        &self,
        profile: &ProfileConfig,
        provider: &OAuthProvider,
    ) -> Result<OAuthToken> {
        match provider {
            OAuthProvider::Chatgpt | OAuthProvider::Openai => {
                self.load_chatgpt_token(profile).await
            }
            OAuthProvider::Github => self.load_github_token(profile).await,
            OAuthProvider::Claude => self.load_simple_token(provider, profile).await,
            OAuthProvider::Google => self.load_simple_token(provider, profile).await,
            OAuthProvider::Kimi => self.load_simple_token(provider, profile).await,
            OAuthProvider::Qwen => self.load_simple_token(provider, profile).await,
        }
    }

    /// ChatGPT: 加载凭证 + 过期时自动 refresh
    async fn load_chatgpt_token(&self, profile: &ProfileConfig) -> Result<OAuthToken> {
        let cred = super::source::load_credential_chain(
            &profile.oauth_provider.as_ref().unwrap().normalize(),
        )
        .with_context(|| {
            format!(
                "ChatGPT token not available for '{}'. Run `claudex auth login chatgpt --profile {}`",
                profile.name, profile.name
            )
        })?;

        let token = cred.into_oauth_token();

        // 如果过期且有 refresh_token，自动刷新
        if token.is_expired(60) {
            if let Some(ref refresh_tok) = token.refresh_token {
                tracing::info!(
                    profile = %profile.name,
                    "ChatGPT token expired, refreshing..."
                );
                return super::exchange::refresh_chatgpt_token(&self.http_client, refresh_tok)
                    .await;
            }
            anyhow::bail!(
                "ChatGPT token expired for '{}' and no refresh_token available. Run `claudex auth login chatgpt --profile {}`",
                profile.name, profile.name
            );
        }

        Ok(token)
    }

    /// GitHub: 加载 GitHub token + 交换为 Copilot bearer token
    async fn load_github_token(&self, profile: &ProfileConfig) -> Result<OAuthToken> {
        let cred =
            super::source::load_credential_chain(&OAuthProvider::Github).with_context(|| {
                format!(
                "GitHub token not available for '{}'. Run `claudex auth login github --profile {}`",
                profile.name, profile.name
            )
            })?;

        let github_token = &cred.access_token;

        // 交换为 Copilot bearer token
        let copilot = super::exchange::exchange_github_for_copilot(&self.http_client, github_token)
            .await
            .context("failed to exchange GitHub token for Copilot bearer token")?;

        Ok(OAuthToken {
            access_token: copilot.token,
            refresh_token: None,
            expires_at: Some(copilot.expires_at * 1000), // seconds -> millis
            token_type: Some("Bearer".to_string()),
            scopes: None,
            extra: Some(serde_json::json!({
                "provider": "copilot",
                "github_token": github_token,
            })),
        })
    }

    /// 简单 provider: 直接加载凭证，不做额外交换
    async fn load_simple_token(
        &self,
        provider: &OAuthProvider,
        profile: &ProfileConfig,
    ) -> Result<OAuthToken> {
        let cred = super::source::load_credential_chain(provider).with_context(|| {
            format!(
                "OAuth token not available for '{}'. Run `claudex auth login {} --profile {}`",
                profile.name,
                provider.display_name().to_lowercase(),
                profile.name
            )
        })?;
        Ok(cred.into_oauth_token())
    }
}

/// 将 token 信息注入到 profile 的 api_key 和 extra_env 中
pub fn apply_token_to_profile(profile: &mut ProfileConfig, token: &OAuthToken) {
    profile.api_key = token.access_token.clone();

    // ChatGPT: 注入 CHATGPT_ACCOUNT_ID
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

    // GitHub Copilot: 标记 provider
    if let Some(true) = token
        .extra
        .as_ref()
        .map(|e| e.get("provider").and_then(|v| v.as_str()) == Some("copilot"))
    {
        profile
            .extra_env
            .entry("COPILOT_AUTH".to_string())
            .or_insert_with(|| "true".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_oauth_profile(name: &str, provider: OAuthProvider) -> ProfileConfig {
        ProfileConfig {
            name: name.to_string(),
            auth_type: AuthType::OAuth,
            oauth_provider: Some(provider),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_get_token_non_oauth_profile_fails() {
        let manager = TokenManager::new(reqwest::Client::new());
        let profile = ProfileConfig {
            name: "test".to_string(),
            auth_type: AuthType::ApiKey,
            ..Default::default()
        };
        let result = manager.get_token(&profile).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not OAuth"));
    }

    #[tokio::test]
    async fn test_get_token_no_provider_fails() {
        let manager = TokenManager::new(reqwest::Client::new());
        let profile = ProfileConfig {
            name: "test".to_string(),
            auth_type: AuthType::OAuth,
            oauth_provider: None,
            ..Default::default()
        };
        let result = manager.get_token(&profile).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("no oauth_provider"));
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let manager = TokenManager::new(reqwest::Client::new());

        // 手动写入缓存
        {
            let mut cache = manager.cache.lock().await;
            cache.insert(
                "test-profile".to_string(),
                CachedToken {
                    token: OAuthToken {
                        access_token: "cached-token".to_string(),
                        refresh_token: None,
                        expires_at: Some(chrono::Utc::now().timestamp_millis() + 3600_000),
                        token_type: Some("Bearer".to_string()),
                        scopes: None,
                        extra: None,
                    },
                    cached_at: chrono::Utc::now().timestamp_millis(),
                },
            );
        }

        let profile = make_oauth_profile("test-profile", OAuthProvider::Claude);
        let token = manager.get_token(&profile).await.unwrap();
        assert_eq!(token.access_token, "cached-token");
    }

    #[tokio::test]
    async fn test_invalidate_clears_cache() {
        let manager = TokenManager::new(reqwest::Client::new());

        {
            let mut cache = manager.cache.lock().await;
            cache.insert(
                "test".to_string(),
                CachedToken {
                    token: OAuthToken {
                        access_token: "old".to_string(),
                        refresh_token: None,
                        expires_at: Some(chrono::Utc::now().timestamp_millis() + 3600_000),
                        token_type: None,
                        scopes: None,
                        extra: None,
                    },
                    cached_at: chrono::Utc::now().timestamp_millis(),
                },
            );
        }

        manager.invalidate("test").await;

        let cache = manager.cache.lock().await;
        assert!(!cache.contains_key("test"));
    }

    #[test]
    fn test_apply_token_to_profile_chatgpt() {
        let mut profile = make_oauth_profile("codex", OAuthProvider::Chatgpt);
        let token = OAuthToken {
            access_token: "test-access".to_string(),
            refresh_token: None,
            expires_at: None,
            token_type: Some("Bearer".to_string()),
            scopes: None,
            extra: Some(serde_json::json!({"account_id": "acc-123"})),
        };
        apply_token_to_profile(&mut profile, &token);
        assert_eq!(profile.api_key, "test-access");
        assert_eq!(
            profile.extra_env.get("CHATGPT_ACCOUNT_ID").unwrap(),
            "acc-123"
        );
    }

    #[test]
    fn test_apply_token_to_profile_no_override_existing() {
        let mut profile = make_oauth_profile("codex", OAuthProvider::Chatgpt);
        profile
            .extra_env
            .insert("CHATGPT_ACCOUNT_ID".to_string(), "existing".to_string());
        let token = OAuthToken {
            access_token: "tok".to_string(),
            refresh_token: None,
            expires_at: None,
            token_type: None,
            scopes: None,
            extra: Some(serde_json::json!({"account_id": "new-acc"})),
        };
        apply_token_to_profile(&mut profile, &token);
        // 不覆盖已有值
        assert_eq!(
            profile.extra_env.get("CHATGPT_ACCOUNT_ID").unwrap(),
            "existing"
        );
    }

    #[test]
    fn test_apply_token_to_profile_copilot() {
        let mut profile = make_oauth_profile("copilot", OAuthProvider::Github);
        let token = OAuthToken {
            access_token: "copilot-bearer".to_string(),
            refresh_token: None,
            expires_at: None,
            token_type: Some("Bearer".to_string()),
            scopes: None,
            extra: Some(serde_json::json!({"provider": "copilot"})),
        };
        apply_token_to_profile(&mut profile, &token);
        assert_eq!(profile.api_key, "copilot-bearer");
        assert_eq!(profile.extra_env.get("COPILOT_AUTH").unwrap(), "true");
    }
}
