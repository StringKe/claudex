use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("profile '{0}' not found")]
    ProfileNotFound(String),

    #[error("profile '{0}' is disabled")]
    ProfileDisabled(String),

    #[error("circuit breaker open for '{0}'")]
    CircuitBreakerOpen(String),

    #[error("upstream HTTP {status}: {body}")]
    UpstreamError { status: u16, body: String },

    #[error("translation error: {0}")]
    Translation(#[from] anyhow::Error),

    #[error("OAuth token error: {0}")]
    OAuthError(String),

    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("invalid request: {0}")]
    BadRequest(String),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ProxyError::ProfileNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ProxyError::ProfileDisabled(_) => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            ProxyError::CircuitBreakerOpen(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
            }
            ProxyError::UpstreamError { status, body } => (
                StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY),
                body.clone(),
            ),
            ProxyError::Translation(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ProxyError::OAuthError(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            ProxyError::Request(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            ProxyError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
        };
        (status, message).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_not_found_display() {
        let err = ProxyError::ProfileNotFound("grok".to_string());
        assert_eq!(err.to_string(), "profile 'grok' not found");
    }

    #[test]
    fn test_circuit_breaker_display() {
        let err = ProxyError::CircuitBreakerOpen("test".to_string());
        assert!(err.to_string().contains("circuit breaker"));
    }

    #[test]
    fn test_upstream_error_display() {
        let err = ProxyError::UpstreamError {
            status: 503,
            body: "service unavailable".to_string(),
        };
        assert!(err.to_string().contains("503"));
    }

    #[test]
    fn test_bad_request_display() {
        let err = ProxyError::BadRequest("invalid JSON".to_string());
        assert!(err.to_string().contains("invalid JSON"));
    }

    #[test]
    fn test_into_response_not_found() {
        let err = ProxyError::ProfileNotFound("test".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_into_response_bad_request() {
        let err = ProxyError::BadRequest("bad".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_into_response_unauthorized() {
        let err = ProxyError::OAuthError("expired".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
