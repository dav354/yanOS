use axum::extract::{
    State, WebSocketUpgrade,
    ws::{Message, WebSocket},
};
use serde_json::json;
use tokio::sync::broadcast;
use tower_sessions::Session;
use tracing::instrument;

use crate::api::state::AppState;
use crate::error::AppError;
use crate::events::ExternalEvent;

/// Stream external events to the UI via WebSocket.
#[instrument(skip(state, ws, session))]
pub async fn stream_events(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
    session: Session,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let username: Option<String> = session.get("username").await.unwrap_or(None);
    if username.is_none() {
        return Err(AppError::Unauthorized(
            "Authentication required".to_string(),
        ));
    }

    let rx = state.event_bus.subscribe();
    Ok(ws.on_upgrade(move |socket| async move {
        handle_event_socket(socket, rx).await;
    }))
}

async fn handle_event_socket(stream: WebSocket, mut rx: broadcast::Receiver<ExternalEvent>) {
    let mut socket = stream;
    while let Ok(event) = rx.recv().await {
        let payload = match event {
            ExternalEvent::ConfigChanged(path) => json!({
                "type": "config_changed",
                "path": path
            }),
        };
        if socket
            .send(Message::Text(payload.to_string().into()))
            .await
            .is_err()
        {
            break;
        }
    }
}
