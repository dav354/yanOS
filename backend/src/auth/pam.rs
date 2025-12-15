//! PAM-based authentication for system user login.
//!
//! This module provides authentication against the system's PAM stack,
//! allowing users to log in with their illumos/OmniOS system credentials.
//!
//! # PAM Configuration
//! By default, uses the "yanos" PAM service (configurable via PAM_SERVICE_NAME env).
//! Create `/etc/pam.d/yanos` or add to `/etc/pam.conf`:
//! ```text
//! yanos  auth    required  pam_unix_auth.so.1
//! ```
//!
//! # Session Management
//! On successful authentication, the username is stored in the session
//! for subsequent request authentication via the auth_guard middleware.

use std::ffi::CString;
use std::ptr;

use axum::{routing::post, Json, Router};
use pam_sys::{
    pam_authenticate, pam_end, pam_handle_t, pam_message, pam_response, pam_start, pam_strerror,
    PAM_CONV_ERR, PAM_PROMPT_ECHO_OFF, PAM_SUCCESS,
};
use tower_sessions::Session;
use tracing::{error, info, instrument};

use crate::error::AppError;

const PAM_SUCCESS_I32: i32 = PAM_SUCCESS as i32;
const PAM_CONV_ERR_I32: i32 = PAM_CONV_ERR as i32;
const PAM_PROMPT_ECHO_OFF_I32: i32 = PAM_PROMPT_ECHO_OFF as i32;

/// Payload for a login request.
#[derive(serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
}

/// Adds authentication-related routes to the router.
pub fn add_auth_routes<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        .route("/api/v1/login", post(login_handler))
        .route("/api/v1/logout", post(logout_handler))
}

/// Struct to hold the credentials for the PAM conversation callback.
struct PamCredentials {
    password: CString,
}

/// PAM conversation function.
unsafe extern "C" fn pam_conversation(
    num_msg: i32,
    msg: *mut *const pam_message,
    resp: *mut *mut pam_response,
    appdata_ptr: *mut std::ffi::c_void,
) -> i32 {
    if num_msg <= 0 || num_msg > 32 {
        return PAM_CONV_ERR_I32;
    }

    unsafe {
        let credentials = &*(appdata_ptr as *const PamCredentials);
        let messages =
            std::slice::from_raw_parts(msg as *const *const pam_message, num_msg as usize);

        // Allocate memory for responses using libc::calloc to be compatible with PAM's expectation
        // that it can free it with free().
        let resp_ptr = libc::calloc(num_msg as usize, std::mem::size_of::<pam_response>())
            as *mut pam_response;
        if resp_ptr.is_null() {
            return PAM_CONV_ERR_I32;
        }

        let responses = std::slice::from_raw_parts_mut(resp_ptr, num_msg as usize);

        for (i, msg_ptr) in messages.iter().enumerate() {
            let msg = &**msg_ptr;
            if msg.msg_style == PAM_PROMPT_ECHO_OFF_I32 {
                // Password prompt
                let pass_ptr = libc::strdup(credentials.password.as_ptr());
                if pass_ptr.is_null() {
                    libc::free(resp_ptr as *mut _);
                    return PAM_CONV_ERR_I32;
                }
                responses[i].resp = pass_ptr;
                responses[i].resp_retcode = 0;
            } else {
                // Ignore other messages (echo on, error msg, text info)
                responses[i].resp = ptr::null_mut();
                responses[i].resp_retcode = 0;
            }
        }

        *resp = resp_ptr;
        PAM_SUCCESS_I32
    }
}

/// Handles the login request, authenticating against PAM.
#[utoipa::path(
    post,
    path = "/api/v1/login",
    request_body = LoginPayload,
    responses(
        (status = 200, description = "Authentication successful"),
        (status = 401, description = "Invalid credentials")
    )
)]
#[instrument(skip(payload, session))]
pub async fn login_handler(
    session: Session,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<&'static str>, AppError> {
    // Early validation: reject empty or whitespace-only credentials
    // This avoids hitting PAM which can be slow for invalid inputs
    if payload.username.trim().is_empty() || payload.password.is_empty() {
        return Err(AppError::Unauthorized(
            "Invalid credentials".to_string(),
        ));
    }

    info!(username = %payload.username, "Attempting to authenticate user");

    authenticate(payload.username.clone(), payload.password.clone()).await?;

    // Store username in the session.
    session
        .insert("username", &payload.username)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to insert into session: {}", e)))?;

    Ok(Json("Authentication successful"))
}

/// Handles the logout request, clearing the session.
#[utoipa::path(
    post,
    path = "/api/v1/logout",
    responses(
        (status = 200, description = "Logout successful")
    )
)]
#[instrument(skip(session))]
pub async fn logout_handler(session: Session) -> Result<Json<&'static str>, AppError> {
    session.flush().await.map_err(|e| {
        AppError::InternalServerError(format!("Failed to clear session: {}", e))
    })?;
    info!("User logged out");
    Ok(Json("Logout successful"))
}

/// Authenticates a user against the configured PAM stack.
pub async fn authenticate(username: String, password: String) -> Result<(), AppError> {
    // Early validation to avoid slow PAM lookups for invalid inputs
    if username.trim().is_empty() || password.is_empty() {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // Reject overly long usernames (max 256 chars) to avoid PAM issues
    if username.len() > 256 {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    let result = tokio::task::spawn_blocking(move || {
        // Default to the dedicated "yanos" PAM stack so we can avoid TTY-dependent modules.
        let service_name = std::env::var("PAM_SERVICE_NAME").unwrap_or_else(|_| "yanos".to_string());

        let c_service =
            CString::new(service_name.clone()).map_err(|_| "Invalid PAM service name")?;
        let c_user = CString::new(username).map_err(|_| "Invalid username")?;
        let c_pass = CString::new(password).map_err(|_| "Invalid password")?;

        let credentials = PamCredentials { password: c_pass };

        let conv = pam_sys::pam_conv {
            conv: Some(
                pam_conversation
                    as unsafe extern "C" fn(
                        i32,
                        *mut *const pam_message,
                        *mut *mut pam_response,
                        *mut std::ffi::c_void,
                    ) -> i32,
            ),

            appdata_ptr: &credentials as *const _ as *mut _,
        };

        let mut pam_h: *mut pam_handle_t = ptr::null_mut();

        unsafe {
            // 1. Start PAM transaction
            let retval = pam_start(c_service.as_ptr(), c_user.as_ptr(), &conv, &mut pam_h);

            if retval != PAM_SUCCESS_I32 {
                let msg = pam_strerror(pam_h, retval)
                    .as_ref()
                    .map(|c_str| {
                        std::ffi::CStr::from_ptr(c_str)
                            .to_string_lossy()
                            .into_owned()
                    })
                    .unwrap_or_else(|| "Failed to start PAM transaction".to_string());
                return Err(msg);
            }

            // 2. Authenticate
            let retval = pam_authenticate(pam_h, 0);

            if retval != PAM_SUCCESS_I32 {
                let _ = pam_end(pam_h, retval);
                let msg = pam_strerror(pam_h, retval)
                    .as_ref()
                    .map(|c_str| {
                        std::ffi::CStr::from_ptr(c_str)
                            .to_string_lossy()
                            .into_owned()
                    })
                    .unwrap_or_else(|| "Authentication failed".to_string());
                return Err(msg);
            }

            // 3. End PAM transaction
            pam_end(pam_h, PAM_SUCCESS_I32);
            Ok(())
        }
    })
    .await
    .map_err(|e| AppError::InternalServerError(format!("PAM task failed: {}", e)))?;

    match result {
        Ok(_) => {
            info!("User authenticated via PAM");
            Ok(())
        }
        Err(e) => {
            error!("PAM authentication failed: {}", e);
            Err(AppError::Unauthorized("Invalid credentials".to_string()))
        }
    }
}
