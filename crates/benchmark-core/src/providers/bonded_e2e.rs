//! Bonded end-to-end multi-WAN qualification suite.
//!
//! Because MultiNet's remote relay data plane (`relay/bonding-server`), packet
//! reassembly queue, and bidirectional return transport are not yet implemented,
//! this suite registers all bonded E2E tests as BLOCKED with clear architectural rationale.
//!
//! Under no circumstance may any of these tests be marked PASS without real
//! end-to-end verification.

use crate::models::{EvidenceTier, TestCaseResult, TestStatus};

/// Returns the formal suite of bonded end-to-end multi-WAN qualification tests.
pub fn get_bonded_e2e_suite() -> Vec<TestCaseResult> {
    vec![
        TestCaseResult {
            test_id: "bonded_e2e_2wan_single_flow".to_string(),
            test_name: "Two-WAN Single-Flow Throughput Aggregation".to_string(),
            tier: EvidenceTier::BondedE2e,
            status: TestStatus::Blocked,
            reason: Some(
                "BLOCKED: Remote relay ingress (relay/bonding-server) is a foundation placeholder. End-to-end multi-WAN bonding requires deployed relay ingress and return-path routing."
                    .to_string(),
            ),
            metrics: None,
            per_path_metrics: vec![],
            raw_samples: vec![],
        },
        TestCaseResult {
            test_id: "bonded_e2e_3wan_asymmetric".to_string(),
            test_name: "Three-WAN Asymmetric Flow Bonding (Fiber + Starlink + 5G)".to_string(),
            tier: EvidenceTier::BondedE2e,
            status: TestStatus::Blocked,
            reason: Some(
                "BLOCKED: Requires active 3-path physical/virtual multi-homing and remote relay ingress."
                    .to_string(),
            ),
            metrics: None,
            per_path_metrics: vec![],
            raw_samples: vec![],
        },
        TestCaseResult {
            test_id: "bonded_e2e_relay_reassembly".to_string(),
            test_name: "Relay Ingress, Reorder-Tolerant Reassembly & Deduplication".to_string(),
            tier: EvidenceTier::BondedE2e,
            status: TestStatus::Blocked,
            reason: Some(
                "BLOCKED: crates/bonding-engine and crates/tunnel-protocol currently lack a reassembly queue and multipath sliding-window replay filter."
                    .to_string(),
            ),
            metrics: None,
            per_path_metrics: vec![],
            raw_samples: vec![],
        },
        TestCaseResult {
            test_id: "bonded_e2e_single_flow_failover".to_string(),
            test_name: "Long-Lived TCP Flow Seamless Failover Continuity".to_string(),
            tier: EvidenceTier::BondedE2e,
            status: TestStatus::Blocked,
            reason: Some(
                "BLOCKED: TCP flow migration requires an active relay proxy terminating the remote connection. Blocked by relay server placeholder."
                    .to_string(),
            ),
            metrics: None,
            per_path_metrics: vec![],
            raw_samples: vec![],
        },
        TestCaseResult {
            test_id: "bonded_e2e_return_traffic".to_string(),
            test_name: "Bidirectional Symmetric Return-Path Wire Delivery".to_string(),
            tier: EvidenceTier::BondedE2e,
            status: TestStatus::Blocked,
            reason: Some(
                "BLOCKED: BondedSocket is currently unidirectional (send only). Return path receiver is not implemented."
                    .to_string(),
            ),
            metrics: None,
            per_path_metrics: vec![],
            raw_samples: vec![],
        },
    ]
}
