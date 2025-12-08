use axum::{extract::Request, middleware::Next, response::Response};
use tower_sessions::Session;

use crate::error::AppError;

/// Middleware to enforce authentication.
pub async fn auth_guard(session: Session, req: Request, next: Next) -> Result<Response, AppError> {
    if session
        .get::<String>("username")
        .await
        .unwrap_or(None)
        .is_some()
    {
        Ok(next.run(req).await)
    } else {
        Err(AppError::Unauthorized(
            "Authentication required".to_string(),
        ))
    }
}
