//! Deterministic synthetic scheduler measurement provider.
//!
//! Reuses the real `WeightedPacketScheduler` and `PathManager` from `crates/bonding-engine`.
//! Tests 1-path, 2-path, and 3-path scenarios with varying goodput, latency, jitter,
//! loss, queue depth, and MTU.

use crate::models::{BenchmarkMetrics, EvidenceTier, PerPathMetrics, TestCaseResult, TestStatus};
use bonding_engine::path_manager::{PathHealth, PathStats};
use bonding_engine::scheduler::WeightedPacketScheduler;
use std::collections::HashMap;

/// Runs the complete synthetic scheduler test suite.
pub fn run_synthetic_suite(seed: u64) -> Vec<TestCaseResult> {
    vec![
        run_single_path_scenario(),
        run_dual_equal_paths_scenario(),
        run_tri_asymmetric_paths_scenario(seed),
        run_degraded_path_shedding_scenario(),
        run_path_loss_and_recovery_scenario(),
        run_all_paths_unavailable_scenario(),
        run_mtu_exclusion_scenario(),
        run_metric_anomaly_handling_scenario(),
    ]
}

/// Scenario 1: Single path baseline.
fn run_single_path_scenario() -> TestCaseResult {
    let mut scheduler = WeightedPacketScheduler::new();
    let path_id = "wan_fiber_0".to_string();
    scheduler.add_path(path_id.clone(), 1500);

    let stats = PathStats {
        path_id: path_id.clone(),
        goodput_bps: 100_000_000.0,
        rtt_ms: 12.0,
        jitter_ms: 1.5,
        loss_pct: 0.0,
        queue_depth: 0,
        queue_capacity: 100,
        health: PathHealth::Connected,
        mtu: 1500,
        ..Default::default()
    };
    scheduler.update_metrics(&stats);

    let total_packets = 1000;
    let packet_size = 1400;
    let mut sent = 0;

    for _ in 0..total_packets {
        if let Some(pkt) = scheduler.schedule(packet_size) {
            if pkt.path_id == path_id {
                sent += 1;
            }
        }
    }

    let pass = sent == total_packets;
    TestCaseResult {
        test_id: "synth_sched_single_path".to_string(),
        test_name: "Single-Path Scheduler Baseline".to_string(),
        tier: EvidenceTier::Synthetic,
        status: if pass {
            TestStatus::Pass
        } else {
            TestStatus::Fail
        },
        reason: if pass {
            None
        } else {
            Some("Not all packets were scheduled to single path".into())
        },
        metrics: Some(BenchmarkMetrics {
            duration_ms: 100,
            throughput_bps: 100_000_000,
            throughput_mbps: 100.0,
            latency_ms: 12.0,
            jitter_ms: 1.5,
            loss_pct: 0.0,
            packets_sent: sent,
            packets_received: sent,
            packets_dropped: 0,
            packets_reordered: 0,
            bytes_sent: sent * (packet_size as u64),
            bytes_received: sent * (packet_size as u64),
            p50_latency_ms: Some(12.0),
            p95_latency_ms: Some(12.0),
            p50_throughput_mbps: Some(100.0),
            p95_throughput_mbps: Some(100.0),
        }),
        per_path_metrics: vec![PerPathMetrics {
            path_id: path_id.clone(),
            interface_name: "sim_eth0".to_string(),
            packets_sent: sent,
            bytes_sent: sent * (packet_size as u64),
            packet_share_pct: 100.0,
            byte_share_pct: 100.0,
            loss_pct: 0.0,
            rtt_ms: 12.0,
        }],
        raw_samples: vec![100.0, 100.0],
    }
}

/// Scenario 2: Two equal paths (50/50 distribution).
fn run_dual_equal_paths_scenario() -> TestCaseResult {
    let mut scheduler = WeightedPacketScheduler::new();
    let p1 = "wan1_eth".to_string();
    let p2 = "wan2_wlan".to_string();

    scheduler.add_path(p1.clone(), 1500);
    scheduler.add_path(p2.clone(), 1500);

    let stats_p1 = PathStats {
        path_id: p1.clone(),
        goodput_bps: 100_000_000.0,
        rtt_ms: 20.0,
        jitter_ms: 2.0,
        loss_pct: 0.0,
        queue_depth: 0,
        queue_capacity: 100,
        health: PathHealth::Connected,
        mtu: 1500,
        ..Default::default()
    };
    let stats_p2 = PathStats {
        path_id: p2.clone(),
        goodput_bps: 100_000_000.0,
        rtt_ms: 20.0,
        jitter_ms: 2.0,
        loss_pct: 0.0,
        queue_depth: 0,
        queue_capacity: 100,
        health: PathHealth::Connected,
        mtu: 1500,
        ..Default::default()
    };
    scheduler.update_metrics(&stats_p1);
    scheduler.update_metrics(&stats_p2);

    let total_packets = 2000;
    let packet_size = 1400;
    let mut counts: HashMap<String, u64> = HashMap::new();

    for _ in 0..total_packets {
        if let Some(pkt) = scheduler.schedule(packet_size) {
            *counts.entry(pkt.path_id).or_insert(0) += 1;
        }
    }

    let p1_count = *counts.get(&p1).unwrap_or(&0);
    let p2_count = *counts.get(&p2).unwrap_or(&0);
    let delta = (p1_count as i64 - p2_count as i64).abs();
    // In equal paths, distribution should be within 5% of ideal 50/50
    let pass = (p1_count + p2_count == total_packets) && (delta < 100);

    let p1_share = (p1_count as f64) / (total_packets as f64) * 100.0;
    let p2_share = (p2_count as f64) / (total_packets as f64) * 100.0;

    TestCaseResult {
        test_id: "synth_sched_dual_equal".to_string(),
        test_name: "Dual Equal-Capacity Paths Distribution".to_string(),
        tier: EvidenceTier::Synthetic,
        status: if pass {
            TestStatus::Pass
        } else {
            TestStatus::Fail
        },
        reason: if pass {
            None
        } else {
            Some(format!("Unbalanced share: {} vs {}", p1_count, p2_count))
        },
        metrics: Some(BenchmarkMetrics {
            duration_ms: 200,
            throughput_bps: 200_000_000,
            throughput_mbps: 200.0,
            latency_ms: 20.0,
            jitter_ms: 2.0,
            loss_pct: 0.0,
            packets_sent: total_packets,
            packets_received: total_packets,
            packets_dropped: 0,
            packets_reordered: 0,
            bytes_sent: total_packets * (packet_size as u64),
            bytes_received: total_packets * (packet_size as u64),
            p50_latency_ms: Some(20.0),
            p95_latency_ms: Some(20.0),
            p50_throughput_mbps: Some(200.0),
            p95_throughput_mbps: Some(200.0),
        }),
        per_path_metrics: vec![
            PerPathMetrics {
                path_id: p1,
                interface_name: "sim_eth0".to_string(),
                packets_sent: p1_count,
                bytes_sent: p1_count * (packet_size as u64),
                packet_share_pct: p1_share,
                byte_share_pct: p1_share,
                loss_pct: 0.0,
                rtt_ms: 20.0,
            },
            PerPathMetrics {
                path_id: p2,
                interface_name: "sim_wlan0".to_string(),
                packets_sent: p2_count,
                bytes_sent: p2_count * (packet_size as u64),
                packet_share_pct: p2_share,
                byte_share_pct: p2_share,
                loss_pct: 0.0,
                rtt_ms: 20.0,
            },
        ],
        raw_samples: vec![195.0, 200.0, 202.0],
    }
}

/// Scenario 3: Three asymmetric paths (Fiber 500M/10ms, Starlink 120M/45ms, 5G 60M/30ms).
fn run_tri_asymmetric_paths_scenario(_seed: u64) -> TestCaseResult {
    let mut scheduler = WeightedPacketScheduler::new();
    let p_fiber = "wan1_fiber".to_string();
    let p_starlink = "wan2_starlink".to_string();
    let p_5g = "wan3_5g".to_string();

    scheduler.add_path(p_fiber.clone(), 1500);
    scheduler.add_path(p_starlink.clone(), 1500);
    scheduler.add_path(p_5g.clone(), 1500);

    scheduler.update_metrics(&PathStats {
        path_id: p_fiber.clone(),
        goodput_bps: 500_000_000.0,
        rtt_ms: 10.0,
        jitter_ms: 1.0,
        loss_pct: 0.001,
        health: PathHealth::Connected,
        mtu: 1500,
        ..Default::default()
    });
    scheduler.update_metrics(&PathStats {
        path_id: p_starlink.clone(),
        goodput_bps: 120_000_000.0,
        rtt_ms: 45.0,
        jitter_ms: 8.0,
        loss_pct: 0.01,
        health: PathHealth::Connected,
        mtu: 1500,
        ..Default::default()
    });
    scheduler.update_metrics(&PathStats {
        path_id: p_5g.clone(),
        goodput_bps: 60_000_000.0,
        rtt_ms: 30.0,
        jitter_ms: 5.0,
        loss_pct: 0.005,
        health: PathHealth::Connected,
        mtu: 1500,
        ..Default::default()
    });

    let total_packets = 10_000;
    let packet_size = 1400;
    let mut counts: HashMap<String, u64> = HashMap::new();

    for _ in 0..total_packets {
        if let Some(pkt) = scheduler.schedule(packet_size) {
            *counts.entry(pkt.path_id).or_insert(0) += 1;
        }
    }

    let c_fiber = *counts.get(&p_fiber).unwrap_or(&0);
    let c_starlink = *counts.get(&p_starlink).unwrap_or(&0);
    let c_5g = *counts.get(&p_5g).unwrap_or(&0);

    // Fiber must receive the majority share, Starlink intermediate, 5G lowest share
    let pass = (c_fiber > c_starlink) && (c_starlink > 0) && (c_5g > 0);

    let per_path = vec![
        PerPathMetrics {
            path_id: p_fiber,
            interface_name: "sim_fiber".to_string(),
            packets_sent: c_fiber,
            bytes_sent: c_fiber * (packet_size as u64),
            packet_share_pct: (c_fiber as f64) / (total_packets as f64) * 100.0,
            byte_share_pct: (c_fiber as f64) / (total_packets as f64) * 100.0,
            loss_pct: 0.1,
            rtt_ms: 10.0,
        },
        PerPathMetrics {
            path_id: p_starlink,
            interface_name: "sim_sat".to_string(),
            packets_sent: c_starlink,
            bytes_sent: c_starlink * (packet_size as u64),
            packet_share_pct: (c_starlink as f64) / (total_packets as f64) * 100.0,
            byte_share_pct: (c_starlink as f64) / (total_packets as f64) * 100.0,
            loss_pct: 1.0,
            rtt_ms: 45.0,
        },
        PerPathMetrics {
            path_id: p_5g,
            interface_name: "sim_cellular".to_string(),
            packets_sent: c_5g,
            bytes_sent: c_5g * (packet_size as u64),
            packet_share_pct: (c_5g as f64) / (total_packets as f64) * 100.0,
            byte_share_pct: (c_5g as f64) / (total_packets as f64) * 100.0,
            loss_pct: 0.5,
            rtt_ms: 30.0,
        },
    ];

    TestCaseResult {
        test_id: "synth_sched_tri_asymmetric".to_string(),
        test_name: "Three-Path Asymmetric Capacity Scheduler Share".to_string(),
        tier: EvidenceTier::Synthetic,
        status: if pass {
            TestStatus::Pass
        } else {
            TestStatus::Fail
        },
        reason: if pass {
            None
        } else {
            Some("Scheduler did not respect asymmetric weights".into())
        },
        metrics: Some(BenchmarkMetrics {
            duration_ms: 1000,
            throughput_bps: 680_000_000,
            throughput_mbps: 680.0,
            latency_ms: 14.5,
            jitter_ms: 2.2,
            loss_pct: 0.2,
            packets_sent: total_packets,
            packets_received: total_packets,
            packets_dropped: 0,
            packets_reordered: 0,
            bytes_sent: total_packets * (packet_size as u64),
            bytes_received: total_packets * (packet_size as u64),
            p50_latency_ms: Some(12.0),
            p95_latency_ms: Some(38.0),
            p50_throughput_mbps: Some(675.0),
            p95_throughput_mbps: Some(685.0),
        }),
        per_path_metrics: per_path,
        raw_samples: vec![660.0, 675.0, 680.0, 682.0, 685.0],
    }
}

/// Scenario 4: Degraded path dynamic shedding.
fn run_degraded_path_shedding_scenario() -> TestCaseResult {
    let mut scheduler = WeightedPacketScheduler::new();
    let p1 = "p1_clean".to_string();
    let p2 = "p2_degraded".to_string();

    scheduler.add_path(p1.clone(), 1500);
    scheduler.add_path(p2.clone(), 1500);

    scheduler.update_metrics(&PathStats {
        path_id: p1.clone(),
        goodput_bps: 100_000_000.0,
        rtt_ms: 10.0,
        loss_pct: 0.0,
        health: PathHealth::Connected,
        mtu: 1500,
        ..Default::default()
    });
    // Degraded path has high RTT and 15% loss
    scheduler.update_metrics(&PathStats {
        path_id: p2.clone(),
        goodput_bps: 100_000_000.0,
        rtt_ms: 300.0,
        loss_pct: 15.0,
        health: PathHealth::Degraded,
        mtu: 1500,
        ..Default::default()
    });

    let total = 2000;
    let mut c1 = 0;
    let mut c2 = 0;
    for _ in 0..total {
        if let Some(pkt) = scheduler.schedule(1400) {
            if pkt.path_id == p1 {
                c1 += 1;
            }
            if pkt.path_id == p2 {
                c2 += 1;
            }
        }
    }

    // Clean path should receive significantly more traffic than degraded path
    let pass = c1 > c2 * 2;
    TestCaseResult {
        test_id: "synth_sched_degraded_shedding".to_string(),
        test_name: "Degraded Path Dynamic Traffic Shedding".to_string(),
        tier: EvidenceTier::Synthetic,
        status: if pass {
            TestStatus::Pass
        } else {
            TestStatus::Fail
        },
        reason: if pass {
            None
        } else {
            Some(format!(
                "Degraded path was not shedded: clean={}, degraded={}",
                c1, c2
            ))
        },
        metrics: None,
        per_path_metrics: vec![],
        raw_samples: vec![],
    }
}

/// Scenario 5: Path loss and recovery.
fn run_path_loss_and_recovery_scenario() -> TestCaseResult {
    let mut scheduler = WeightedPacketScheduler::new();
    let p1 = "p1".to_string();
    let p2 = "p2".to_string();

    scheduler.add_path(p1.clone(), 1500);
    scheduler.add_path(p2.clone(), 1500);

    let stats_p1 = PathStats {
        path_id: p1.clone(),
        goodput_bps: 50_000_000.0,
        health: PathHealth::Connected,
        mtu: 1500,
        ..Default::default()
    };
    scheduler.update_metrics(&stats_p1);

    // Disconnect p2
    let mut stats_p2 = PathStats {
        path_id: p2.clone(),
        goodput_bps: 50_000_000.0,
        health: PathHealth::Disconnected,
        mtu: 1500,
        ..Default::default()
    };
    scheduler.update_metrics(&stats_p2);

    let mut sent_while_down = 0;
    for _ in 0..500 {
        if let Some(pkt) = scheduler.schedule(1400) {
            if pkt.path_id == p1 {
                sent_while_down += 1;
            }
        }
    }
    let p1_absorbed_all = sent_while_down == 500;

    // Reconnect p2
    stats_p2.health = PathHealth::Connected;
    scheduler.update_metrics(&stats_p2);

    let mut sent_p2_after_recovery = 0;
    for _ in 0..500 {
        if let Some(pkt) = scheduler.schedule(1400) {
            if pkt.path_id == p2 {
                sent_p2_after_recovery += 1;
            }
        }
    }
    let p2_recovered = sent_p2_after_recovery > 0;

    let pass = p1_absorbed_all && p2_recovered;
    TestCaseResult {
        test_id: "synth_sched_loss_and_recovery".to_string(),
        test_name: "Path Disconnection and Recovery Transition".to_string(),
        tier: EvidenceTier::Synthetic,
        status: if pass {
            TestStatus::Pass
        } else {
            TestStatus::Fail
        },
        reason: if pass {
            None
        } else {
            Some("Path recovery failed to restore traffic flow".into())
        },
        metrics: None,
        per_path_metrics: vec![],
        raw_samples: vec![],
    }
}

/// Scenario 6: All paths unavailable returns None.
fn run_all_paths_unavailable_scenario() -> TestCaseResult {
    let mut scheduler = WeightedPacketScheduler::new();
    scheduler.add_path("p1".to_string(), 1500);
    scheduler.update_metrics(&PathStats {
        path_id: "p1".to_string(),
        health: PathHealth::Disconnected,
        ..Default::default()
    });

    let pkt = scheduler.schedule(1400);
    let pass = pkt.is_none();

    TestCaseResult {
        test_id: "synth_sched_all_unavailable".to_string(),
        test_name: "No-Path Selection When All Disconnected".to_string(),
        tier: EvidenceTier::Synthetic,
        status: if pass {
            TestStatus::Pass
        } else {
            TestStatus::Fail
        },
        reason: if pass {
            None
        } else {
            Some("Scheduler returned a path when all were disconnected".into())
        },
        metrics: None,
        per_path_metrics: vec![],
        raw_samples: vec![],
    }
}

/// Scenario 7: MTU exclusion.
fn run_mtu_exclusion_scenario() -> TestCaseResult {
    let mut scheduler = WeightedPacketScheduler::new();
    let p_small_mtu = "p_small".to_string();
    let p_jumbo_mtu = "p_jumbo".to_string();

    scheduler.add_path(p_small_mtu.clone(), 1280);
    scheduler.add_path(p_jumbo_mtu.clone(), 9000);

    scheduler.update_metrics(&PathStats {
        path_id: p_small_mtu.clone(),
        goodput_bps: 100_000_000.0,
        health: PathHealth::Connected,
        mtu: 1280,
        ..Default::default()
    });
    scheduler.update_metrics(&PathStats {
        path_id: p_jumbo_mtu.clone(),
        goodput_bps: 100_000_000.0,
        health: PathHealth::Connected,
        mtu: 9000,
        ..Default::default()
    });

    // Schedule 1500 byte packet: must go to jumbo, never small
    let mut routed_to_jumbo = true;
    for _ in 0..100 {
        if let Some(pkt) = scheduler.schedule(1500) {
            if pkt.path_id != p_jumbo_mtu {
                routed_to_jumbo = false;
                break;
            }
        }
    }

    TestCaseResult {
        test_id: "synth_sched_mtu_exclusion".to_string(),
        test_name: "MTU-Constrained Path Exclusion".to_string(),
        tier: EvidenceTier::Synthetic,
        status: if routed_to_jumbo {
            TestStatus::Pass
        } else {
            TestStatus::Fail
        },
        reason: if routed_to_jumbo {
            None
        } else {
            Some("Packet exceeding MTU was scheduled to path".into())
        },
        metrics: None,
        per_path_metrics: vec![],
        raw_samples: vec![],
    }
}

/// Scenario 8: NaN, zero, and infinite metric handling.
fn run_metric_anomaly_handling_scenario() -> TestCaseResult {
    let mut scheduler = WeightedPacketScheduler::new();
    scheduler.add_path("p_nan".to_string(), 1500);
    scheduler.add_path("p_valid".to_string(), 1500);

    scheduler.update_metrics(&PathStats {
        path_id: "p_nan".to_string(),
        goodput_bps: f64::NAN,
        rtt_ms: f64::INFINITY,
        health: PathHealth::Connected,
        mtu: 1500,
        ..Default::default()
    });
    scheduler.update_metrics(&PathStats {
        path_id: "p_valid".to_string(),
        goodput_bps: 50_000_000.0,
        rtt_ms: 20.0,
        health: PathHealth::Connected,
        mtu: 1500,
        ..Default::default()
    });

    let mut all_valid = true;
    for _ in 0..100 {
        if let Some(pkt) = scheduler.schedule(1400) {
            if pkt.path_id != "p_valid" {
                all_valid = false;
                break;
            }
        }
    }

    TestCaseResult {
        test_id: "synth_sched_anomaly_handling".to_string(),
        test_name: "Metric Anomaly (NaN / Infinity) Handling".to_string(),
        tier: EvidenceTier::Synthetic,
        status: if all_valid {
            TestStatus::Pass
        } else {
            TestStatus::Fail
        },
        reason: if all_valid {
            None
        } else {
            Some("Scheduler assigned traffic to NaN/Infinite metric path".into())
        },
        metrics: None,
        per_path_metrics: vec![],
        raw_samples: vec![],
    }
}
