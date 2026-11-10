//! Loopback UDP socket and authenticated framing provider.
//!
//! Exercises actual OS UDP sockets on 127.0.0.1, framing serialization,
//! and constant-time HMAC tag verification.

use crate::models::{
    calculate_percentiles, BenchmarkMetrics, EvidenceTier, TestCaseResult, TestStatus,
};
use std::net::UdpSocket;
use std::time::Instant;
use tunnel_protocol::framing::{decode_frame, encode_frame, FramingLayer};

/// Runs the loopback socket and framing benchmark suite.
pub fn run_loopback_suite() -> Vec<TestCaseResult> {
    vec![run_loopback_framing_socket_test()]
}

fn run_loopback_framing_socket_test() -> TestCaseResult {
    let key = [0x5au8; 32];
    let mut sender_framing = FramingLayer::new(key);
    let mut receiver_framing = FramingLayer::new(key);

    // Bind local loopback sockets
    let rx_socket = match UdpSocket::bind("127.0.0.1:0") {
        Ok(s) => s,
        Err(e) => {
            return TestCaseResult {
                test_id: "loopback_rx_bind".to_string(),
                test_name: "Loopback Socket Binding".to_string(),
                tier: EvidenceTier::Loopback,
                status: TestStatus::Fail,
                reason: Some(format!("Failed to bind RX socket: {}", e)),
                metrics: None,
                per_path_metrics: vec![],
                raw_samples: vec![],
            }
        }
    };
    rx_socket.set_nonblocking(true).unwrap_or(());
    let rx_addr = rx_socket.local_addr().unwrap();

    let tx_socket = match UdpSocket::bind("127.0.0.1:0") {
        Ok(s) => s,
        Err(e) => {
            return TestCaseResult {
                test_id: "loopback_tx_bind".to_string(),
                test_name: "Loopback Socket Binding".to_string(),
                tier: EvidenceTier::Loopback,
                status: TestStatus::Fail,
                reason: Some(format!("Failed to bind TX socket: {}", e)),
                metrics: None,
                per_path_metrics: vec![],
                raw_samples: vec![],
            }
        }
    };

    let packet_count = 100;
    let payload = vec![0x42u8; 1024];
    let start = Instant::now();

    let mut sent = 0;
    let mut received = 0;
    let mut bytes_sent = 0;
    let mut bytes_received = 0;
    let mut buf = [0u8; 4096];
    let mut rtt_samples = Vec::with_capacity(packet_count);

    for _ in 0..packet_count {
        let pkt_start = Instant::now();
        let frame = match sender_framing.encapsulate(payload.clone()) {
            Ok(f) => f,
            Err(e) => {
                return TestCaseResult {
                    test_id: "loopback_framing".to_string(),
                    test_name: "Loopback Authenticated Framing".to_string(),
                    tier: EvidenceTier::Loopback,
                    status: TestStatus::Fail,
                    reason: Some(format!("Framing encapsulation error: {}", e)),
                    metrics: None,
                    per_path_metrics: vec![],
                    raw_samples: vec![],
                }
            }
        };

        let wire_bytes = encode_frame(&frame);

        if tx_socket.send_to(&wire_bytes, rx_addr).is_ok() {
            sent += 1;
            bytes_sent += wire_bytes.len() as u64;
        }

        // Read immediately
        if let Ok((n, _)) = rx_socket.recv_from(&mut buf) {
            if let Ok(deserialized) = decode_frame(&buf[..n]) {
                if let Ok((_seq, decapsulated_payload)) =
                    receiver_framing.decapsulate(&deserialized)
                {
                    if decapsulated_payload == payload {
                        received += 1;
                        bytes_received += n as u64;
                        rtt_samples.push(pkt_start.elapsed().as_secs_f64() * 1000.0);
                    }
                }
            }
        }
    }

    // Drain any remaining packets
    let drain_start = Instant::now();
    while received < sent && drain_start.elapsed().as_millis() < 50 {
        if let Ok((n, _)) = rx_socket.recv_from(&mut buf) {
            if let Ok(deserialized) = decode_frame(&buf[..n]) {
                if let Ok((_seq, decapsulated_payload)) =
                    receiver_framing.decapsulate(&deserialized)
                {
                    if decapsulated_payload == payload {
                        received += 1;
                        bytes_received += n as u64;
                        rtt_samples.push(drain_start.elapsed().as_secs_f64() * 1000.0);
                    }
                }
            }
        }
    }

    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_millis().max(1) as u64;
    let throughput_bps = (bytes_received * 8 * 1000) / elapsed_ms;
    let throughput_mbps = (throughput_bps as f64) / 1_000_000.0;
    let pass = received == sent;

    let avg_latency_ms = if !rtt_samples.is_empty() {
        rtt_samples.iter().sum::<f64>() / rtt_samples.len() as f64
    } else {
        0.0
    };
    let jitter_ms = if rtt_samples.len() > 1 {
        rtt_samples
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .sum::<f64>()
            / (rtt_samples.len() - 1) as f64
    } else {
        0.0
    };
    let (p50_lat, p95_lat) = calculate_percentiles(&rtt_samples)
        .map(|(p50, p95)| (Some(p50), Some(p95)))
        .unwrap_or((None, None));

    TestCaseResult {
        test_id: "loopback_udp_framing".to_string(),
        test_name: "Loopback Authenticated Framing & UDP Socket Wire Test".to_string(),
        tier: EvidenceTier::Loopback,
        status: if pass {
            TestStatus::Pass
        } else {
            TestStatus::Fail
        },
        reason: if pass {
            None
        } else {
            Some(format!(
                "Packet loss on loopback: sent {}, received {}",
                sent, received
            ))
        },
        metrics: Some(BenchmarkMetrics {
            duration_ms: elapsed_ms,
            throughput_bps,
            throughput_mbps,
            latency_ms: avg_latency_ms,
            jitter_ms,
            loss_pct: if sent > 0 {
                ((sent - received) as f64) / (sent as f64) * 100.0
            } else {
                0.0
            },
            packets_sent: sent,
            packets_received: received,
            packets_dropped: sent - received,
            packets_reordered: 0,
            bytes_sent,
            bytes_received,
            p50_latency_ms: p50_lat,
            p95_latency_ms: p95_lat,
            p50_throughput_mbps: None,
            p95_throughput_mbps: None,
        }),
        per_path_metrics: vec![],
        raw_samples: vec![throughput_mbps],
    }
}
