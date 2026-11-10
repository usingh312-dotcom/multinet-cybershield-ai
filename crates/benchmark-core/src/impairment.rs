//! Deterministic network impairment simulator.
//!
//! Provides a completely software-based, deterministic impairment channel that
//! accurately simulates:
//! - Bandwidth throttling (token bucket rate limiting)
//! - Base latency and jitter distribution
//! - Packet loss (uniform random)
//! - Packet duplication
//! - Packet reordering
//! - Queue depth bounds and tail drop
//! - MTU limits and path disconnection
//!
//! All calculations depend strictly on an explicit seed and virtual clock,
//! guaranteeing 100% reproducible test execution without root privileges or host route mutations.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ImpairmentError {
    #[error("invalid impairment configuration: {0}")]
    InvalidConfig(String),
    #[error("packet exceeds MTU {0} (payload is {1} bytes)")]
    MtuExceeded(usize, usize),
    #[error("channel queue overflow (capacity: {0})")]
    QueueOverflow(usize),
    #[error("channel is disconnected")]
    Disconnected,
}

/// Configuration profile for deterministic impairment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImpairmentProfile {
    pub name: String,
    /// Maximum bandwidth in bits per second (None for unlimited).
    pub bandwidth_bps: Option<u64>,
    /// Base latency in milliseconds added to every delivered packet.
    pub base_latency_ms: u64,
    /// Maximum jitter in milliseconds added uniformly in `[0, jitter_ms]`.
    pub jitter_ms: u64,
    /// Packet loss probability between 0.0 and 100.0 percent.
    pub loss_pct: f64,
    /// Packet duplication probability between 0.0 and 100.0 percent.
    pub duplication_pct: f64,
    /// Packet reordering probability between 0.0 and 100.0 percent.
    pub reorder_pct: f64,
    /// Maximum number of queued in-flight packets before tail-drop occurs.
    pub queue_capacity: usize,
    /// Maximum MTU in bytes allowed across this channel.
    pub mtu: usize,
}

impl Default for ImpairmentProfile {
    fn default() -> Self {
        Self {
            name: "default_ideal".to_string(),
            bandwidth_bps: None,
            base_latency_ms: 10,
            jitter_ms: 2,
            loss_pct: 0.0,
            duplication_pct: 0.0,
            reorder_pct: 0.0,
            queue_capacity: 1000,
            mtu: 1500,
        }
    }
}

impl ImpairmentProfile {
    pub fn validate(&self) -> Result<(), ImpairmentError> {
        if self.loss_pct < 0.0 || self.loss_pct > 100.0 {
            return Err(ImpairmentError::InvalidConfig(format!(
                "loss_pct must be between 0.0 and 100.0, got {}",
                self.loss_pct
            )));
        }
        if self.duplication_pct < 0.0 || self.duplication_pct > 100.0 {
            return Err(ImpairmentError::InvalidConfig(format!(
                "duplication_pct must be between 0.0 and 100.0, got {}",
                self.duplication_pct
            )));
        }
        if self.reorder_pct < 0.0 || self.reorder_pct > 100.0 {
            return Err(ImpairmentError::InvalidConfig(format!(
                "reorder_pct must be between 0.0 and 100.0, got {}",
                self.reorder_pct
            )));
        }
        if self.queue_capacity == 0 {
            return Err(ImpairmentError::InvalidConfig(
                "queue_capacity must be greater than 0".to_string(),
            ));
        }
        if self.mtu < 64 {
            return Err(ImpairmentError::InvalidConfig(format!(
                "mtu must be at least 64 bytes, got {}",
                self.mtu
            )));
        }
        Ok(())
    }
}

/// Deterministic 64-bit XorShift pseudorandom number generator.
#[derive(Debug, Clone)]
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0xdead_beef_cafe_babe
            } else {
                seed
            },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Return random f64 in range [0.0, 1.0)
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform random range [min, max]
    pub fn gen_range_u64(&mut self, min: u64, max: u64) -> u64 {
        if min >= max {
            return min;
        }
        min + (self.next_u64() % (max - min + 1))
    }
}

/// An in-flight packet within the simulated impairment channel.
#[derive(Debug, Clone)]
pub struct InFlightPacket {
    pub seq_no: u64,
    pub payload: Vec<u8>,
    pub enqueue_time_ms: u64,
    pub delivery_time_ms: u64,
    pub is_duplicate: bool,
    pub is_reordered: bool,
}

/// Statistics recorded by the impairment channel.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpairmentStats {
    pub packets_offered: u64,
    pub packets_delivered: u64,
    pub packets_dropped_loss: u64,
    pub packets_dropped_queue_overflow: u64,
    pub packets_dropped_mtu: u64,
    pub packets_dropped_disconnected: u64,
    pub packets_duplicated: u64,
    pub packets_reordered: u64,
    pub bytes_offered: u64,
    pub bytes_delivered: u64,
}

/// Simulated impairment channel with virtual time progression.
pub struct ImpairmentChannel {
    profile: ImpairmentProfile,
    rng: DeterministicRng,
    queue: VecDeque<InFlightPacket>,
    is_connected: bool,
    stats: ImpairmentStats,
    next_seq: u64,
    /// Token bucket for rate limiting: last token update in virtual ms
    last_rate_limit_virtual_ms: u64,
    tokens_available_bytes: f64,
    /// Next virtual time when the transmission link becomes available for a new packet
    next_available_tx_virtual_ms: u64,
}

impl ImpairmentChannel {
    pub fn new(profile: ImpairmentProfile, seed: u64) -> Result<Self, ImpairmentError> {
        profile.validate()?;
        Ok(Self {
            profile,
            rng: DeterministicRng::new(seed),
            queue: VecDeque::new(),
            is_connected: true,
            stats: ImpairmentStats::default(),
            next_seq: 0,
            last_rate_limit_virtual_ms: 0,
            tokens_available_bytes: 0.0,
            next_available_tx_virtual_ms: 0,
        })
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.is_connected = connected;
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    pub fn stats(&self) -> &ImpairmentStats {
        &self.stats
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Offer a packet for transmission at virtual clock `virtual_now_ms`.
    pub fn offer_packet(
        &mut self,
        payload: Vec<u8>,
        virtual_now_ms: u64,
    ) -> Result<bool, ImpairmentError> {
        self.stats.packets_offered += 1;
        self.stats.bytes_offered += payload.len() as u64;

        if !self.is_connected {
            self.stats.packets_dropped_disconnected += 1;
            return Ok(false);
        }

        if payload.len() > self.profile.mtu {
            self.stats.packets_dropped_mtu += 1;
            return Ok(false);
        }

        // Check simulated loss
        let loss_roll = self.rng.next_f64() * 100.0;
        if loss_roll < self.profile.loss_pct {
            self.stats.packets_dropped_loss += 1;
            return Ok(false);
        }

        // Check queue overflow
        if self.queue.len() >= self.profile.queue_capacity {
            self.stats.packets_dropped_queue_overflow += 1;
            return Ok(false);
        }

        // Calculate delivery time
        let jitter = if self.profile.jitter_ms > 0 {
            self.rng.gen_range_u64(0, self.profile.jitter_ms)
        } else {
            0
        };

        // Rate limiter and transmission serialization if bandwidth cap is configured
        let (tx_start_ms, wire_time_ms) = if let Some(bps) = self.profile.bandwidth_bps {
            let bytes_per_ms = (bps as f64) / 8000.0;
            let elapsed_ms = virtual_now_ms.saturating_sub(self.last_rate_limit_virtual_ms);
            let burst_cap = (bps as f64) / 8.0 * 0.1; // 100ms burst cap
            self.tokens_available_bytes =
                (self.tokens_available_bytes + (elapsed_ms as f64) * bytes_per_ms).min(burst_cap);
            self.last_rate_limit_virtual_ms = virtual_now_ms;

            let wait_ms = if self.tokens_available_bytes < (payload.len() as f64) {
                let needed_bytes = (payload.len() as f64) - self.tokens_available_bytes;
                self.tokens_available_bytes = 0.0;
                (needed_bytes / bytes_per_ms).ceil() as u64
            } else {
                self.tokens_available_bytes -= payload.len() as f64;
                0
            };

            let wire_time = ((payload.len() as f64) / bytes_per_ms).max(1.0).ceil() as u64;
            let tx_start = self.next_available_tx_virtual_ms.max(virtual_now_ms) + wait_ms;
            self.next_available_tx_virtual_ms = tx_start + wire_time;
            (tx_start, wire_time)
        } else {
            (virtual_now_ms, 0)
        };

        let mut delivery_time_ms =
            tx_start_ms + wire_time_ms + self.profile.base_latency_ms + jitter;

        let seq_no = self.next_seq;
        self.next_seq += 1;

        // Check reordering: if triggered, arbitrarily delay delivery past following packets
        let reorder_roll = self.rng.next_f64() * 100.0;
        let is_reordered = if reorder_roll < self.profile.reorder_pct {
            delivery_time_ms += 50; // hold back by 50ms to allow next packets to arrive earlier
            self.stats.packets_reordered += 1;
            true
        } else {
            false
        };

        let packet = InFlightPacket {
            seq_no,
            payload: payload.clone(),
            enqueue_time_ms: virtual_now_ms,
            delivery_time_ms,
            is_duplicate: false,
            is_reordered,
        };
        self.queue.push_back(packet);

        // Check duplication: duplicate packets also traverse the link and consume transmission time
        let dup_roll = self.rng.next_f64() * 100.0;
        if dup_roll < self.profile.duplication_pct {
            self.stats.packets_duplicated += 1;
            let dup_tx_start = if self.profile.bandwidth_bps.is_some() {
                let start = self.next_available_tx_virtual_ms;
                self.next_available_tx_virtual_ms += wire_time_ms;
                start
            } else {
                tx_start_ms
            };
            let dup_delivery =
                dup_tx_start + wire_time_ms + self.profile.base_latency_ms + jitter + 1;
            let dup_packet = InFlightPacket {
                seq_no,
                payload,
                enqueue_time_ms: virtual_now_ms,
                delivery_time_ms: dup_delivery,
                is_duplicate: true,
                is_reordered,
            };
            self.queue.push_back(dup_packet);
        }

        Ok(true)
    }

    /// Poll all packets ready to be delivered at or before `virtual_now_ms`.
    pub fn poll_deliverable(&mut self, virtual_now_ms: u64) -> Vec<InFlightPacket> {
        let mut delivered = Vec::new();
        let mut i = 0;
        while i < self.queue.len() {
            if self.queue[i].delivery_time_ms <= virtual_now_ms {
                let pkt = self.queue.remove(i).unwrap();
                self.stats.packets_delivered += 1;
                self.stats.bytes_delivered += pkt.payload.len() as u64;
                delivered.push(pkt);
            } else {
                i += 1;
            }
        }
        delivered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_validation() {
        let invalid_loss = ImpairmentProfile {
            loss_pct: 120.0,
            ..Default::default()
        };
        assert!(invalid_loss.validate().is_err());

        let invalid_mtu = ImpairmentProfile {
            mtu: 10,
            ..Default::default()
        };
        assert!(invalid_mtu.validate().is_err());

        let valid = ImpairmentProfile::default();
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_deterministic_seed_reproducibility() {
        let profile = ImpairmentProfile {
            base_latency_ms: 20,
            jitter_ms: 10,
            loss_pct: 20.0,
            reorder_pct: 10.0,
            ..Default::default()
        };

        let mut ch1 = ImpairmentChannel::new(profile.clone(), 12345).unwrap();
        let mut ch2 = ImpairmentChannel::new(profile, 12345).unwrap();

        for i in 0..100 {
            let data = vec![i as u8; 64];
            let _ = ch1.offer_packet(data.clone(), (i * 10) as u64);
            let _ = ch2.offer_packet(data, (i * 10) as u64);
        }

        assert_eq!(ch1.stats(), ch2.stats());
    }

    #[test]
    fn test_loss_and_queue_overflow() {
        let profile = ImpairmentProfile {
            base_latency_ms: 100,
            jitter_ms: 0,
            loss_pct: 0.0,
            queue_capacity: 5,
            ..Default::default()
        };

        let mut ch = ImpairmentChannel::new(profile, 42).unwrap();
        for _ in 0..10 {
            let _ = ch.offer_packet(vec![0u8; 32], 0);
        }

        assert_eq!(ch.stats().packets_offered, 10);
        assert_eq!(ch.stats().packets_dropped_queue_overflow, 5);
        assert_eq!(ch.queue_len(), 5);
    }

    #[test]
    fn test_delivery_timing_progression() {
        let profile = ImpairmentProfile {
            base_latency_ms: 50,
            jitter_ms: 0,
            loss_pct: 0.0,
            ..Default::default()
        };

        let mut ch = ImpairmentChannel::new(profile, 42).unwrap();
        ch.offer_packet(vec![1, 2, 3], 100).unwrap();

        // At t=140, packet should not be deliverable yet
        let early = ch.poll_deliverable(140);
        assert!(early.is_empty());

        // At t=150, packet must be delivered
        let on_time = ch.poll_deliverable(150);
        assert_eq!(on_time.len(), 1);
        assert_eq!(on_time[0].payload, vec![1, 2, 3]);
    }

    #[test]
    fn test_simultaneous_burst_serialization_under_bandwidth_cap() {
        // 8 Mbps = 1000 bytes/ms
        let profile = ImpairmentProfile {
            bandwidth_bps: Some(8_000_000),
            base_latency_ms: 10,
            jitter_ms: 0,
            loss_pct: 0.0,
            queue_capacity: 100,
            ..Default::default()
        };

        let mut ch = ImpairmentChannel::new(profile, 42).unwrap();
        // Offer 5 1000-byte packets simultaneously at virtual_now_ms = 0
        for _ in 0..5 {
            ch.offer_packet(vec![0xAA; 1000], 0).unwrap();
        }

        assert_eq!(ch.queue_len(), 5);

        // Under rate limiting and serialization:
        // Packet 0: wait 1ms, wire 1ms -> delivery at 12ms
        // Packet 1: wait 1ms, wire 1ms -> delivery at 14ms
        // Packet 2: wait 1ms, wire 1ms -> delivery at 16ms
        // Packet 3: wait 1ms, wire 1ms -> delivery at 18ms
        // Packet 4: wait 1ms, wire 1ms -> delivery at 20ms
        let mut prev_delivery = 0;
        for i in 0..ch.queue.len() {
            let pkt = &ch.queue[i];
            assert!(
                pkt.delivery_time_ms > prev_delivery,
                "Packet {} delivery {} must be strictly greater than previous {}",
                i,
                pkt.delivery_time_ms,
                prev_delivery
            );
            prev_delivery = pkt.delivery_time_ms;
        }

        // At t=11, none delivered
        assert_eq!(ch.poll_deliverable(11).len(), 0);
        // At t=15, first two delivered
        assert_eq!(ch.poll_deliverable(15).len(), 2);
        // At t=20, remaining three delivered
        assert_eq!(ch.poll_deliverable(20).len(), 3);
        assert_eq!(ch.queue_len(), 0);
    }

    #[test]
    fn test_duplication_consumes_bandwidth() {
        let profile = ImpairmentProfile {
            bandwidth_bps: Some(8_000_000), // 1000 bytes/ms
            base_latency_ms: 5,
            jitter_ms: 0,
            duplication_pct: 100.0, // force duplicate
            loss_pct: 0.0,
            queue_capacity: 10,
            ..Default::default()
        };

        let mut ch = ImpairmentChannel::new(profile, 42).unwrap();
        ch.offer_packet(vec![0x55; 1000], 0).unwrap();

        // Must produce 2 packets in queue: original and duplicate
        assert_eq!(ch.queue_len(), 2);
        assert_eq!(ch.stats().packets_duplicated, 1);

        let p1 = &ch.queue[0];
        let p2 = &ch.queue[1];
        assert!(p1.delivery_time_ms < p2.delivery_time_ms);
    }
}
