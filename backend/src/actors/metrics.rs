//! Metrics collection actor for system monitoring.
//!
//! This actor runs a background task that collects system metrics at 1Hz:
//! - CPU utilization (aggregate and per-core) via kstat cpu:*:sys
//! - Memory usage via kstat unix:0:system_pages
//! - ZFS ARC size via kstat zfs:0:arcstats
//! - Network throughput via kstat link statistics
//!
//! Metrics are broadcast to WebSocket subscribers and kept in a rolling history
//! buffer for new clients to receive recent data on connection.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::interval;
use tracing::{info, warn};

use crate::adapters;
use crate::error::AppError;

// --- Data Structures ---

/// A single point-in-time system metrics snapshot.
/// Sent to WebSocket clients as JSON at 1Hz intervals.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricPoint {
    /// Timestamp in Unix milliseconds
    pub ts: i64,
    /// CPU user time percentage (0-100)
    pub cpu_user: f32,
    /// CPU kernel/system time percentage (0-100)
    pub cpu_system: f32,
    /// CPU idle time percentage (0-100)
    pub cpu_idle: f32,
    /// Per-core CPU breakdown for detailed monitoring
    pub per_core: Vec<CpuCoreMetric>,
    /// Physical memory in use (bytes), includes ARC
    pub memory_used: u64,
    /// Total physical memory (bytes)
    pub memory_total: u64,
    /// ZFS ARC cache size (bytes) - reclaimable memory
    pub zfs_arc: u64,
    /// Network receive rate (bytes/second)
    pub rx_bytes: u64,
    /// Network transmit rate (bytes/second)
    pub tx_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpuCoreMetric {
    pub id: i32,
    pub cpu_user: f32,
    pub cpu_system: f32,
    pub cpu_idle: f32,
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

#[derive(Clone, Copy)]
struct CpuCounters {
    idle: u64,
    user: u64,
    kernel: u64,
}

pub struct MetricsActor {
    receiver: mpsc::Receiver<MetricsCommand>,
    broadcast_tx: broadcast::Sender<MetricPoint>,
    pub history: Arc<RwLock<VecDeque<MetricPoint>>>,
    max_history: usize,
    
    // State for rate calculations
    prev_cpu: HashMap<i32, CpuCounters>,
    prev_net_rx: Option<u64>,
    prev_net_tx: Option<u64>,
    last_update: Instant,
    page_size: u64,
    arc_warned: bool,
    kstat: adapters::kstat::KstatReader,
}

impl MetricsActor {
    pub fn new(
        receiver: mpsc::Receiver<MetricsCommand>,
        broadcast_tx: broadcast::Sender<MetricPoint>,
    ) -> Result<Self, AppError> {
        // Determine page size once
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
        let kstat = adapters::kstat::KstatReader::new()
            .map_err(|e| AppError::ServiceUnavailable(format!("Failed to open kstat: {e:?}")))?;

        Ok(Self {
            receiver,
            broadcast_tx,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(3600))),
            max_history: 3600,
            prev_cpu: HashMap::new(),
            prev_net_rx: None,
            prev_net_tx: None,
            last_update: Instant::now(),
            page_size,
            arc_warned: false,
            kstat,
        })
    }

    pub async fn run(mut self) {
        info!("MetricsActor started (Native Illumos Backend)");

        // Timer for data collection (1Hz)
        let mut ticker = interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    self.collect_and_broadcast().await;
                }
                maybe_cmd = self.receiver.recv() => {
                    match maybe_cmd {
                        Some(_) => {} // Handle subscriptions if needed
                        None => {
                            info!(target: "yanos::metrics", "MetricsActor control channel closed; exiting");
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn collect_and_broadcast(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        let elapsed = if elapsed < 0.001 { 1.0 } else { elapsed };
        self.last_update = now;

        // Refresh kstat chain (lightweight check usually)
        let _ = self.kstat.update();

        // 1. Memory (kstat named FFI)
        let (mem_total, mem_used) = self.read_memory();

        // 2. CPU (kstat counters via FFI)
        let (cpu_user, cpu_system, cpu_idle, per_core) = self.read_cpu(elapsed);

        // 3. Network (kstat link counters)
        let (rx_rate, tx_rate) = self.read_network(elapsed);

        // 4. ZFS ARC (FFI)
        let zfs_arc = self.read_arc_size_bytes();

        let point = MetricPoint {
            ts: chrono::Utc::now().timestamp_millis(),
            cpu_user,
            cpu_system,
            cpu_idle,
            per_core,
            memory_used: mem_used,
            memory_total: mem_total,
            zfs_arc,
            rx_bytes: rx_rate,
            tx_bytes: tx_rate,
        };

        // Update History
        {
            let mut hist = self.history.write().await;
            if hist.len() >= self.max_history {
                hist.pop_front();
            }
            hist.push_back(point.clone());
        }

        let _ = self.broadcast_tx.send(point);
    }



    fn read_memory(&mut self) -> (u64, u64) {
        let (phys_pages, free_pages) = self.kstat.get_memory_pages();
        let total = phys_pages * self.page_size;
        let used = (phys_pages.saturating_sub(free_pages)) * self.page_size;
        (total, used)
    }

    fn read_cpu(&mut self, _elapsed: f64) -> (f32, f32, f32, Vec<CpuCoreMetric>) {
        let mut total_user = 0u64;
        let mut total_sys = 0u64;
        let mut total_idle = 0u64;
        let mut per_core = Vec::new();
        let mut next_prev = HashMap::new();

        let ticks = self.kstat.get_cpu_ticks_by_instance();
        for (instance, raw) in ticks {
            let current = CpuCounters {
                idle: raw.idle,
                user: raw.user,
                kernel: raw.kernel,
            };
            next_prev.insert(instance, current);

            let mut u_pct = 0.0;
            let mut s_pct = 0.0;
            let mut i_pct = 0.0;

            if let Some(prev) = self.prev_cpu.get(&instance) {
                let d_user = current.user.saturating_sub(prev.user);
                let d_sys = current.kernel.saturating_sub(prev.kernel);
                let d_idle = current.idle.saturating_sub(prev.idle);
                let total = d_user + d_sys + d_idle;
                if total > 0 {
                    u_pct = (d_user as f32 / total as f32) * 100.0;
                    s_pct = (d_sys as f32 / total as f32) * 100.0;
                    i_pct = (d_idle as f32 / total as f32) * 100.0;
                }
                total_user += d_user;
                total_sys += d_sys;
                total_idle += d_idle;
            }

            per_core.push(CpuCoreMetric {
                id: instance,
                cpu_user: u_pct,
                cpu_system: s_pct,
                cpu_idle: i_pct,
            });
        }

        self.prev_cpu = next_prev;

        let mut agg_user = 0.0;
        let mut agg_sys = 0.0;
        let mut agg_idle = 0.0;
        let total = total_user + total_sys + total_idle;
        if total > 0 {
            agg_user = (total_user as f32 / total as f32) * 100.0;
            agg_sys = (total_sys as f32 / total as f32) * 100.0;
            agg_idle = (total_idle as f32 / total as f32) * 100.0;
        }

        (agg_user, agg_sys, agg_idle, per_core)
    }

    fn read_network(&mut self, elapsed: f64) -> (u64, u64) {
        // Sum rbytes64/obytes64 across all net-class kstats
        let tot_rx = self.kstat.sum_field_any("rbytes64", Some("net"));
        let tot_tx = self.kstat.sum_field_any("obytes64", Some("net"));

        let rx_rate = if let Some(prev_rx) = self.prev_net_rx {
            let d_rx = tot_rx.saturating_sub(prev_rx);
            (d_rx as f64 / elapsed) as u64
        } else {
            0
        };

        let tx_rate = if let Some(prev_tx) = self.prev_net_tx {
            let d_tx = tot_tx.saturating_sub(prev_tx);
            (d_tx as f64 / elapsed) as u64
        } else {
            0
        };

        self.prev_net_rx = Some(tot_rx);
        self.prev_net_tx = Some(tot_tx);

        (rx_rate, tx_rate)
    }

    fn read_arc_size_bytes(&mut self) -> u64 {
        match self.kstat.get_named("zfs", 0, "arcstats", "size") {
            Ok(size) => size,
            Err(e) => {
                if !self.arc_warned {
                    warn!(target: "yanos::metrics", error = ?e, "Failed to read ZFS ARC size via FFI; reporting 0");
                    self.arc_warned = true;
                }
                0
            }
        }
    }
}

pub fn start_metrics_actor() -> Result<Arc<MetricsState>, AppError> {
    let (command_tx, rx) = mpsc::channel(32);
    let (broadcast_tx, _) = broadcast::channel(100);
    let actor = MetricsActor::new(rx, broadcast_tx.clone())?;

    let history = actor.history.clone();

    tokio::spawn(actor.run());

    Ok(Arc::new(MetricsState {
        broadcast_tx,
        history,
        command_tx,
    }))
}
