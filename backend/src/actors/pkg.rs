use time::OffsetDateTime;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info};
use uuid::Uuid;

use crate::adapters;
use crate::core::PackageInfo;
use crate::error::AppError;
use crate::events::{EventBus, ExternalEvent};

#[derive(Debug)]
pub enum PkgMessage {
    Update {
        resp: oneshot::Sender<Result<(), AppError>>,
    },
    List {
        resp: oneshot::Sender<Result<Vec<PackageInfo>, AppError>>,
    },
    CheckUpdates,
    GetUpdates {
        resp: oneshot::Sender<Vec<PackageInfo>>,
    },
}

#[derive(Clone, Debug)]
pub struct PkgActorHandle {
    tx: mpsc::Sender<PkgMessage>,
}

impl PkgActorHandle {
    pub async fn list(&self) -> Result<Vec<PackageInfo>, AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(PkgMessage::List { resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Pkg actor unavailable: {e}")))?;
        rx.await
            .map_err(|e| AppError::ServiceUnavailable(format!("Pkg actor channel closed: {e}")))?
    }

    pub async fn update(&self) -> Result<(), AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(PkgMessage::Update { resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Pkg actor unavailable: {e}")))?;
        rx.await
            .map_err(|e| AppError::ServiceUnavailable(format!("Pkg actor channel closed: {e}")))?
    }

    pub async fn check_updates(&self) {
        let _ = self.tx.send(PkgMessage::CheckUpdates).await;
    }

    pub async fn get_updates(&self) -> Result<Vec<PackageInfo>, AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(PkgMessage::GetUpdates { resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Pkg actor unavailable: {e}")))?;
        rx.await.map_err(|e| AppError::InternalServerError(format!("Actor closed: {e}")))
    }
}

struct ActorState {
    updates: Vec<PackageInfo>,
    last_checked: Option<OffsetDateTime>,
}

pub fn start_pkg_actor(event_bus: EventBus) -> PkgActorHandle {
    let (tx, mut rx) = mpsc::channel(32);
    
    tokio::spawn(async move {
        let mut state = ActorState {
            updates: Vec::new(),
            last_checked: None,
        };

        // Initial check on startup
        info!(target: "yanos::pkg_actor", "Starting package actor, performing initial update check...");
        
        let initial_start = std::time::Instant::now();
        match adapters::get_pkg_updates() {
            Ok(up) => {
                state.updates = up;
                debug!(target: "yanos::pkg_actor", "Initial check took {:?}", initial_start.elapsed());
            },
            Err(e) => debug!(target: "yanos::pkg_actor", "Initial update check failed: {e:?}"),
        }

        while let Some(msg) = rx.recv().await {
            match msg {
                PkgMessage::Update { resp } => {
                    debug!(target: "yanos::pkg_actor", "Package update requested (not implemented)");
                    let _ = resp.send(Ok(()));
                }
                PkgMessage::List { resp } => {
                    // debug!(target: "yanos::pkg_actor", "Package list requested");
                    let result = adapters::get_pkg_list();
                    let _ = resp.send(result);
                }
                PkgMessage::CheckUpdates => {
                    info!(target: "yanos::pkg_actor", "Checking for updates...");
                    let task_id = Uuid::new_v4().to_string();
                    let task_name = "Package Update Check".to_string();
                    let start_ts = OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339).unwrap_or_default();
                    let start_time = std::time::Instant::now();

                    event_bus.publish(ExternalEvent::TaskStarted {
                        id: task_id.clone(),
                        name: task_name.clone(),
                        started_at: start_ts.clone(),
                    });

                    // Refresh catalog first to ensure we see latest updates
                    if let Err(e) = adapters::pkg::refresh_catalog() {
                        tracing::warn!(target: "zos::pkg_actor", "Failed to refresh catalog: {e:?}");
                    }

                    match adapters::get_pkg_updates() {
                        Ok(up) => {
                            let duration = start_time.elapsed().as_millis() as u64;
                            info!(target: "yanos::pkg_actor", "Found {} updates in {}ms", up.len(), duration);
                            state.updates = up;
                            state.last_checked = Some(OffsetDateTime::now_utc());

                            event_bus.publish(ExternalEvent::TaskCompleted {
                                id: task_id,
                                name: task_name,
                                started_at: start_ts,
                                duration_ms: duration,
                                status: "success".to_string(),
                            });
                        }
                        Err(e) => {
                            let duration = start_time.elapsed().as_millis() as u64;
                            tracing::error!(target: "yanos::pkg_actor", "Failed to check updates: {e:?}");
                            
                            event_bus.publish(ExternalEvent::TaskCompleted {
                                id: task_id,
                                name: task_name,
                                started_at: start_ts,
                                duration_ms: duration,
                                status: "failed".to_string(),
                            });
                        }
                    }
                }
                PkgMessage::GetUpdates { resp } => {
                    let _ = resp.send(state.updates.clone());
                }
            }
        }
    });
    PkgActorHandle { tx }
}
