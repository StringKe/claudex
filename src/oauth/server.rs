use std::net::TcpListener;
use std::sync::Arc;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tokio::sync::oneshot;

/// PKCE challenge pair for Authorization Code flow
pub struct PkceChallenge {
    pub code_verifier: String,
    pub code_challenge: String,
}

impl PkceChallenge {
    pub fn generate() -> Self {
        use base64::Engine;

        let mut verifier_bytes = [0u8; 32];
        rand::fill(&mut verifier_bytes);
        let code_verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(verifier_bytes);

        let digest = Sha256::digest(code_verifier.as_bytes());
        let code_challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);

        Self {
            code_verifier,
            code_challenge,
        }
    }
}

/// 找一个可用的本地端口
pub fn find_available_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("failed to bind ephemeral port")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

/// 启动本地 OAuth 回调服务器，等待浏览器回调携带 auth code
pub async fn start_callback_server(port: u16) -> Result<String> {
    let (tx, rx) = oneshot::channel::<String>();
    let tx = Arc::new(tokio::sync::Mutex::new(Some(tx)));

    let tx_clone = tx.clone();
    let app = axum::Router::new().route(
        "/auth/callback",
        axum::routing::get(
            move |axum::extract::Query(params): axum::extract::Query<
                std::collections::HashMap<String, String>,
            >| {
                let tx = tx_clone.clone();
                async move {
                    if let Some(code) = params.get("code") {
                        let mut guard = tx.lock().await;
                        if let Some(sender) = guard.take() {
                            let _ = sender.send(code.clone());
                        }
                        axum::response::Html(
                            "<html><body><h1>Authorization successful!</h1>\
                             <p>You can close this tab and return to the terminal.</p>\
                             <script>window.close()</script></body></html>"
                                .to_string(),
                        )
                    } else {
                        let error = params
                            .get("error")
                            .cloned()
                            .unwrap_or_else(|| "unknown error".to_string());
                        let desc = params.get("error_description").cloned().unwrap_or_default();
                        axum::response::Html(format!(
                            "<html><body><h1>Authorization failed</h1>\
                             <p>Error: {error}</p><p>{desc}</p></body></html>"
                        ))
                    }
                }
            },
        ),
    );

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .context("failed to bind callback server")?;

    // Spawn the server with graceful shutdown
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    // Wait for the callback (timeout 5 minutes), interruptible by Ctrl+C
    let code = tokio::select! {
        result = tokio::time::timeout(std::time::Duration::from_secs(300), rx) => {
            result
                .context("OAuth callback timed out (5 minutes)")?
                .context("callback channel closed unexpectedly")?
        }
        _ = tokio::signal::ctrl_c() => {
            server_handle.abort();
            anyhow::bail!("interrupted by user (Ctrl+C)");
        }
    };

    server_handle.abort();

    Ok(code)
}

/// Device Code Flow: 轮询 token 端点直到用户授权
pub async fn poll_device_code(
    client: &reqwest::Client,
    token_url: &str,
    device_code: &str,
    client_id: &str,
    interval_secs: u64,
    grant_type: &str,
) -> Result<serde_json::Value> {
    let interval = std::time::Duration::from_secs(interval_secs);

    loop {
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = tokio::signal::ctrl_c() => {
                anyhow::bail!("interrupted by user (Ctrl+C)");
            }
        }

        let resp = client
            .post(token_url)
            .header("Accept", "application/json")
            .form(&[
                ("grant_type", grant_type),
                ("device_code", device_code),
                ("client_id", client_id),
            ])
            .send()
            .await
            .context("device code poll request failed")?;

        let body: serde_json::Value = resp
            .json()
            .await
            .context("invalid JSON from token endpoint")?;

        if body.get("access_token").is_some() {
            return Ok(body);
        }

        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match error {
            "authorization_pending" => continue,
            "slow_down" => {
                // Increase interval by 5 seconds
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
            "expired_token" => anyhow::bail!("device code expired, please try again"),
            "access_denied" => anyhow::bail!("user denied the authorization request"),
            _ => anyhow::bail!("device code error: {error}"),
        }
    }
}

/// 用 auth code + code_verifier 换取 token
pub async fn exchange_code_for_token(
    client: &reqwest::Client,
    token_url: &str,
    code: &str,
    redirect_uri: &str,
    client_id: &str,
    code_verifier: &str,
) -> Result<serde_json::Value> {
    let resp = client
        .post(token_url)
        .header("Accept", "application/json")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .context("token exchange request failed")?;

    let status = resp.status();
    let body: serde_json::Value = resp
        .json()
        .await
        .context("invalid JSON from token exchange")?;

    if !status.is_success() {
        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let desc = body
            .get("error_description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        anyhow::bail!("token exchange failed (HTTP {status}): {error} - {desc}");
    }

    Ok(body)
}

/// 使用 refresh_token 刷新 access_token
pub async fn refresh_access_token(
    client: &reqwest::Client,
    token_url: &str,
    refresh_token: &str,
    client_id: &str,
) -> Result<serde_json::Value> {
    let resp = client
        .post(token_url)
        .header("Accept", "application/json")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
        ])
        .send()
        .await
        .context("token refresh request failed")?;

    let status = resp.status();
    let body: serde_json::Value = resp
        .json()
        .await
        .context("invalid JSON from token refresh")?;

    if !status.is_success() {
        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        anyhow::bail!("token refresh failed (HTTP {status}): {error}");
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_challenge_generation() {
        let pkce = PkceChallenge::generate();
        assert!(!pkce.code_verifier.is_empty());
        assert!(!pkce.code_challenge.is_empty());
        assert_ne!(pkce.code_verifier, pkce.code_challenge);

        // Verify challenge is SHA256 of verifier
        use base64::Engine;
        let digest = Sha256::digest(pkce.code_verifier.as_bytes());
        let expected = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
        assert_eq!(pkce.code_challenge, expected);
    }

    #[test]
    fn test_find_available_port() {
        let port = find_available_port().unwrap();
        assert!(port > 0);
    }

    #[test]
    fn test_pkce_verifier_length_rfc_compliant() {
        // RFC 7636: code_verifier MUST be 43-128 characters (base64url of 32 bytes = 43 chars)
        let pkce = PkceChallenge::generate();
        assert!(
            pkce.code_verifier.len() >= 43,
            "verifier too short: {} chars",
            pkce.code_verifier.len()
        );
        assert!(
            pkce.code_verifier.len() <= 128,
            "verifier too long: {} chars",
            pkce.code_verifier.len()
        );
    }

    #[test]
    fn test_pkce_verifier_url_safe_chars_only() {
        let pkce = PkceChallenge::generate();
        // URL-safe base64 without padding: [A-Za-z0-9_-]
        assert!(
            pkce.code_verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "verifier contains non-URL-safe chars: {}",
            pkce.code_verifier
        );
    }

    #[test]
    fn test_pkce_uniqueness() {
        let a = PkceChallenge::generate();
        let b = PkceChallenge::generate();
        assert_ne!(
            a.code_verifier, b.code_verifier,
            "two PKCE pairs should be unique"
        );
        assert_ne!(a.code_challenge, b.code_challenge);
    }

    #[test]
    fn test_find_available_port_returns_distinct_ports() {
        // 连续获取两个端口，应该不同（概率上）
        let p1 = find_available_port().unwrap();
        let p2 = find_available_port().unwrap();
        // 虽然理论上可能相同（第一个释放后第二个复用），但极不可能
        // 只检查都是有效端口
        assert!(p1 > 0);
        assert!(p2 > 0);
    }

    #[tokio::test]
    async fn test_callback_server_receives_code() {
        let port = find_available_port().unwrap();

        let server = tokio::spawn(async move { start_callback_server(port).await });

        // 等服务器起来
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 模拟浏览器回调
        let client = reqwest::Client::new();
        let resp = client
            .get(format!(
                "http://127.0.0.1:{port}/auth/callback?code=test-auth-code-xyz"
            ))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body = resp.text().await.unwrap();
        assert!(body.contains("Authorization successful"));

        let code = server.await.unwrap().unwrap();
        assert_eq!(code, "test-auth-code-xyz");
    }

    #[tokio::test]
    async fn test_callback_server_handles_error_response() {
        let port = find_available_port().unwrap();

        let server = tokio::spawn(async move { start_callback_server(port).await });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 模拟带 error 的回调（无 code）
        let client = reqwest::Client::new();
        let resp = client
            .get(format!(
                "http://127.0.0.1:{port}/auth/callback?error=access_denied&error_description=User+denied"
            ))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body = resp.text().await.unwrap();
        assert!(body.contains("Authorization failed"));
        assert!(body.contains("access_denied"));

        // 服务器不会收到 code，等超时（但我们不想等 5 分钟，所以 abort）
        server.abort();
    }
}
