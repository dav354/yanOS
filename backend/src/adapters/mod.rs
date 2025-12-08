pub mod network;
pub mod pkg;
pub mod system;

pub use network::get_network_interfaces;
pub use pkg::get_pkg_list;
pub use system::{get_hostname, get_system_info};
