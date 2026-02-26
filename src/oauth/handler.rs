use anyhow::Result;

use super::{OAuthProvider, OAuthToken};

/// Trait abstracting per-provider OAuth operations.
pub trait OAuthProviderHandler: Send + Sync {
    fn provider(&self) -> OAuthProvider;

    fn login(
        &self,
        profile_name: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>>;

    fn refresh(
        &self,
        profile_name: &str,
        token: &OAuthToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>>;

    fn read_external_token(&self) -> Result<OAuthToken>;
}

/// Factory: get the handler for a given provider.
pub fn for_provider(provider: &OAuthProvider) -> Box<dyn OAuthProviderHandler> {
    match provider.normalize() {
        OAuthProvider::Claude => Box::new(ClaudeHandler),
        OAuthProvider::Chatgpt | OAuthProvider::Openai => Box::new(ChatgptHandler),
        OAuthProvider::Google => Box::new(ExternalCliHandler {
            provider: OAuthProvider::Google,
        }),
        OAuthProvider::Kimi => Box::new(ExternalCliHandler {
            provider: OAuthProvider::Kimi,
        }),
        OAuthProvider::Qwen => Box::new(DeviceCodeHandler {
            provider: OAuthProvider::Qwen,
        }),
        OAuthProvider::Github => Box::new(GithubHandler),
    }
}

// ── Claude ──

struct ClaudeHandler;

impl OAuthProviderHandler for ClaudeHandler {
    fn provider(&self) -> OAuthProvider {
        OAuthProvider::Claude
    }

    fn login(
        &self,
        _profile_name: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        Box::pin(async {
            let cred = super::source::read_claude_credentials()?;
            Ok(cred.into_oauth_token())
        })
    }

    fn refresh(
        &self,
        _profile_name: &str,
        _token: &OAuthToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        Box::pin(async {
            let cred = super::source::read_claude_credentials()?;
            Ok(cred.into_oauth_token())
        })
    }

    fn read_external_token(&self) -> Result<OAuthToken> {
        super::source::read_claude_credentials().map(|c| c.into_oauth_token())
    }
}

// ── ChatGPT (was OpenAI) ──

struct ChatgptHandler;

impl OAuthProviderHandler for ChatgptHandler {
    fn provider(&self) -> OAuthProvider {
        OAuthProvider::Chatgpt
    }

    fn login(
        &self,
        _profile_name: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        Box::pin(async {
            let cred = super::source::read_codex_credentials()?;
            Ok(cred.into_oauth_token())
        })
    }

    fn refresh(
        &self,
        _profile_name: &str,
        token: &OAuthToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        let refresh_tok = token.refresh_token.clone();
        Box::pin(async move {
            let refresh_tok =
                refresh_tok.ok_or_else(|| anyhow::anyhow!("no refresh_token, please re-login"))?;
            let client = reqwest::Client::new();
            super::exchange::refresh_chatgpt_token(&client, &refresh_tok).await
        })
    }

    fn read_external_token(&self) -> Result<OAuthToken> {
        super::source::read_codex_credentials().map(|c| c.into_oauth_token())
    }
}

// ── External CLI: Google, Kimi ──

struct ExternalCliHandler {
    provider: OAuthProvider,
}

impl OAuthProviderHandler for ExternalCliHandler {
    fn provider(&self) -> OAuthProvider {
        self.provider.clone()
    }

    fn login(
        &self,
        _profile_name: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        let provider = self.provider.clone();
        Box::pin(async move {
            let cred = super::source::load_credential_chain(&provider)?;
            Ok(cred.into_oauth_token())
        })
    }

    fn refresh(
        &self,
        _profile_name: &str,
        _token: &OAuthToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        let provider = self.provider.clone();
        Box::pin(async move {
            let cred = super::source::load_credential_chain(&provider)?;
            Ok(cred.into_oauth_token())
        })
    }

    fn read_external_token(&self) -> Result<OAuthToken> {
        super::source::load_credential_chain(&self.provider).map(|c| c.into_oauth_token())
    }
}

// ── GitHub Copilot ──

struct GithubHandler;

impl OAuthProviderHandler for GithubHandler {
    fn provider(&self) -> OAuthProvider {
        OAuthProvider::Github
    }

    fn login(
        &self,
        _profile_name: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        Box::pin(async {
            let cred = super::source::load_credential_chain(&OAuthProvider::Github)?;
            let client = reqwest::Client::new();
            let copilot =
                super::exchange::exchange_github_for_copilot(&client, &cred.access_token).await?;
            Ok(OAuthToken {
                access_token: copilot.token,
                refresh_token: None,
                expires_at: Some(copilot.expires_at * 1000),
                token_type: Some("Bearer".to_string()),
                scopes: None,
                extra: Some(serde_json::json!({"provider": "copilot"})),
            })
        })
    }

    fn refresh(
        &self,
        _profile_name: &str,
        _token: &OAuthToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        Box::pin(async {
            let cred = super::source::load_credential_chain(&OAuthProvider::Github)?;
            let client = reqwest::Client::new();
            let copilot =
                super::exchange::exchange_github_for_copilot(&client, &cred.access_token).await?;
            Ok(OAuthToken {
                access_token: copilot.token,
                refresh_token: None,
                expires_at: Some(copilot.expires_at * 1000),
                token_type: Some("Bearer".to_string()),
                scopes: None,
                extra: Some(serde_json::json!({"provider": "copilot"})),
            })
        })
    }

    fn read_external_token(&self) -> Result<OAuthToken> {
        super::source::read_copilot_config().map(|c| c.into_oauth_token())
    }
}

// ── Device Code: Qwen ──

struct DeviceCodeHandler {
    provider: OAuthProvider,
}

impl OAuthProviderHandler for DeviceCodeHandler {
    fn provider(&self) -> OAuthProvider {
        self.provider.clone()
    }

    fn login(
        &self,
        _profile_name: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        let provider = self.provider.clone();
        Box::pin(async move {
            // Qwen device code login requires interactive I/O
            anyhow::bail!(
                "use `claudex auth login {}` for interactive device code flow",
                provider.display_name().to_lowercase()
            )
        })
    }

    fn refresh(
        &self,
        profile_name: &str,
        _token: &OAuthToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        let profile_name = profile_name.to_string();
        Box::pin(async move {
            let token = super::source::load_keyring(&profile_name)?;
            let refresh_token = token
                .refresh_token
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("no refresh_token, please re-login"))?;
            let client = reqwest::Client::new();
            let resp = super::server::refresh_access_token(
                &client,
                "https://chat.qwen.ai/api/oauth/token",
                refresh_token,
                "claudex-qwen",
            )
            .await?;
            let mut new_token = OAuthToken::from_token_response(&resp)
                .ok_or_else(|| anyhow::anyhow!("failed to parse refreshed token"))?;
            if new_token.refresh_token.is_none() {
                new_token.refresh_token = token.refresh_token;
            }
            Ok(new_token)
        })
    }

    fn read_external_token(&self) -> Result<OAuthToken> {
        anyhow::bail!("Qwen has no external CLI credentials")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_returns_correct_provider() {
        assert_eq!(
            for_provider(&OAuthProvider::Claude).provider(),
            OAuthProvider::Claude
        );
        assert_eq!(
            for_provider(&OAuthProvider::Chatgpt).provider(),
            OAuthProvider::Chatgpt
        );
        // Openai normalizes to Chatgpt handler
        assert_eq!(
            for_provider(&OAuthProvider::Openai).provider(),
            OAuthProvider::Chatgpt
        );
        assert_eq!(
            for_provider(&OAuthProvider::Google).provider(),
            OAuthProvider::Google
        );
        assert_eq!(
            for_provider(&OAuthProvider::Qwen).provider(),
            OAuthProvider::Qwen
        );
        assert_eq!(
            for_provider(&OAuthProvider::Kimi).provider(),
            OAuthProvider::Kimi
        );
        assert_eq!(
            for_provider(&OAuthProvider::Github).provider(),
            OAuthProvider::Github
        );
    }

    #[test]
    fn test_all_providers_have_handler() {
        let providers = [
            OAuthProvider::Claude,
            OAuthProvider::Chatgpt,
            OAuthProvider::Openai,
            OAuthProvider::Google,
            OAuthProvider::Qwen,
            OAuthProvider::Kimi,
            OAuthProvider::Github,
        ];
        for p in &providers {
            let _handler = for_provider(p);
        }
    }
}
