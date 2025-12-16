//! Terminal actor for web-based shell sessions.
//!
//! This module provides a PTY-based terminal session that can be accessed
//! over WebSocket. Each session spawns a login shell for the authenticated
//! user via `su -` to ensure proper environment initialization.
//!
//! # Architecture
//! - `TerminalActorHandle` - async handle for sending commands to the session
//! - `TerminalSession` - contains the handle and output receiver
//! - PTY I/O runs on a blocking thread, bridged to async via channels
//!
//! # Security
//! The user must already be authenticated via PAM before a terminal session
//! is created. Authentication is checked in the WebSocket handler, not here.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument};

use crate::error::AppError;

/// Messages sent to the terminal actor.
#[derive(Debug)]
pub enum TerminalMessage {
    /// User input data (keystrokes) to write to the PTY
    Input(String),
    /// Resize the PTY to match the client's terminal dimensions
    Resize { rows: u16, cols: u16 },
    /// Gracefully terminate the session
    Shutdown,
}

/// Handle for communicating with a terminal session.
/// Clone-able to allow multiple references (e.g., input and resize handlers).
#[derive(Clone, Debug)]
pub struct TerminalActorHandle {
    tx: mpsc::Sender<TerminalMessage>,
}

impl TerminalActorHandle {
    pub async fn send_input(&self, data: String) -> Result<(), AppError> {
        self.tx
            .send(TerminalMessage::Input(data))
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("terminal actor unavailable: {e}")))
    }

    pub async fn resize(&self, rows: u16, cols: u16) -> Result<(), AppError> {
        self.tx
            .send(TerminalMessage::Resize { rows, cols })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("terminal actor unavailable: {e}")))
    }

    pub async fn shutdown(&self) {
        let _ = self.tx.send(TerminalMessage::Shutdown).await;
    }
}

pub struct TerminalSession {
    pub handle: TerminalActorHandle,
    pub output: mpsc::Receiver<Vec<u8>>,
}

#[instrument(skip(username))]
pub fn start_terminal_session(username: String) -> Result<TerminalSession, AppError> {
    info!(target: "yanos::terminal", user = %username, "Starting terminal session");

    let (tx, mut rx) = mpsc::channel::<TerminalMessage>(32);
    let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>(64);

    let pty_system = NativePtySystem::default();

    // Build shell command for the authenticated user.
    // SECURITY NOTE: The user has already been authenticated via PAM in the WebSocket
    // handshake (ws_handler checks session). We spawn a login shell as that user.
    // On illumos, `login -f` requires console access, so we use `su -` instead
    // which respects PAM and properly initializes the user environment.
    let build_su_shell = || {
        let mut cmd = CommandBuilder::new("/usr/bin/su");
        cmd.arg("-"); // Login shell
        cmd.arg(&username);
        cmd.env("TERM", "xterm-256color");
        cmd.env("LANG", "en_US.UTF-8");
        cmd
    };

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| AppError::InternalServerError(format!("openpty failed: {e}")))?;

    // Spawn shell via su which handles user switching properly on illumos.
    // The web service must run as root (or have appropriate privileges) for this to work.
    let mut child = pair
        .slave
        .spawn_command(build_su_shell())
        .map_err(|e| AppError::InternalServerError(format!("Failed to spawn shell for user '{}': {e}", username)))?;

    drop(pair.slave);

    let master = pair.master;
    let reader = master
        .try_clone_reader()
        .map_err(|e| AppError::InternalServerError(format!("clone reader failed: {e}")))?;
    let mut writer = master
        .take_writer()
        .map_err(|e| AppError::InternalServerError(format!("take writer failed: {e}")))?;
    let master_for_resize = Arc::new(Mutex::new(master));

    // PTY -> channel on blocking thread
    {
        let out_tx = out_tx.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut reader = reader;
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if out_tx.blocking_send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
    }

    // Actor loop
    tokio::spawn(async move {
        debug!(target: "yanos::terminal", "Terminal actor loop started");

        while let Some(msg) = rx.recv().await {
            match msg {
                TerminalMessage::Input(data) => {
                    debug!(target: "yanos::terminal", bytes = data.len(), "Received input");
                    if let Err(e) = writer.write_all(data.as_bytes()) {
                        error!(target: "yanos::terminal", error = ?e, "Terminal write failed");
                        break;
                    }
                    let _ = writer.flush();
                }
                TerminalMessage::Resize { rows, cols } => {
                    debug!(target: "yanos::terminal", rows, cols, "Resizing terminal");
                    if let Ok(master) = master_for_resize.lock() {
                        let _ = master.resize(PtySize {
                            rows,
                            cols,
                            pixel_width: 0,
                            pixel_height: 0,
                        });
                    }
                }
                TerminalMessage::Shutdown => {
                    info!(target: "yanos::terminal", "Shutdown requested");
                    break;
                }
            }
        }

        info!(target: "yanos::terminal", "Cleaning up terminal session");
        let _ = child.kill();
        let _ = child.wait();
    });

    Ok(TerminalSession {
        handle: TerminalActorHandle { tx },
        output: out_rx,
    })
}
