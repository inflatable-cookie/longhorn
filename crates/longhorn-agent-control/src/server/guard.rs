//! Bearer-token and `Origin` enforcement ahead of MCP dispatch.
//!
//! Both checks run in an axum layer outside the rmcp service, so a rejected
//! request never reaches tool dispatch at all — the ordering is
//! construction, not convention. `Origin` is checked first: a
//! browser-originated request gets 403 without probing token validity.
//!
//! The `Origin` policy is the DNS-rebinding defense and is not optional
//! (contract 022): browsers always send `Origin` on POST, non-browser
//! agents send none, so an absent header passes and a present one must be
//! a loopback origin. rmcp's own loopback-only `Host` validation stays on
//! underneath as a second layer.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode, Uri, header},
    middleware::Next,
    response::IntoResponse,
};

use crate::InstanceToken;

/// Shared guard state: the one instance token.
#[derive(Clone)]
pub(super) struct GuardState {
    token: Arc<InstanceToken>,
}

impl GuardState {
    /// Wraps the instance token.
    pub(super) fn new(token: InstanceToken) -> Self {
        Self {
            token: Arc::new(token),
        }
    }
}

/// Rejects browser-originated and unauthenticated requests before dispatch.
pub(super) async fn auth_origin_guard(
    State(state): State<GuardState>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    if let Some(origin) = request.headers().get(header::ORIGIN) {
        let allowed = origin.to_str().ok().is_some_and(is_loopback_origin);
        if !allowed {
            return StatusCode::FORBIDDEN.into_response();
        }
    }

    let presented = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    match presented {
        Some(presented) if state.token.verify(presented) => next.run(request).await,
        _ => (
            StatusCode::UNAUTHORIZED,
            [(
                header::WWW_AUTHENTICATE,
                "Bearer realm=\"longhorn-agent-control\"",
            )],
        )
            .into_response(),
    }
}

/// A present `Origin` must be a loopback origin: `http`/`https` with host
/// `localhost`, a `*.localhost` subdomain, `127.0.0.1`, or `[::1]`. The
/// browser `null` origin (sandboxed frames, redirects) fails by falling
/// through the parse.
fn is_loopback_origin(origin: &str) -> bool {
    let Ok(uri) = origin.parse::<Uri>() else {
        return false;
    };
    let scheme_ok = matches!(uri.scheme_str(), Some("http" | "https"));
    let host_ok = uri.host().is_some_and(|host| {
        host == "localhost"
            || host.ends_with(".localhost")
            || host == "127.0.0.1"
            || host == "::1"
            || host == "[::1]"
    });
    scheme_ok && host_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_policy() {
        for allowed in [
            "http://localhost:3000",
            "https://localhost",
            "http://app.localhost:8080",
            "http://127.0.0.1:49152",
            "http://[::1]:9000",
        ] {
            assert!(is_loopback_origin(allowed), "{allowed} must be allowed");
        }
        for rejected in [
            "https://evil.example",
            "null",
            "http://127.0.0.1.evil.example",
            "file://",
            "http://localhost.evil.example",
            "not a url",
        ] {
            assert!(!is_loopback_origin(rejected), "{rejected} must be rejected");
        }
    }
}
