pub mod kstat;
pub mod network;
pub mod pkg;
pub mod system;
pub mod zfs;

pub use network::get_network_interfaces;
pub use pkg::{get_pkg_list, get_pkg_updates};
pub use system::{get_hostname, get_system_info};
pub use zfs::{DatasetInfo, LibZfsHandle, PoolInfo, get_dataset, get_pool, list_datasets, list_pools};
