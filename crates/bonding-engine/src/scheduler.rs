//! Weighted deficit packet scheduler for multi-path bonding.
//!
//! Implements weighted deficit round-robin across paths. Weights are derived
//! exclusively from measured per-path goodput, RTT, jitter, loss and queue
//! depth -- never from Wi-Fi PHY rates. Disconnected, unhealthy, queue-full
//! or MTU-constrained paths are skipped.

use std::collections::HashMap;
use crate::path_manager::{PathHealth, PathStats};

pub type PathId = String;

/// A scheduling decision: which path to send on and how many bytes.
#[derive(Debug, Clone)]
pub struct ScheduledPacket {
    pub path_id: PathId,
    pub seq_no: u64,
    pub size: usize,
    pub weight: f64,
}

/// Per-path accounting exposed to the dashboard.
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    pub path_id: PathId,
    pub weight: f64,
    pub share_pct: f64,
    pub packets_sent: u64,
    pub bytes_sent: u64,
    pub skips_unhealthy: u64,
    pub skips_mtu: u64,
}

#[derive(Debug, Clone, Default)]
struct PathState {
    weight: f64,
    deficit: f64,
    mtu: usize,
    health: PathHealth,
    stats: PathStats,
}

/// Weighted deficit round-robin scheduler.
pub struct WeightedPacketScheduler {
    paths: HashMap<PathId, PathState>,
    next_seq: u64,
    /// Rotating cursor across cumulative weight space for deterministic,
    /// proportional path selection (prevents the highest-weight path from
    /// monopolizing every scheduling decision).
    wrr_cursor: u64,
}

impl Default for WeightedPacketScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl WeightedPacketScheduler {
    pub fn new() -> Self {
        Self {
            paths: HashMap::new(),
            next_seq: 0,
            wrr_cursor: 0,
        }
    }

    pub fn add_path(&mut self, path_id: PathId, mtu: usize) {
        self.paths.insert(
            path_id,
            PathState {
                mtu,
                ..Default::default()
            },
        );
    }

    pub fn remove_path(&mut self, path_id: &str) -> bool {
        self.paths.remove(path_id).is_some()
    }

    /// Update the scheduler's view of a path's measured metrics.
    pub fn update_metrics(&mut self, stats: &PathStats) {
        self.paths
            .entry(stats.path_id.clone())
            .and_modify(|st| {
                st.health = stats.health;
                st.stats = stats.clone();
                st.mtu = stats.mtu;
            });
    }

    /// Select the next path for `packet_size` bytes, or `None` when no path
    /// can carry the packet (all unavailable, unhealthy, queue-full, or
    /// smaller-MTU than the packet).
    pub fn schedule(&mut self, packet_size: usize) -> Option<ScheduledPacket> {
        let mut total_weight: f64 = 0.0;
        let mut eligible: Vec<(PathId, f64)> = Vec::new();

        for (id, state) in &mut self.paths {
            if !state.health.usable() || state.stats.queue_full() || state.mtu < packet_size {
                continue;
            }
            let weight = compute_weight(&state.stats);
            if !weight.is_finite() || weight <= 0.0 {
                continue;
            }
            state.weight = weight;
            state.deficit += weight;
            total_weight += weight;
            eligible.push((id.clone(), weight));
        }

        if eligible.is_empty() {
            return None;
        }

        // Deterministic weighted round-robin: walk a cursor across the
        // cumulative weight space so each eligible path is chosen in
        // proportion to its weight, and no path is permanently starved.
        self.wrr_cursor = self.wrr_cursor.wrapping_add(1_000_003);
        let target = (self.wrr_cursor as f64) % total_weight;
        let mut acc = 0.0f64;
        let mut chosen: Option<PathId> = None;
        for (id, weight) in &eligible {
            acc += weight;
            if target < acc {
                chosen = Some(id.clone());
                break;
            }
        }
        let id = chosen.unwrap_or_else(|| eligible.last().unwrap().0.clone());

        let state = self.paths.get_mut(&id).unwrap();
        state.deficit -= packet_size as f64;
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        state.stats.tx_packets = state.stats.tx_packets.saturating_add(1);
        state.stats.tx_bytes = state.stats.tx_bytes.saturating_add(packet_size as u64);
        Some(ScheduledPacket {
            path_id: id,
            seq_no: seq,
            size: packet_size,
            weight: state.weight,
        })
    }

    pub fn record_transmission(&mut self, path_id: &str, bytes: usize) {
        if let Some(s) = self.paths.get_mut(path_id) {
            s.stats.tx_bytes = s.stats.tx_bytes.saturating_add(bytes as u64);
            s.stats.tx_packets = s.stats.tx_packets.saturating_add(1);
        }
    }

    pub fn record_acknowledgement(&mut self, path_id: &str, bytes: usize) {
        if let Some(s) = self.paths.get_mut(path_id) {
            s.stats.acked_bytes = s.stats.acked_bytes.saturating_add(bytes as u64);
            s.stats.rx_acks = s.stats.rx_acks.saturating_add(1);
        }
    }

    pub fn stats(&self) -> Vec<SchedulerStats> {
        let total_weight: f64 = self.paths.values().map(|s| s.weight).sum();
        self.paths
            .iter()
            .map(|(id, state)| {
                let share = if total_weight > 0.0 {
                    state.weight / total_weight * 100.0
                } else {
                    0.0
                };
                SchedulerStats {
                    path_id: id.clone(),
                    weight: state.weight,
                    share_pct: share,
                    packets_sent: state.stats.tx_packets,
                    bytes_sent: state.stats.tx_bytes,
                    skips_unhealthy: 0,
                    skips_mtu: 0,
                }
            })
            .collect()
    }

    pub fn eligible_count(&self) -> usize {
        self.paths
            .values()
            .filter(|s| compute_weight(&s.stats) > 0.0)
            .count()
    }
}

/// Compute a path's dynamic scheduling weight from measured metrics only.
///
/// Never derived from Wi-Fi PHY/link speed (requirement #10).
fn compute_weight(stats: &PathStats) -> f64 {
    if !stats.health.usable() || stats.queue_full() {
        return 0.0;
    }
    let goodput = stats.goodput_bps.max(0.0);
    let loss_factor = 1.0 / (1.0 + stats.loss_pct);
    let latency_factor = 1.0 / (1.0 + stats.rtt_ms / 100.0);
    let jitter_factor = 1.0 / (1.0 + stats.jitter_ms);
    let queue_factor = if stats.queue_capacity > 0 {
        1.0 / (1.0 + stats.queue_depth as f64 / stats.queue_capacity as f64)
    } else {
        1.0
    };
    goodput * loss_factor * latency_factor * jitter_factor * queue_factor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_manager::PathHealth;

    fn stats(path_id: &str, goodput: f64, loss: f64, rtt: f64, mtu: usize) -> PathStats {
        PathStats {
            path_id: path_id.to_string(),
            health: PathHealth::Connected,
            mtu,
            goodput_bps: goodput,
            loss_pct: loss,
            rtt_ms: rtt,
            jitter_ms: 0.0,
            queue_depth: 0,
            queue_capacity: 1000,
            ..Default::default()
        }
    }

    #[test]
    fn synthetic_capacity_distribution_is_proportional() {
        // 20 / 100 / 80 Mbps -> weights should track capacity roughly.
        let mut sched = WeightedPacketScheduler::new();
        for (p, g) in [
            ("wifi1", 20_000_000.0),
            ("usb", 100_000_000.0),
            ("wifi2", 80_000_000.0),
        ] {
            sched.add_path(p.into(), 1500);
            sched.update_metrics(&stats(p, g, 0.0, 5.0, 1500));
        }
        let mut sends: HashMap<PathId, u64> = HashMap::new();
        for _ in 0..3000 {
            if let Some(sp) = sched.schedule(1200) {
                *sends.entry(sp.path_id).or_insert(0) += 1;
            }
        }
        let w = sends.get("usb").unwrap();
        let a = sends.get("wifi1").unwrap();
        let _b = sends.get("wifi2").unwrap();
        assert!(*w > *a * 3, "USB should carry far more than Wi-Fi 1: {} vs {}", w, a);
        let total: u64 = sends.values().sum();
        assert!(total > 2000, "most packets should be scheduled: {}", total);
    }

    #[test]
    fn equal_capacity_equal_distribution() {
        let mut sched = WeightedPacketScheduler::new();
        for p in ["a", "b", "c"] {
            sched.add_path(p.into(), 1500);
            sched.update_metrics(&stats(p, 50_000_000.0, 0.0, 5.0, 1500));
        }
        let mut sends: HashMap<PathId, u64> = HashMap::new();
        for _ in 0..300 {
            if let Some(sp) = sched.schedule(1000) {
                *sends.entry(sp.path_id).or_insert(0) += 1;
            }
        }
        let total: u64 = sends.values().sum();
        assert_eq!(sends.len(), 3, "all three paths should be used");
        for v in sends.values() {
            assert!(
                (*v as f64 - total as f64 / 3.0).abs() < total as f64 * 0.25,
                "equal capacity should split evenly: {:?}",
                sends
            );
        }
    }

    #[test]
    fn loss_penalty_reduces_weight() {
        let mut sched = WeightedPacketScheduler::new();
        sched.add_path("clean".into(), 1500);
        sched.add_path("lossy".into(), 1500);
        sched.update_metrics(&stats("clean", 50_000_000.0, 0.0, 5.0, 1500));
        sched.update_metrics(&stats("lossy", 50_000_000.0, 40.0, 5.0, 1500));
        let mut sends: HashMap<PathId, u64> = HashMap::new();
        for _ in 0..500 {
            if let Some(sp) = sched.schedule(1000) {
                *sends.entry(sp.path_id).or_insert(0) += 1;
            }
        }
        assert!(
            sends.get("clean").unwrap() > sends.get("lossy").unwrap(),
            "clean path should be preferred over lossy path: {:?}",
            sends
        );
    }

    #[test]
    fn latency_and_queue_penalty() {
        let mut sched = WeightedPacketScheduler::new();
        sched.add_path("fast".into(), 1500);
        sched.add_path("slow".into(), 1500);
        sched.update_metrics(&stats("fast", 50_000_000.0, 0.0, 5.0, 1500));
        let mut slow = stats("slow", 50_000_000.0, 0.0, 300.0, 1500);
        slow.queue_depth = 900;
        slow.queue_capacity = 1000;
        sched.update_metrics(&slow);
        let mut sends: HashMap<PathId, u64> = HashMap::new();
        for _ in 0..500 {
            if let Some(sp) = sched.schedule(1000) {
                *sends.entry(sp.path_id).or_insert(0) += 1;
            }
        }
        assert!(
            sends.get("fast").unwrap() > sends.get("slow").unwrap(),
            "fast/low-queue path should be preferred: {:?}",
            sends
        );
    }

    #[test]
    fn unavailable_path_removed_from_schedule() {
        let mut sched = WeightedPacketScheduler::new();
        sched.add_path("good".into(), 1500);
        sched.add_path("bad".into(), 1500);
        sched.update_metrics(&stats("good", 50_000_000.0, 0.0, 5.0, 1500));
        sched.update_metrics(&stats("bad", 50_000_000.0, 0.0, 5.0, 1500));
        sched.remove_path("bad");
        for _ in 0..100 {
            let sp = sched.schedule(1000).expect("good path should always schedule");
            assert_eq!(sp.path_id, "good");
        }
    }

    #[test]
    fn path_recovers_and_is_scheduled_again() {
        let mut sched = WeightedPacketScheduler::new();
        sched.add_path("p".into(), 1500);
        sched.update_metrics(&stats("p", 50_000_000.0, 0.0, 5.0, 1500));
        let bad = PathStats {
            path_id: "p".into(),
            health: PathHealth::Disconnected,
            mtu: 1500,
            goodput_bps: 0.0,
            loss_pct: 50.0,
            rtt_ms: 999.0,
            jitter_ms: 0.0,
            queue_depth: 0,
            queue_capacity: 1000,
            ..Default::default()
        };
        sched.update_metrics(&bad);
        assert!(sched.schedule(1000).is_none());
        sched.update_metrics(&stats("p", 50_000_000.0, 0.0, 5.0, 1500));
        assert!(sched.schedule(1000).is_some());
    }

    #[test]
    fn mtu_rejection_skips_small_mtu_path() {
        let mut sched = WeightedPacketScheduler::new();
        sched.add_path("small".into(), 500);
        sched.add_path("big".into(), 1500);
        sched.update_metrics(&stats("small", 50_000_000.0, 0.0, 5.0, 500));
        sched.update_metrics(&stats("big", 50_000_000.0, 0.0, 5.0, 1500));
        // 1400-byte packet cannot fit on small-mtu path.
        for _ in 0..50 {
            let sp = sched.schedule(1400).expect("big path should schedule");
            assert_eq!(sp.path_id, "big");
        }
    }

    #[test]
    fn fairness_across_paths() {
        let mut sched = WeightedPacketScheduler::new();
        for p in ["x", "y", "z", "w"] {
            sched.add_path(p.into(), 1500);
            sched.update_metrics(&stats(p, 50_000_000.0, 0.0, 5.0, 1500));
        }
        let mut sends: HashMap<PathId, u64> = HashMap::new();
        for _ in 0..400 {
            if let Some(sp) = sched.schedule(1000) {
                *sends.entry(sp.path_id).or_insert(0) += 1;
            }
        }
        assert_eq!(sends.len(), 4, "all paths should get a share: {:?}", sends);
        let total: u64 = sends.values().sum();
        for v in sends.values() {
            assert!(
                (*v as f64 - total as f64 / 4.0).abs() < total as f64 * 0.35,
                "path share too uneven: {:?}",
                sends
            );
        }
    }

    #[test]
    fn variable_packet_sizes_handled() {
        let mut sched = WeightedPacketScheduler::new();
        sched.add_path("p".into(), 1500);
        sched.update_metrics(&stats("p", 50_000_000.0, 0.0, 5.0, 1500));
        let sizes = [100, 500, 1400, 1, 1499];
        let mut total = 0usize;
        for &sz in sizes.iter().cycle().take(20) {
            let sp = sched.schedule(sz).expect("should schedule variable sizes");
            assert_eq!(sp.size, sz);
            total += sz;
        }
        assert_eq!(sched.stats()[0].bytes_sent, total as u64);
    }

    #[test]
    fn all_paths_unavailable_returns_none() {
        let mut sched = WeightedPacketScheduler::new();
        sched.add_path("p".into(), 1500);
        sched.update_metrics(&stats("p", 0.0, 60.0, 999.0, 1500));
        assert!(sched.schedule(100).is_none());
        assert_eq!(sched.eligible_count(), 0);
    }

    #[test]
    fn invalid_and_stale_metrics_rejected() {
        let mut sched = WeightedPacketScheduler::new();
        sched.add_path("p".into(), 1500);

        // NaN goodput must collapse to a non-schedulable weight.
        let nan = stats("p", f64::NAN, 0.0, 5.0, 1500);
        sched.update_metrics(&nan);
        assert!(sched.schedule(100).is_none(), "NaN goodput must not schedule");

        // Disconnected / stale health must not schedule regardless of goodput.
        let mut disc = stats("p", 50_000_000.0, 0.0, 5.0, 1500);
        disc.health = PathHealth::Disconnected;
        sched.update_metrics(&disc);
        assert!(sched.schedule(100).is_none(), "disconnected path must not schedule");
    }

    #[test]
    fn scheduler_state_cleanup_on_remove() {
        let mut sched = WeightedPacketScheduler::new();
        sched.add_path("p".into(), 1500);
        sched.update_metrics(&stats("p", 50_000_000.0, 0.0, 5.0, 1500));
        assert!(sched.schedule(100).is_some());
        assert!(sched.remove_path("p"));
        assert!(sched.schedule(100).is_none());
        assert_eq!(sched.eligible_count(), 0);
    }

    #[test]
    fn high_packet_volume_stability() {
        let mut sched = WeightedPacketScheduler::new();
        for p in ["a", "b", "c"] {
            sched.add_path(p.into(), 1500);
            sched.update_metrics(&stats(p, 50_000_000.0, 0.0, 5.0, 1500));
        }
        let mut total = 0usize;
        for _ in 0..10_000 {
            if let Some(sp) = sched.schedule(1000) {
                total += sp.size;
            }
        }
        assert!(total > 0, "high volume should schedule without overflow/panic");
        for st in sched.paths.values() {
            assert!(st.weight.is_finite(), "weight must remain finite: {:?}", st.weight);
            assert!(st.deficit.is_finite(), "deficit must remain finite: {:?}", st.deficit);
        }
    }
}
