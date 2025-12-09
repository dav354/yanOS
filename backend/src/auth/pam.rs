use std::ffi::CString;
use std::ptr;

use axum::{Json, Router, routing::post};
use pam_sys::{
    PAM_CONV_ERR, PAM_PROMPT_ECHO_OFF, PAM_SUCCESS, pam_authenticate, pam_end, pam_handle_t,
    pam_message, pam_response, pam_start,
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
    router.route("/api/v1/login", post(login_handler))
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
    info!(username = %payload.username, "Attempting to authenticate user");

    let username = payload.username.clone();
    let password = payload.password.clone();

    // The PAM authentication needs to be run in a blocking thread
    // to avoid blocking the async runtime.
    let result = tokio::task::spawn_blocking(move || {
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
            let retval = pam_start(
                c"login".as_ptr(), // Service name
                c_user.as_ptr(),
                &conv,
                &mut pam_h,
            );

            if retval != PAM_SUCCESS_I32 {
                return Err("Failed to start PAM transaction".to_string());
            }

            // 2. Authenticate
            let retval = pam_authenticate(pam_h, 0);

            if retval != PAM_SUCCESS_I32 {
                let _ = pam_end(pam_h, retval);
                return Err("Authentication failed".to_string());
            }

            // 3. Account Management (optional but recommended)
            // let retval = pam_acct_mgmt(pam_h, 0);
            // if retval != PAM_SUCCESS { ... }

            // 4. End PAM transaction
            pam_end(pam_h, PAM_SUCCESS_I32);
            Ok(())
        }
    })
    .await
    .map_err(|e| AppError::InternalServerError(format!("PAM task failed: {}", e)))?;

    match result {
        Ok(_) => {
            info!(username = %payload.username, "User authenticated successfully");
            // Store username in the session.
            session
                .insert("username", &payload.username)
                .await
                .map_err(|e| {
                    AppError::InternalServerError(format!("Failed to insert into session: {}", e))
                })?;
            Ok(Json("Authentication successful"))
        }
        Err(e) => {
            error!(username = %payload.username, "Authentication failed: {}", e);
            Err(AppError::Unauthorized("Invalid credentials".to_string()))
        }
    }
}
