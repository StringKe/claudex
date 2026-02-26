use anyhow::Result;

use super::{OAuthProvider, OAuthToken};

/// Trait abstracting per-provider OAuth operations.
///
/// Each provider implements login (obtain initial token) and refresh
/// (re-validate or refresh an expired token). The trait uses dynamic
/// dispatch so providers can be selected at runtime.
pub trait OAuthProviderHandler: Send + Sync {
    /// Which provider this handler serves.
    fn provider(&self) -> OAuthProvider;

    /// Obtain an initial OAuth token (interactive: may open browser, read CLI files, etc.)
    fn login(
        &self,
        profile_name: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>>;

    /// Refresh an existing token. Returns the new token.
    fn refresh(
        &self,
        profile_name: &str,
        token: &OAuthToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>>;

    /// Read token from external CLI files (non-interactive).
    /// Falls back to keyring if no external CLI is available.
    fn read_external_token(&self) -> Result<OAuthToken>;
}

/// Factory: get the handler for a given provider.
pub fn for_provider(provider: &OAuthProvider) -> Box<dyn OAuthProviderHandler> {
    match provider {
        OAuthProvider::Claude => Box::new(ClaudeHandler),
        OAuthProvider::Openai => Box::new(OpenaiHandler),
        OAuthProvider::Google => Box::new(ExternalCliHandler {
            provider: OAuthProvider::Google,
        }),
        OAuthProvider::Kimi => Box::new(ExternalCliHandler {
            provider: OAuthProvider::Kimi,
        }),
        OAuthProvider::Qwen => Box::new(DeviceCodeHandler {
            provider: OAuthProvider::Qwen,
        }),
        OAuthProvider::Github => Box::new(DeviceCodeHandler {
            provider: OAuthProvider::Github,
        }),
    }
}

// ── Claude: read ~/.claude/.credentials.json ──

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
            println!("Reading Claude credentials from ~/.claude/.credentials.json...");
            let token = super::token::read_claude_credentials()
                .map_err(|e| anyhow::anyhow!("Failed to read Claude credentials: {e}"))?;
            println!("Note: Claude subscription profiles bypass the proxy (Claude Code uses its own OAuth).");
            Ok(token)
        })
    }

    fn refresh(
        &self,
        _profile_name: &str,
        _token: &OAuthToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        Box::pin(async {
            let token = super::token::read_claude_credentials()?;
            println!("Refreshed Claude token from ~/.claude/.credentials.json");
            Ok(token)
        })
    }

    fn read_external_token(&self) -> Result<OAuthToken> {
        super::token::read_claude_credentials()
    }
}

// ── OpenAI: read Codex CLI + refresh_token ──

struct OpenaiHandler;

impl OAuthProviderHandler for OpenaiHandler {
    fn provider(&self) -> OAuthProvider {
        OAuthProvider::Openai
    }

    fn login(
        &self,
        profile_name: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        let profile_name = profile_name.to_string();
        Box::pin(async move {
            match super::token::read_codex_credentials() {
                Ok(token) => {
                    let auth_mode = token
                        .extra
                        .as_ref()
                        .and_then(|e| e.get("auth_mode"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    println!("Found Codex CLI credentials (auth_mode: {auth_mode})");
                    println!("Token will be refreshed automatically from ~/.codex/auth.json");
                    Ok(token)
                }
                Err(_) => {
                    println!("No Codex CLI credentials found at ~/.codex/auth.json");
                    println!();
                    println!("To use your ChatGPT subscription with Claudex:");
                    println!("  1. Install Codex CLI: npm install -g @openai/codex");
                    println!("  2. Login: codex --login");
                    println!(
                        "  3. Re-run: claudex auth login openai --profile {}",
                        profile_name
                    );
                    anyhow::bail!("no OpenAI credentials available")
                }
            }
        })
    }

    fn refresh(
        &self,
        _profile_name: &str,
        token: &OAuthToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        let refresh_tok = token.refresh_token.clone();
        Box::pin(async move {
            let refresh_tok = refresh_tok.ok_or_else(|| {
                anyhow::anyhow!(
                    "no refresh_token in Codex credentials, please re-login with `codex --login`"
                )
            })?;
            super::providers::refresh_openai_token_pub(&refresh_tok, Some(refresh_tok.clone()))
                .await
        })
    }

    fn read_external_token(&self) -> Result<OAuthToken> {
        super::token::read_external_token(&OAuthProvider::Openai)
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
            println!(
                "Reading {} credentials from external CLI...",
                provider.display_name()
            );
            let token = super::token::read_external_token(&provider)?;
            Ok(token)
        })
    }

    fn refresh(
        &self,
        _profile_name: &str,
        _token: &OAuthToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        let provider = self.provider.clone();
        Box::pin(async move {
            let token = super::token::read_external_token(&provider)?;
            println!(
                "Refreshed {} token from external CLI",
                provider.display_name()
            );
            Ok(token)
        })
    }

    fn read_external_token(&self) -> Result<OAuthToken> {
        super::token::read_external_token(&self.provider)
    }
}

// ── Device Code: GitHub, Qwen ──

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
            // Device code login is handled by providers::login_device_code
            // which requires interactive I/O. Delegate to the existing impl.
            super::providers::login_device_code_pub(&provider).await
        })
    }

    fn refresh(
        &self,
        profile_name: &str,
        _token: &OAuthToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OAuthToken>> + Send + '_>> {
        let provider = self.provider.clone();
        let profile_name = profile_name.to_string();
        Box::pin(async move {
            super::providers::refresh_device_code_pub(&provider, &profile_name).await
        })
    }

    fn read_external_token(&self) -> Result<OAuthToken> {
        super::token::read_external_token(&self.provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_returns_correct_provider() {
        let handler = for_provider(&OAuthProvider::Claude);
        assert_eq!(handler.provider(), OAuthProvider::Claude);

        let handler = for_provider(&OAuthProvider::Openai);
        assert_eq!(handler.provider(), OAuthProvider::Openai);

        let handler = for_provider(&OAuthProvider::Google);
        assert_eq!(handler.provider(), OAuthProvider::Google);

        let handler = for_provider(&OAuthProvider::Qwen);
        assert_eq!(handler.provider(), OAuthProvider::Qwen);

        let handler = for_provider(&OAuthProvider::Kimi);
        assert_eq!(handler.provider(), OAuthProvider::Kimi);

        let handler = for_provider(&OAuthProvider::Github);
        assert_eq!(handler.provider(), OAuthProvider::Github);
    }

    #[test]
    fn test_all_providers_have_handler() {
        let providers = [
            OAuthProvider::Claude,
            OAuthProvider::Openai,
            OAuthProvider::Google,
            OAuthProvider::Qwen,
            OAuthProvider::Kimi,
            OAuthProvider::Github,
        ];
        for p in &providers {
            let handler = for_provider(p);
            assert_eq!(&handler.provider(), p);
        }
    }
}
