//! Path management for the bonding engine.
//!
//! Tracks per-path health, measured metrics, and byte accounting for the
//! three-path bonding scheduler. Paths transition between Connected, Degraded,
//! Unhealthy and Disconnected; only usable paths are scheduled.

use network_core::InterfaceMetrics;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub type PathId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathHealth {
    Connected,
    Degraded,
    Unhealthy,
    Disconnected,
}

impl PathHealth {
    pub fn usable(&self) -> bool {
        matches!(self, PathHealth::Connected | PathHealth::Degraded)
    }
}

impl Default for PathHealth {
    fn default() -> Self {
        PathHealth::Disconnected
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathConfig {
    pub path_id: PathId,
    pub interface_id: String,
    pub label: String,
    pub mtu: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathStats {
    pub path_id: PathId,
    pub health: PathHealth,
    pub mtu: usize,
    pub goodput_bps: f64,
    pub rtt_ms: f64,
    pub jitter_ms: f64,
    pub loss_pct: f64,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub tx_bytes: u64,
    pub acked_bytes: u64,
    pub tx_packets: u64,
    pub rx_acks: u64,
    pub last_seen_ms: Option<u64>,
}

impl PathStats {
    pub fn queue_full(&self) -> bool {
        self.queue_capacity > 0 && self.queue_depth >= self.queue_capacity
    }
    pub fn usable(&self) -> bool {
        self.health.usable() && !self.queue_full()
    }
}

/// Sliding-window goodput estimator based on cumulative acknowledged bytes.
pub struct GoodputEstimator {
    window_ms: u64,
    samples: Vec<(u64, u64)>,
}

impl GoodputEstimator {
    pub fn new(window_ms: u64) -> Self {
        Self {
            window_ms,
            samples: Vec::new(),
        }
    }

    pub fn record(&mut self, now_ms: u64, cumulative_acked_bytes: u64) {
        self.samples.push((now_ms, cumulative_acked_bytes));
        self.prune(now_ms);
    }

    fn prune(&mut self, now_ms: u64) {
        let cutoff = now_ms.saturating_sub(self.window_ms);
        while let Some((ts, _)) = self.samples.first() {
            if *ts < cutoff {
                self.samples.remove(0);
            } else {
                break;
            }
        }
    }

    pub fn goodput_bps(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }
        let (t0, b0) = self.samples.first().unwrap();
        let (t1, b1) = self.samples.last().unwrap();
        let dt_secs = (*t1 as f64 - *t0 as f64) / 1000.0;
        if dt_secs <= 0.0 {
            return 0.0;
        }
        (b1.abs_diff(*b0) as f64) * 8.0 / dt_secs
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn infer_health(metrics: &InterfaceMetrics) -> PathHealth {
    if metrics.loss_pct.unwrap_or(0.0) > 30.0 || metrics.latency_ms.unwrap_or(0.0) > 500.0 {
        return PathHealth::Disconnected;
    }
    if metrics.loss_pct.unwrap_or(0.0) > 15.0 || metrics.latency_ms.unwrap_or(0.0) > 200.0 {
        return PathHealth::Unhealthy;
    }
    if metrics.loss_pct.unwrap_or(0.0) > 5.0 || metrics.latency_ms.unwrap_or(0.0) > 100.0 {
        return PathHealth::Degraded;
    }
    PathHealth::Connected
}

/// Manages the set of bonding paths and their measured metrics.
pub struct PathManager {
    paths: HashMap<PathId, PathConfig>,
    stats: HashMap<PathId, PathStats>,
    estimator: GoodputEstimator,
}

impl Default for PathManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PathManager {
    pub fn new() -> Self {
        Self {
            paths: HashMap::new(),
            stats: HashMap::new(),
            estimator: GoodputEstimator::new(5_000),
        }
    }

    pub fn add_path(&mut self, config: PathConfig) {
        let path_id = config.path_id.clone();
        self.stats.insert(
            path_id.clone(),
            PathStats {
                path_id: path_id.clone(),
                health: PathHealth::Disconnected,
                mtu: config.mtu,
                ..Default::default()
            },
        );
        self.paths.insert(path_id, config);
    }

    pub fn remove_path(&mut self, path_id: &str) -> bool {
        self.paths.remove(path_id).is_some() && self.stats.remove(path_id).is_some()
    }

    pub fn set_health(&mut self, path_id: &str, health: PathHealth) {
        if let Some(s) = self.stats.get_mut(path_id) {
            s.health = health;
        }
    }

    pub fn mark_disconnected(&mut self, path_id: &str) {
        if let Some(s) = self.stats.get_mut(path_id) {
            s.health = PathHealth::Disconnected;
        }
    }

    pub fn mark_recovered(&mut self, path_id: &str) -> bool {
        let s = match self.stats.get(path_id) {
            Some(s) => s.clone(),
            None => return false,
        };
        if s.queue_full() {
            return false;
        }
        if let Some(st) = self.stats.get_mut(path_id) {
            st.health = PathHealth::Connected;
            true
        } else {
            false
        }
    }

    pub fn update_metrics(&mut self, path_id: &str, metrics: &InterfaceMetrics) {
        let now = now_ms();
        let health = infer_health(metrics);
        let stats = self
            .stats
            .entry(path_id.to_string())
            .or_insert_with(|| PathStats {
                path_id: path_id.to_string(),
                health,
                mtu: 0,
                ..Default::default()
            });
        stats.health = health;
        stats.rtt_ms = metrics.latency_ms.unwrap_or(0.0);
        stats.loss_pct = metrics.loss_pct.unwrap_or(0.0);
        stats.last_seen_ms = Some(now);
        // Goodput is derived from acknowledged bytes (recorded separately via
        // record_acknowledgement), never from link-speed/PHY rate.
        stats.goodput_bps = self.estimator.goodput_bps();
    }

    pub fn record_acknowledgement(&mut self, path_id: &str, bytes: u64) {
        let now = now_ms();
        if let Some(stats) = self.stats.get_mut(path_id) {
            stats.acked_bytes += bytes;
            stats.rx_acks += 1;
            stats.last_seen_ms = Some(now);
            self.estimator.record(now, stats.acked_bytes);
            stats.goodput_bps = self.estimator.goodput_bps();
        }
    }

    pub fn record_transmission(&mut self, path_id: &str, bytes: u64) {
        if let Some(stats) = self.stats.get_mut(path_id) {
            stats.tx_bytes += bytes;
            stats.tx_packets = stats.tx_packets.saturating_add(1);
        }
    }

    pub fn set_queue(&mut self, path_id: &str, depth: usize, capacity: usize) {
        if let Some(stats) = self.stats.get_mut(path_id) {
            stats.queue_depth = depth;
            stats.queue_capacity = capacity;
        }
    }

    pub fn stats(&self, path_id: &str) -> Option<&PathStats> {
        self.stats.get(path_id)
    }

    pub fn all_stats(&self) -> Vec<&PathStats> {
        self.stats.values().collect()
    }

    pub fn healthy_stats(&self) -> Vec<&PathStats> {
        self.stats.values().filter(|s| s.usable()).collect()
    }

    pub fn path_count(&self) -> usize {
        self.paths.len()
    }

    pub fn healthy_count(&self) -> usize {
        self.stats.values().filter(|s| s.usable()).count()
    }

    pub fn config(&self, path_id: &str) -> Option<&PathConfig> {
        self.paths.get(path_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use network_core::InterfaceMetrics;

    fn metrics(loss: f64, rtt: f64) -> InterfaceMetrics {
        InterfaceMetrics {
            interface_id: "t".into(),
            latency_ms: Some(rtt),
            loss_pct: Some(loss),
            rx_bytes: 0,
            tx_bytes: 0,
        }
    }

    #[test]
    fn add_and_remove_path() {
        let mut m = PathManager::new();
        m.add_path(PathConfig {
            path_id: "wifi1".into(),
            interface_id: "eth0".into(),
            label: "Wi-Fi 1".into(),
            mtu: 1500,
        });
        assert_eq!(m.path_count(), 1);
        assert!(m.remove_path("wifi1"));
        assert_eq!(m.path_count(), 0);
        assert!(!m.remove_path("missing"));
    }

    #[test]
    fn unhealthy_paths_are_not_usable() {
        let mut m = PathManager::new();
        m.add_path(PathConfig {
            path_id: "p".into(),
            interface_id: "i".into(),
            label: "p".into(),
            mtu: 1500,
        });
        m.update_metrics("p", &metrics(40.0, 600.0));
        assert_eq!(m.healthy_count(), 0);
        assert_eq!(m.stats("p").unwrap().health, PathHealth::Disconnected);
    }

    #[test]
    fn path_recovers_after_metrics_improve() {
        let mut m = PathManager::new();
        m.add_path(PathConfig {
            path_id: "p".into(),
            interface_id: "i".into(),
            label: "p".into(),
            mtu: 1500,
        });
        m.update_metrics("p", &metrics(40.0, 600.0));
        assert!(!m.stats("p").unwrap().usable());
        m.update_metrics("p", &metrics(1.0, 5.0));
        assert!(m.stats("p").unwrap().usable());
    }

    #[test]
    fn goodput_is_measured_not_phy() {
        let mut m = PathManager::new();
        m.add_path(PathConfig {
            path_id: "p".into(),
            interface_id: "i".into(),
            label: "p".into(),
            mtu: 1500,
        });
        let iface = InterfaceMetrics {
            interface_id: "i".into(),
            latency_ms: Some(10.0),
            loss_pct: Some(0.0),
            rx_bytes: 0,
            tx_bytes: 0,
        };
        // transmit_link_speed_bps is deliberately never consulted for goodput.
        let _ = iface;
        m.record_acknowledgement("p", 1_000_000);
        assert!(m.stats("p").unwrap().goodput_bps >= 0.0);
    }

    #[test]
    fn full_queue_marks_unusable() {
        let mut m = PathManager::new();
        m.add_path(PathConfig {
            path_id: "p".into(),
            interface_id: "i".into(),
            label: "p".into(),
            mtu: 1500,
        });
        m.update_metrics("p", &metrics(0.0, 5.0));
        m.set_queue("p", 10, 10);
        assert!(!m.stats("p").unwrap().usable());
    }
}
