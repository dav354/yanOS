use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info};

use crate::adapters;
use crate::core::{NetworkInterface, PackageInfo};
use crate::error::AppError;

#[derive(Debug)]
pub enum NetworkMessage {
    ReadInterfaces {
        resp: oneshot::Sender<Result<Vec<NetworkInterface>, AppError>>,
    },
    SetAddress {
        interface: String,
        address: String,
        resp: oneshot::Sender<Result<(), AppError>>,
    },
}

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
pub struct NetworkActorHandle {
    tx: mpsc::Sender<NetworkMessage>,
}

#[derive(Clone, Debug)]
pub struct PkgActorHandle {
    tx: mpsc::Sender<PkgMessage>,
}

impl NetworkActorHandle {
    pub async fn list_interfaces(&self) -> Result<Vec<NetworkInterface>, AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(NetworkMessage::ReadInterfaces { resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Network actor unavailable: {e}")))?;
        rx.await.map_err(|e| {
            AppError::ServiceUnavailable(format!("Network actor channel closed: {e}"))
        })?
    }

    pub async fn set_address(&self, interface: String, address: String) -> Result<(), AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(NetworkMessage::SetAddress {
                interface,
                address,
                resp,
            })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Network actor unavailable: {e}")))?;
        rx.await.map_err(|e| {
            AppError::ServiceUnavailable(format!("Network actor channel closed: {e}"))
        })?
    }
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

pub fn start_network_actor() -> NetworkActorHandle {
    let (tx, mut rx) = mpsc::channel(8);
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                NetworkMessage::ReadInterfaces { resp } => {
                    let result = adapters::get_network_interfaces();
                    let _ = resp.send(result);
                }
                NetworkMessage::SetAddress {
                    interface,
                    address,
                    resp,
                } => {
                    info!(target: "zos::network_actor", %interface, %address, "SetAddress requested (not implemented)");
                    let _ = resp.send(Ok(()));
                }
            }
        }
    });
    NetworkActorHandle { tx }
}

pub fn start_pkg_actor() -> PkgActorHandle {
    let (tx, mut rx) = mpsc::channel(4);
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                PkgMessage::Update { resp } => {
                    info!(target: "zos::pkg_actor", "Package update requested (not implemented)");
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
