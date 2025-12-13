//! Authentication guard middleware.
//!
//! This middleware checks every request for valid authentication before
//! allowing access to protected routes. It supports two authentication methods:
//!
//! 1. **Session-based**: Check for "username" in the session (set after login)
//! 2. **HTTP Basic Auth**: Decode Authorization header and authenticate via PAM
//!
//! For API clients that can't maintain sessions, HTTP Basic Auth provides
//! a stateless authentication option (credentials are verified on each request).

use axum::http::header::AUTHORIZATION;
use axum::{extract::Request, middleware::Next, response::Response};
use base64::Engine;
use tower_sessions::Session;

use crate::auth::pam::authenticate;
use crate::error::AppError;

/// Middleware to enforce authentication on protected routes.
/// Returns 401 Unauthorized if no valid session or Basic Auth credentials.
pub async fn auth_guard(session: Session, req: Request, next: Next) -> Result<Response, AppError> {
    if session
        .get::<String>("username")
        .await
        .unwrap_or(None)
        .is_some()
    {
        return Ok(next.run(req).await);
    } else if let Some(header_value) = req.headers().get(AUTHORIZATION) {
        if let Ok(header_str) = header_value.to_str() {
            if let Some(basic) = header_str.strip_prefix("Basic ") {
                if let Ok(decoded) =
                    base64::engine::general_purpose::STANDARD.decode(basic.trim())
                {
                    if let Ok(pair) = String::from_utf8(decoded) {
                        if let Some((user, pass)) = pair.split_once(':') {
                            // Authenticate against PAM
                            authenticate(user.to_string(), pass.to_string()).await?;
                            // Persist session for subsequent requests
                            session
                                .insert("username", &user.to_string())
                                .await
                                .map_err(|e| {
                                    AppError::InternalServerError(format!(
                                        "Failed to insert into session: {e}"
                                    ))
                                })?;
                            return Ok(next.run(req).await);
                        }
                    }
                }
            }
        }
    }

    Err(AppError::Unauthorized(
        "Authentication required".to_string(),
    ))
}
