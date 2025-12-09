use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::select;
use tower_sessions::Session;
use tracing::{instrument, warn};

use crate::actors::start_terminal_session;
use crate::api::AppState;
use crate::error::AppError;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "input")]
    Input { data: String },
    #[serde(rename = "resize")]
    Resize { rows: u16, cols: u16 },
}

#[instrument(skip(ws, session))]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(_state): State<AppState>,
    session: Session,
) -> Result<impl IntoResponse, AppError> {
    // Authenticate and get user
    let username: String = session
        .get("username")
        .await
        .map_err(|e| AppError::InternalServerError(format!("Session error: {e}")))?
        .ok_or(AppError::Unauthorized("Not logged in".to_string()))?;

    Ok(ws.on_upgrade(move |socket| handle_terminal(socket, username)))
}

async fn handle_terminal(socket: WebSocket, username: String) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let session = match start_terminal_session(username.clone()) {
        Ok(session) => session,
        Err(e) => {
            let _ = ws_sender
                .send(Message::Text(
                    format!("Failed to start terminal: {e:?}").into(),
                ))
                .await;
            return;
        }
    };

    let mut output_rx = session.output;
    let handle = session.handle;

    loop {
        select! {
            Some(msg) = ws_receiver.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(cmd) = serde_json::from_str::<ClientMessage>(&text) {
                            match cmd {
                                ClientMessage::Input { data } => {
                                    if let Err(e) = handle.send_input(data).await {
                                        warn!(error = ?e, "failed to send input to terminal actor");
                                        break;
                                    }
                                }
                                ClientMessage::Resize { rows, cols } => {
                                    if let Err(e) = handle.resize(rows, cols).await {
                                        warn!(error = ?e, "failed to resize terminal");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Binary(bin)) => {
                        if let Ok(text) = String::from_utf8(bin.to_vec()) {
                            let _ = handle.send_input(text).await;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        break;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        warn!(error = ?e, "websocket recv error");
                        break;
                    }
                }
            },
            maybe_output = output_rx.recv() => {
                match maybe_output {
                    Some(data) => {
                        if ws_sender.send(Message::Binary(data.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    handle.shutdown().await;
}
