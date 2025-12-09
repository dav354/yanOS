use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use tokio::sync::mpsc;
use tracing::{error, info, instrument};

use crate::error::AppError;

#[derive(Debug)]
pub enum TerminalMessage {
    Input(String),
    Resize { rows: u16, cols: u16 },
    Shutdown,
}

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
    let (tx, mut rx) = mpsc::channel::<TerminalMessage>(32);
    let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>(64);

    let pty_system = NativePtySystem::default();

    let mut cmd = CommandBuilder::new("/usr/bin/login");
    cmd.arg("-f");
    cmd.arg(&username);
    cmd.env("TERM", "xterm-256color");
    cmd.env("LANG", "en_US.UTF-8");

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| AppError::InternalServerError(format!("openpty failed: {e}")))?;

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| AppError::InternalServerError(format!("spawn login failed: {e}")))?;

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
        while let Some(msg) = rx.recv().await {
            match msg {
                TerminalMessage::Input(data) => {
                    if let Err(e) = writer.write_all(data.as_bytes()) {
                        error!(error = ?e, "terminal write failed");
                        break;
                    }
                    let _ = writer.flush();
                }
                TerminalMessage::Resize { rows, cols } => {
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
                    info!(target: "zos::terminal_actor", "shutdown requested");
                    break;
                }
            }
        }

        let _ = child.kill();
        let _ = child.wait();
    });

    Ok(TerminalSession {
        handle: TerminalActorHandle { tx },
        output: out_rx,
    })
}
