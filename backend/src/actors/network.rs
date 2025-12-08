use tokio::sync::{mpsc, oneshot};
use tracing::info;

use crate::adapters;
use crate::core::NetworkInterface;
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

#[derive(Clone, Debug)]
pub struct NetworkActorHandle {
    tx: mpsc::Sender<NetworkMessage>,
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
                    info!(
                        target: "zos::network_actor",
                        %interface,
                        %address,
                        "SetAddress requested (not implemented)"
                    );
                    let _ = resp.send(Ok(()));
                }
            }
        }
    });
    NetworkActorHandle { tx }
}
