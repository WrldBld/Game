//! Authentication middleware for HTTP routes
//!
//! Currently extracts user_id from X-User-Id header.
//! Future: JWT validation will be added here.
//!
//! # Usage
//!
//! ```rust,ignore
//! use axum::{Router, middleware};
//!
//! let app = Router::new()
//!     .route("/protected", get(handler))
//!     .layer(middleware::from_fn(auth_middleware));
//!
//! // In handler:
//! async fn handler(Auth(user): Auth) -> impl IntoResponse {
//!     format!("Hello, {}", user.user_id)
//! }
//! ```

use axum::{
    extract::{FromRequestParts, Request},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};

/// User ID extracted from request headers
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
}

/// Middleware that extracts user_id from X-User-Id header
///
/// This is a permissive middleware - it extracts the user if present
/// but doesn't reject requests without authentication. Use `require_auth_middleware`
/// for endpoints that require authentication.
///
/// # Future Enhancement
/// This will be replaced with proper JWT validation when
/// production authentication is implemented.
pub async fn auth_middleware(mut request: Request, next: Next) -> Response {
    let user_id = request
        .headers()
        .get("X-User-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if let Some(user_id) = user_id {
        request
            .extensions_mut()
            .insert(AuthenticatedUser { user_id });
    }
    // Note: We don't reject requests without user_id yet
    // Some endpoints may be public

    next.run(request).await
}

/// Middleware that requires authentication
///
/// Use this for endpoints that must have a user_id.
/// Returns 401 Unauthorized if X-User-Id header is missing.
pub async fn require_auth_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    // Check if already authenticated by previous middleware
    if request.extensions().get::<AuthenticatedUser>().is_some() {
        return Ok(next.run(request).await);
    }

    // Try to extract from header
    let user_id = request
        .headers()
        .get("X-User-Id")
        .and_then(|v| v.to_str().ok());

    if user_id.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}

/// Extractor for authenticated user in handlers
///
/// # Example
///
/// ```rust,ignore
/// async fn protected_handler(Auth(user): Auth) -> impl IntoResponse {
///     format!("User ID: {}", user.user_id)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Auth(pub AuthenticatedUser);

impl<S> FromRequestParts<S> for Auth
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // First try to get from extensions (if auth_middleware ran)
        if let Some(user) = parts.extensions.get::<AuthenticatedUser>() {
            return Ok(Auth(user.clone()));
        }

        // Otherwise try to extract from header directly
        let user_id = parts
            .headers
            .get("X-User-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        Ok(Auth(AuthenticatedUser { user_id }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{header::HeaderValue, Request as HttpRequest},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler(auth: Option<Auth>) -> String {
        match auth {
            Some(Auth(user)) => format!("user:{}", user.user_id),
            None => "no-user".to_string(),
        }
    }

    async fn protected_handler(Auth(user): Auth) -> String {
        format!("user:{}", user.user_id)
    }

    #[tokio::test]
    async fn test_auth_extractor_with_header() {
        let app = Router::new().route("/", get(protected_handler));

        let request = HttpRequest::builder()
            .uri("/")
            .header("X-User-Id", "test-user-123")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"user:test-user-123");
    }

    #[tokio::test]
    async fn test_auth_extractor_without_header() {
        let app = Router::new().route("/", get(protected_handler));

        let request = HttpRequest::builder().uri("/").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
