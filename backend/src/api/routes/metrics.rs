use crate::actors::MetricsState;
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use std::sync::Arc;

#[utoipa::path(
    get,
    path = "/api/v1/metrics/live",
    tag = "metrics",
    description = "Stream live metrics via WebSocket. \n\n**Note:** This endpoint requires a WebSocket client connection (Upgrade: websocket). Standard HTTP requests (like Swagger UI 'Try it out') will fail with `400 Bad Request`.",
    responses(
        (status = 101, description = "WebSocket Upgrade"),
        (status = 400, description = "Bad Request (Missing Upgrade header)")
    ),
    security(
        ("basic_auth" = [])
    )
)]
pub async fn live_metrics(
    ws: WebSocketUpgrade,
    State(state): State<Arc<MetricsState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<MetricsState>) {
    let history = {
        let h = state.history.read().await;
        let len = h.len();
        let keep = 240.min(len); // cap initial dump to reduce client load
        h.iter()
            .skip(len.saturating_sub(keep))
            .cloned()
            .collect::<Vec<_>>()
    };

    // Send history as a single batch
    if !history.is_empty()
        && let Ok(json) = serde_json::to_string(&history)
            && socket.send(Message::Text(json.into())).await.is_err() {
                return;
            }

    let mut rx = state.broadcast_tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&msg)
            && socket.send(Message::Text(json.into())).await.is_err() {
                break;
            }
    }
}
