//! Network Actor for serialized network configuration operations.
//!
//! This actor ensures all network operations (ipadm, dladm, route, DNS)
//! are processed sequentially to avoid race conditions.

use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, instrument};

use crate::adapters;
use crate::core::{NetworkConfig, NetworkInterface, PhysicalLink};
use crate::error::AppError;

/// Messages handled by the NetworkActor.
#[derive(Debug)]
pub enum NetworkMessage {
    /// List combined network interfaces (dladm + ipadm merged)
    ListInterfaces {
        resp: oneshot::Sender<Result<Vec<NetworkInterface>, AppError>>,
    },
    /// List physical links only (dladm)
    ListPhysicalLinks {
        resp: oneshot::Sender<Result<Vec<PhysicalLink>, AppError>>,
    },
    /// Get system network config (DNS, gateway, hostname)
    GetConfig {
        resp: oneshot::Sender<Result<NetworkConfig, AppError>>,
    },
    /// Set static IP address on an interface
    SetStaticAddress {
        interface: String,
        address: String,
        prefix_len: u8,
        resp: oneshot::Sender<Result<(), AppError>>,
    },
    /// Configure DHCP on an interface
    SetDhcp {
        interface: String,
        resp: oneshot::Sender<Result<(), AppError>>,
    },
    /// Set DNS servers and search domains
    SetDns {
        servers: Vec<String>,
        search: Vec<String>,
        resp: oneshot::Sender<Result<(), AppError>>,
    },
    /// Set default gateway
    SetGateway {
        gateway: String,
        resp: oneshot::Sender<Result<(), AppError>>,
    },
    /// Set MTU on a physical link
    SetMtu {
        link: String,
        mtu: u32,
        resp: oneshot::Sender<Result<(), AppError>>,
    },
    /// Set hostname
    SetHostname {
        hostname: String,
        resp: oneshot::Sender<Result<(), AppError>>,
    },
}

/// Handle to communicate with the NetworkActor.
#[derive(Clone, Debug)]
pub struct NetworkActorHandle {
    tx: mpsc::Sender<NetworkMessage>,
}

impl NetworkActorHandle {
    /// List all network interfaces with merged dladm/ipadm data.
    pub async fn list_interfaces(&self) -> Result<Vec<NetworkInterface>, AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(NetworkMessage::ListInterfaces { resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Network actor unavailable: {e}")))?;
        rx.await.map_err(|e| {
            AppError::ServiceUnavailable(format!("Network actor channel closed: {e}"))
        })?
    }

    /// List physical network links.
    pub async fn list_physical_links(&self) -> Result<Vec<PhysicalLink>, AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(NetworkMessage::ListPhysicalLinks { resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Network actor unavailable: {e}")))?;
        rx.await.map_err(|e| {
            AppError::ServiceUnavailable(format!("Network actor channel closed: {e}"))
        })?
    }

    /// Get system network configuration.
    pub async fn get_config(&self) -> Result<NetworkConfig, AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(NetworkMessage::GetConfig { resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Network actor unavailable: {e}")))?;
        rx.await.map_err(|e| {
            AppError::ServiceUnavailable(format!("Network actor channel closed: {e}"))
        })?
    }

    /// Set a static IP address on an interface.
    pub async fn set_static_address(
        &self,
        interface: String,
        address: String,
        prefix_len: u8,
    ) -> Result<(), AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(NetworkMessage::SetStaticAddress {
                interface,
                address,
                prefix_len,
                resp,
            })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Network actor unavailable: {e}")))?;
        rx.await.map_err(|e| {
            AppError::ServiceUnavailable(format!("Network actor channel closed: {e}"))
        })?
    }

    /// Configure DHCP on an interface.
    pub async fn set_dhcp(&self, interface: String) -> Result<(), AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(NetworkMessage::SetDhcp { interface, resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Network actor unavailable: {e}")))?;
        rx.await.map_err(|e| {
            AppError::ServiceUnavailable(format!("Network actor channel closed: {e}"))
        })?
    }

    /// Set DNS servers and search domains.
    pub async fn set_dns(&self, servers: Vec<String>, search: Vec<String>) -> Result<(), AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(NetworkMessage::SetDns {
                servers,
                search,
                resp,
            })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Network actor unavailable: {e}")))?;
        rx.await.map_err(|e| {
            AppError::ServiceUnavailable(format!("Network actor channel closed: {e}"))
        })?
    }

    /// Set the default gateway.
    pub async fn set_gateway(&self, gateway: String) -> Result<(), AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(NetworkMessage::SetGateway { gateway, resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Network actor unavailable: {e}")))?;
        rx.await.map_err(|e| {
            AppError::ServiceUnavailable(format!("Network actor channel closed: {e}"))
        })?
    }

    /// Set MTU on a physical link.
    pub async fn set_mtu(&self, link: String, mtu: u32) -> Result<(), AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(NetworkMessage::SetMtu { link, mtu, resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Network actor unavailable: {e}")))?;
        rx.await.map_err(|e| {
            AppError::ServiceUnavailable(format!("Network actor channel closed: {e}"))
        })?
    }

    /// Set the system hostname.
    pub async fn set_hostname(&self, hostname: String) -> Result<(), AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(NetworkMessage::SetHostname { hostname, resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Network actor unavailable: {e}")))?;
        rx.await.map_err(|e| {
            AppError::ServiceUnavailable(format!("Network actor channel closed: {e}"))
        })?
    }
}

/// Start the NetworkActor and return a handle.
#[instrument(skip_all)]
pub fn start_network_actor() -> NetworkActorHandle {
    let (tx, mut rx) = mpsc::channel(16);

    tokio::spawn(async move {
        info!(target: "yanos::network_actor", "Network actor started");

        while let Some(msg) = rx.recv().await {
            match msg {
                NetworkMessage::ListInterfaces { resp } => {
                    debug!(target: "yanos::network_actor", "Listing network interfaces");
                    let result = adapters::get_network_interfaces();
                    let _ = resp.send(result);
                }
                NetworkMessage::ListPhysicalLinks { resp } => {
                    debug!(target: "yanos::network_actor", "Listing physical links");
                    let result = adapters::network::get_physical_links();
                    let _ = resp.send(result);
                }
                NetworkMessage::GetConfig { resp } => {
                    debug!(target: "yanos::network_actor", "Getting network config");
                    let result = adapters::network::get_network_config();
                    let _ = resp.send(result);
                }
                NetworkMessage::SetStaticAddress {
                    interface,
                    address,
                    prefix_len,
                    resp,
                } => {
                    info!(
                        target: "yanos::network_actor",
                        %interface,
                        %address,
                        prefix_len,
                        "Setting static address"
                    );
                    let result =
                        adapters::network::set_static_address(&interface, &address, prefix_len);
                    let _ = resp.send(result);
                }
                NetworkMessage::SetDhcp { interface, resp } => {
                    info!(
                        target: "yanos::network_actor",
                        %interface,
                        "Configuring DHCP"
                    );
                    let result = adapters::network::set_dhcp(&interface);
                    let _ = resp.send(result);
                }
                NetworkMessage::SetDns {
                    servers,
                    search,
                    resp,
                } => {
                    info!(
                        target: "yanos::network_actor",
                        ?servers,
                        ?search,
                        "Setting DNS configuration"
                    );
                    let result = adapters::network::set_dns_config(&servers, &search);
                    let _ = resp.send(result);
                }
                NetworkMessage::SetGateway { gateway, resp } => {
                    info!(
                        target: "yanos::network_actor",
                        %gateway,
                        "Setting default gateway"
                    );
                    let result = adapters::network::set_default_gateway(&gateway);
                    let _ = resp.send(result);
                }
                NetworkMessage::SetMtu { link, mtu, resp } => {
                    info!(
                        target: "yanos::network_actor",
                        %link,
                        mtu,
                        "Setting MTU"
                    );
                    let result = adapters::network::set_mtu(&link, mtu);
                    let _ = resp.send(result);
                }
                NetworkMessage::SetHostname { hostname, resp } => {
                    info!(
                        target: "yanos::network_actor",
                        %hostname,
                        "Setting hostname"
                    );
                    let result = adapters::network::set_hostname(&hostname);
                    let _ = resp.send(result);
                }
            }
        }

        info!(target: "yanos::network_actor", "Network actor stopped");
    });

    NetworkActorHandle { tx }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_actor_list_interfaces() {
        let actor = start_network_actor();
        let result = actor.list_interfaces().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_network_actor_list_physical_links() {
        let actor = start_network_actor();
        let result = actor.list_physical_links().await;
        // May fail on VMs without physical NICs, just verify it doesn't panic
        match result {
            Ok(links) => {
                println!("Found {} physical links", links.len());
            }
            Err(e) => {
                println!("list_physical_links returned error (may be expected on VM): {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_network_actor_get_config() {
        let actor = start_network_actor();
        let result = actor.get_config().await;
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(!config.hostname.is_empty());
    }

    #[tokio::test]
    async fn test_network_actor_handle_clone() {
        let actor = start_network_actor();
        let cloned = actor.clone();

        let r1 = actor.list_interfaces().await;
        let r2 = cloned.list_interfaces().await;

        assert!(r1.is_ok());
        assert!(r2.is_ok());
    }

    #[test]
    fn test_network_message_debug() {
        let (tx, _) = oneshot::channel();
        let msg = NetworkMessage::ListInterfaces { resp: tx };
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("ListInterfaces"));
    }
}
