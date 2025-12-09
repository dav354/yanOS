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
    responses(
        (status = 101, description = "WebSocket Upgrade")
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

    for point in history {
        if let Ok(json) = serde_json::to_string(&point) {
            if socket.send(Message::Text(json.into())).await.is_err() {
                return;
            }
        }
    }

    let mut rx = state.broadcast_tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&msg) {
            if socket.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    }
}
