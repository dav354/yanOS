use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tracing::instrument;

use crate::events::EventBus;

#[instrument(skip(event_bus))]
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
