use axum::{extract::Request, middleware::Next, response::Response};
use axum::http::header::AUTHORIZATION;
use base64::Engine;
use tower_sessions::Session;

use crate::error::AppError;
use crate::auth::pam::authenticate;

/// Middleware to enforce authentication.
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
