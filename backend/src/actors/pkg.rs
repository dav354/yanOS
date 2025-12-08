use tokio::sync::{mpsc, oneshot};
use tracing::debug;

use crate::adapters;
use crate::core::PackageInfo;
use crate::error::AppError;

#[derive(Debug)]
pub enum PkgMessage {
    Update {
        resp: oneshot::Sender<Result<(), AppError>>,
    },
    List {
        resp: oneshot::Sender<Result<Vec<PackageInfo>, AppError>>,
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
}

pub fn start_pkg_actor() -> PkgActorHandle {
    let (tx, mut rx) = mpsc::channel(4);
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                PkgMessage::Update { resp } => {
                    debug!(target: "zos::pkg_actor", "Package update requested (not implemented)");
                    let _ = resp.send(Ok(()));
                }
                PkgMessage::List { resp } => {
                    debug!(target: "zos::pkg_actor", "Package list requested");
                    let result = adapters::get_pkg_list();
                    let _ = resp.send(result);
                }
            }
        }
    });
    PkgActorHandle { tx }
}
