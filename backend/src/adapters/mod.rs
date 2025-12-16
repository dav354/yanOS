pub mod kstat;
pub mod network;
pub mod pkg;
pub mod system;
pub mod zfs;

pub use network::{
    get_network_addresses, get_network_config, get_network_interfaces, get_physical_links,
    set_default_gateway, set_dhcp, set_dns_config, set_static_address,
};
pub use pkg::{get_pkg_list, get_pkg_updates};
pub use system::{get_hostname, get_system_info};
pub use zfs::{
    get_dataset, get_pool, list_datasets, list_pools, DatasetInfo, LibZfsHandle, PoolInfo,
};
