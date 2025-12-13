use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tracing::instrument;
use utoipa::ToSchema;

use crate::events::EventBus;

#[derive(ToSchema)]
struct EventStreamUpgrade;

#[instrument(skip(event_bus))]
#[utoipa::path(
    get,
    path = "/api/v1/events",
    tag = "events",
    description = "Stream system events (logs, config changes, etc.) via WebSocket. \n\n**Note:** This endpoint requires a WebSocket client connection (Upgrade: websocket). Standard HTTP requests will fail with `400 Bad Request`.",
    responses(
        (status = 101, description = "WebSocket upgrade for live events", body = EventStreamUpgrade),
        (status = 400, description = "Bad Request (Missing Upgrade header)")
    ),
    security(
        ("basic_auth" = [])
    )
)]
pub async fn stream_events(
    ws: WebSocketUpgrade,
    State(event_bus): State<EventBus>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, event_bus))
}

async fn handle_socket(socket: WebSocket, event_bus: EventBus) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = event_bus.subscribe();

    // Spawn a task to handle incoming messages (e.g. pings/close)
    let send_task = tokio::spawn(async move {
        // send recent snapshot first
        for entry in event_bus.snapshot(200) {
            if let Ok(msg) = serde_json::to_string(&entry) {
                if sender.send(Message::Text(msg.into())).await.is_err() {
                    return;
                }
            }
        }

        while let Ok(event) = rx.recv().await {
            if let Ok(msg) = serde_json::to_string(&event) {
                if sender.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Wait for the client to disconnect
    while let Some(Ok(_msg)) = receiver.next().await {
        // We can handle control messages here if needed
    }

    send_task.abort();
}
