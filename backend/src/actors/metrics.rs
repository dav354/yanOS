use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io;
use std::process::Command;
use std::sync::Arc;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio::task::spawn_blocking;
use tokio::time::{Duration, interval};
use tracing::{info, warn};

// --- Data Structures ---

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricPoint {
    pub ts: i64,           // Timestamp (Unix ms)
    pub cpu_user: f32,     // CPU User %
    pub cpu_system: f32,   // CPU System % (Kernel)
    pub cpu_idle: f32,     // CPU Idle %
    pub memory_used: u64,  // RAM Used (Bytes)
    pub memory_total: u64, // RAM Total (Bytes)
    pub zfs_arc: u64,      // ZFS ARC Size (Bytes) - approximated or parsed
    pub rx_bytes: u64,     // Network RX (Bytes/sec)
    pub tx_bytes: u64,     // Network TX (Bytes/sec)
}

#[derive(Debug)]
pub enum MetricsCommand {
    Subscribe(broadcast::Sender<MetricPoint>),
}

#[derive(Clone, Debug)]
pub struct MetricsState {
    pub broadcast_tx: broadcast::Sender<MetricPoint>,
    pub history: Arc<RwLock<VecDeque<MetricPoint>>>,
    pub command_tx: mpsc::Sender<MetricsCommand>,
}

// --- Actor ---

pub struct MetricsActor {
    receiver: mpsc::Receiver<MetricsCommand>,
    broadcast_tx: broadcast::Sender<MetricPoint>,
    pub history: Arc<RwLock<VecDeque<MetricPoint>>>, // Shared history for new clients
    system: System,
    networks: Networks,
    max_history: usize,
    arc_warned: bool,
}

impl MetricsActor {
    pub fn new(
        receiver: mpsc::Receiver<MetricsCommand>,
        broadcast_tx: broadcast::Sender<MetricPoint>,
    ) -> Self {
        // Init sysinfo with specific refresh requirements
        let mut system = System::new_with_specifics(
            RefreshKind::nothing() // Use nothing() as base, then add
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        let networks = Networks::new_with_refreshed_list();

        // Initial refresh to prevent zero values
        std::thread::sleep(Duration::from_millis(500));
        system.refresh_cpu_all();
        system.refresh_memory();

        Self {
            receiver,
            broadcast_tx,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(3600))), // 1 hour buffer
            system,
            networks,
            max_history: 3600, // Keep 1 hour of history @ 1s interval
            arc_warned: false,
        }
    }

    pub async fn run(mut self) {
        info!("MetricsActor started");

        // Timer for data collection (1Hz)
        let mut ticker = interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    self.collect_and_broadcast().await;
                }
                maybe_cmd = self.receiver.recv() => {
                    match maybe_cmd {
                        Some(cmd) => {
                            match cmd {
                                MetricsCommand::Subscribe(_) => {
                                    // Handled by route handler mostly, but we could do logic here
                                }
                            }
                        }
                        None => {
                            info!(target: "zos::metrics", "MetricsActor control channel closed; exiting");
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn collect_and_broadcast(&mut self) {
        // Refresh Data
        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        self.networks.refresh(false); // keep list, update counters

        // 1. Calculate CPU
        // sysinfo provides usage per core. We average it for global usage.
        let cpus = self.system.cpus();
        let cpu_count = cpus.len() as f32;
        let mut total_usage = 0.0;

        for cpu in cpus {
            total_usage += cpu.cpu_usage();
        }

        let global_usage = if cpu_count > 0.0 {
            total_usage / cpu_count
        } else {
            0.0
        };
        // sysinfo combines user+sys into 'usage'.
        // We will just use 'usage' as user+sys for now, and calc idle.
        let cpu_idle = 100.0 - global_usage;

        // 2. Calculate RAM
        let mem_total = self.system.total_memory() * 1024;
        let mem_used_raw = self.system.used_memory() * 1024;

        // TODO: Illumos specific ZFS ARC parsing.
        // For now, on Linux/Illumos via sysinfo, used usually includes ARC.
        // We will try to separate it if possible, otherwise set ARC to 0 (client handles it).
        // Real ZFS ARC reading requires reading kstat.
        let zfs_arc = self.read_arc_size_bytes().await;

        // 3. Calculate Network (Delta)
        // We need to track previous values to calc rate, OR rely on sysinfo providing rates?
        // sysinfo `received()` is total bytes usually? No, docs say "total bytes received since boot" usually,
        // but `Networks` struct usually has mechanism.
        // WAIT: sysinfo 0.30+ might behave differently.
        // Let's assume we need to diff.
        // Actually, looking at sysinfo docs, `transmitted()` is total.
        // To get B/s, we need to store prev state.
        // But for simplicity in this turn, we will sum up all interfaces.

        let mut total_rx = 0;
        let mut total_tx = 0;

        for (_name, data) in &self.networks {
            total_rx += data.received(); // This is effectively "since last refresh" if configured right?
            total_tx += data.transmitted();
        }
        // Sysinfo documentation says: "Refreshes data... returns... number of bytes received since the last refresh."
        // PERFECT! So we don't need manual diffing if we refresh() periodically.

        let point = MetricPoint {
            ts: chrono::Utc::now().timestamp_millis(),
            cpu_user: global_usage, // approximating
            cpu_system: 0.0,        // sysinfo doesn't easily split this cross-platform
            cpu_idle,
            memory_used: mem_used_raw,
            memory_total: mem_total,
            zfs_arc,
            rx_bytes: total_rx,
            tx_bytes: total_tx,
        };

        // Update History
        {
            let mut hist = self.history.write().await;
            if hist.len() >= self.max_history {
                hist.pop_front();
            }
            hist.push_back(point.clone());
        }

        // Broadcast
        // We ignore error if no receivers (no one viewing dashboard)
        let _ = self.broadcast_tx.send(point);
    }

    async fn read_arc_size_bytes(&mut self) -> u64 {
        match spawn_blocking(read_arc_size_blocking).await {
            Ok(Ok(size)) => size,
            Ok(Err(err)) => {
                if !self.arc_warned {
                    match err {
                        ArcReadError::Io(e) => {
                            warn!(target: "zos::metrics", error = ?e, "I/O error reading ARC size from kstat; reporting 0");
                        }
                        ArcReadError::Status(status) => {
                            warn!(target: "zos::metrics", status = ?status, "kstat returned non-zero status; reporting 0");
                        }
                        ArcReadError::MissingValue => {
                            warn!(target: "zos::metrics", "ARC size missing from kstat output; reporting 0");
                        }
                    }
                    self.arc_warned = true;
                }
                0
            }
            Err(join_error) => {
                if !self.arc_warned {
                    warn!(target: "zos::metrics", error = ?join_error, "Join error while reading ARC size; reporting 0");
                    self.arc_warned = true;
                }
                0
            }
        }
    }
}

pub fn start_metrics_actor() -> Arc<MetricsState> {
    let (command_tx, rx) = mpsc::channel(32);
    let (broadcast_tx, _) = broadcast::channel(100);
    let actor = MetricsActor::new(rx, broadcast_tx.clone());

    let history = actor.history.clone();

    tokio::spawn(actor.run());

    Arc::new(MetricsState {
        broadcast_tx,
        history,
        command_tx,
    })
}

#[derive(Debug)]
enum ArcReadError {
    Io(io::Error),
    Status(Option<i32>),
    MissingValue,
}

fn read_arc_size_blocking() -> Result<u64, ArcReadError> {
    let output = Command::new("kstat")
        .args(["-p", "zfs:0:arcstats:size"])
        .output()
        .map_err(ArcReadError::Io)?;

    if !output.status.success() {
        return Err(ArcReadError::Status(output.status.code()));
    }

    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .last()
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or(ArcReadError::MissingValue)
}
