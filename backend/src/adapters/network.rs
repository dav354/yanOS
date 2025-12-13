use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::core::NetworkInterface;
use crate::error::AppError;

pub fn get_network_interfaces() -> Result<Vec<NetworkInterface>, AppError> {
    for attempt in 1..=3 {
        let output = Command::new("ipadm")
            .args(["show-addr", "-p", "-o", "addrobj,state,addr"])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let parsed = String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .filter_map(|line| {
                        let parts: Vec<&str> = line.split(':').collect();
                        if parts.len() >= 3 {
                            let name = parts[0].to_string();
                            // Filter out loopback interfaces (e.g., lo0/v4, lo0/v6)
                            if name.starts_with("lo") {
                                return None;
                            }
                            Some(NetworkInterface {
                                name,
                                state: parts[1].to_string(),
                                address: parts[2].to_string(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect();
                return Ok(parsed);
            }
        }

        if attempt < 3 {
            thread::sleep(Duration::from_millis(200));
        }
    }

    Err(AppError::ServiceUnavailable(
        "Failed to query network interfaces via ipadm".to_string(),
    ))
}
