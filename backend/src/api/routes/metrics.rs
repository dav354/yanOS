use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde_json::json;
use tower_sessions::Session;
use tracing::instrument;

use crate::error::AppError;

/// Stream live metrics over WebSocket (mocked if kstat not available).
#[instrument(skip(ws, session))]
pub async fn stream_metrics(
    ws: WebSocketUpgrade,
    session: Session,
) -> Result<impl IntoResponse, AppError> {
    let username: Option<String> = session.get("username").await.unwrap_or(None);
    if username.is_none() {
        return Err(AppError::Unauthorized(
            "Authentication required".to_string(),
        ));
    }

    Ok(ws.on_upgrade(move |socket| async move {
        handle_metrics_socket(socket).await;
    }))
}

fn sample_metrics() -> serde_json::Value {
    // Try kstat first (illumos); fall back to /proc on dev hosts.
    if let Ok(output) = std::process::Command::new("kstat")
        .args(["-p", "cpu_stat:::idle", "-p", "cpu_stat:::user"])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut idle: u64 = 0;
            let mut user: u64 = 0;
            for line in text.lines() {
                let mut parts = line.split_whitespace();
                if let (Some(_), Some(val)) = (parts.next(), parts.next()) {
                    if line.contains("idle") {
                        idle = val.parse().unwrap_or(0);
                    } else if line.contains("user") {
                        user = val.parse().unwrap_or(0);
                    }
                }
            }
            return json!({ "cpu_user": user, "cpu_idle": idle });
        }
    }

    // Fallback: load from /proc/stat (Linux) if present.
    if let Ok(stat) = std::fs::read_to_string("/proc/stat") {
        if let Some(line) = stat.lines().find(|l| l.starts_with("cpu ")) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let user: u64 = parts[1].parse().unwrap_or(0);
                let idle: u64 = parts[4].parse().unwrap_or(0);
                return json!({ "cpu_user": user, "cpu_idle": idle });
            }
        }
    }

    json!({ "cpu_user": 0, "cpu_idle": 0 })
}

async fn handle_metrics_socket(mut socket: WebSocket) {
    loop {
        let payload = sample_metrics();
        if socket
            .send(Message::Text(payload.to_string().into()))
            .await
            .is_err()
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
