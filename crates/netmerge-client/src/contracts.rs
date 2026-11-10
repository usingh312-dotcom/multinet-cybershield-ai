//! contracts — versioned JSON IPC contract between MultiNet and Net Merge.
//!
//! Every request/response pair is a single JSON object terminated by a newline
//! on the named pipe. `pipe_version` must equal [`super::PIPE_VERSION`].
//!
//! Method reference (version 1):
//! - `health`      → service liveness + negotiated pipe version
//! - `status`      → on/off state + per-adapter snapshot
//! - `adapters`    → discovered adapters with per-adapter metrics
//! - `start`       → activate Mode A on the given adapters
//! - `stop`        → deactivate Mode A and restore routes
//! - `set_limit`   → per-adapter speed cap in Mbit/s
//! - `mode_b_status` → query Rust bonding-engine status (mutual exclusion)

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub use crate::PIPE_VERSION;

/// Incoming request from MultiNet to Net Merge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// Client-generated correlation id, echoed in the response.
    pub id: String,
    /// IPC protocol version the client speaks.
    pub pipe_version: u16,
    /// Method name.
    pub method: Method,
    /// Optional method parameters.
    pub params: Option<Value>,
}

/// Outgoing response from Net Merge to MultiNet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Echoed request id.
    pub id: String,
    /// Whether the method succeeded.
    pub ok: bool,
    /// Success payload, present only when `ok` is true.
    pub result: Option<Value>,
    /// Error detail, present only when `ok` is false.
    pub error: Option<ServiceError>,
}

/// Method names understood by the Net Merge service (version 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    Health,
    Status,
    Adapters,
    Start,
    Stop,
    SetLimit,
    ModeBStatus,
    SetModeB,
}

/// Parameters for `set_mode_b`.
// The client PID is obtained by the server via GetNamedPipeClientProcessId()
// on the authenticated pipe connection, not from a caller-supplied value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetModeBParams {
    pub active: bool,
}

/// Structured service error returned in a failed response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

/// Parameters for `start`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartParams {
    /// Adapter names to merge (empty = all discovered non-virtual adapters).
    pub adapters: Vec<String>,
    /// Optional fixed weights by adapter name.
    pub weights: Option<std::collections::HashMap<String, f64>>,
    /// Derive weights from measured throughput when true.
    pub auto_weight: bool,
    /// IPv6 handling: auto, on, off.
    pub ipv6: String,
    /// TUN MTU in bytes.
    pub mtu: u16,
}

/// Result for `status` / `adapters`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterStatus {
    pub name: String,
    pub index: u32,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
    pub state: String, // up / down / unknown
    pub weight: f64,
    pub active_flows: u64,
    pub total_flows: u64,
    pub failed_flows: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub rate_in_bps: f64,
    pub rate_out_bps: f64,
    pub link_rate_in_bps: f64,
    pub link_rate_out_bps: f64,
    pub limit_mbps: f64,
}

/// Result for `health`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResult {
    pub service: String,
    pub version: String,
    pub pipe_version: u16,
    pub mode_a_active: bool,
    pub mode_b_active: bool,
    pub on: bool,
}

impl Request {
    /// Build a health request.
    pub fn health() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            pipe_version: super::PIPE_VERSION,
            method: Method::Health,
            params: None,
        }
    }

    /// Build a status request.
    pub fn status() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            pipe_version: super::PIPE_VERSION,
            method: Method::Status,
            params: None,
        }
    }

    /// Build an adapters request.
    pub fn adapters() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            pipe_version: super::PIPE_VERSION,
            method: Method::Adapters,
            params: None,
        }
    }

    /// Build a start request.
    pub fn start(params: StartParams) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            pipe_version: super::PIPE_VERSION,
            method: Method::Start,
            params: Some(serde_json::to_value(params).unwrap_or(Value::Null)),
        }
    }

    /// Build a stop request.
    pub fn stop() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            pipe_version: super::PIPE_VERSION,
            method: Method::Stop,
            params: None,
        }
    }

    /// Build a set-limit request.
    pub fn set_limit(adapter: String, mbit_per_sec: f64) -> Self {
        let mut map = serde_json::Map::new();
        map.insert("adapter".into(), Value::String(adapter));
        map.insert(
            "mbit_per_sec".into(),
            Value::Number(
                serde_json::Number::from_f64(mbit_per_sec).unwrap_or(serde_json::Number::from(0)),
            ),
        );
        Self {
            id: Uuid::new_v4().to_string(),
            pipe_version: super::PIPE_VERSION,
            method: Method::SetLimit,
            params: Some(Value::Object(map)),
        }
    }

    /// Build a mode-b-status request.
    pub fn mode_b_status() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            pipe_version: super::PIPE_VERSION,
            method: Method::ModeBStatus,
            params: None,
        }
    }

    /// Build a set-mode-b request.
    /// The client PID is obtained by the server via GetNamedPipeClientProcessId()
    /// on the authenticated pipe connection, not from a caller-supplied value.
    pub fn set_mode_b(active: bool) -> Self {
        let params = SetModeBParams { active };
        Self {
            id: Uuid::new_v4().to_string(),
            pipe_version: super::PIPE_VERSION,
            method: Method::SetModeB,
            params: Some(serde_json::to_value(params).unwrap_or(Value::Null)),
        }
    }
}

/// Look up a named field in a JSON object value.
pub fn value_get<'a>(obj: &'a Value, key: &'a str) -> Option<&'a Value> {
    obj.as_object().and_then(|o| o.get(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_request_roundtrips() {
        let req = Request::health();
        let json = serde_json::to_string(&req).unwrap();
        let back: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(back.method, Method::Health);
        assert_eq!(back.pipe_version, super::PIPE_VERSION);
    }

    #[test]
    fn start_params_roundtrip() {
        let params = StartParams {
            adapters: vec!["Wi-Fi".into()],
            weights: None,
            auto_weight: true,
            ipv6: "auto".into(),
            mtu: 9000,
        };
        let json = serde_json::to_string(&params).unwrap();
        let back: StartParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back.adapters, vec!["Wi-Fi"]);
        assert!(back.auto_weight);
        assert_eq!(back.mtu, 9000);
    }

    #[test]
    fn response_ok_shape() {
        let resp = Response {
            id: "abc".into(),
            ok: true,
            result: Some(Value::String("ok".into())),
            error: None,
        };
        assert!(resp.ok);
        assert!(resp.error.is_none());
    }

    #[test]
    fn set_mode_b_request_roundtrips() {
        let req = Request::set_mode_b(true);
        let json = serde_json::to_string(&req).unwrap();
        let back: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(back.method, Method::SetModeB);
        assert_eq!(back.pipe_version, super::PIPE_VERSION);
        let params: SetModeBParams = serde_json::from_value(back.params.unwrap()).unwrap();
        assert!(params.active);
    }

    #[test]
    fn response_error_shape() {
        let resp = Response {
            id: "def".into(),
            ok: false,
            result: None,
            error: Some(ServiceError {
                code: "unauthorized".into(),
                message: "client not authorized".into(),
                retryable: false,
            }),
        };
        assert!(!resp.ok);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, "unauthorized");
    }

    #[test]
    fn version_mismatch_detected() {
        let req = Request {
            id: "v1".into(),
            pipe_version: 999,
            method: Method::Health,
            params: None,
        };
        assert_ne!(req.pipe_version, super::PIPE_VERSION);
    }

    #[test]
    fn all_methods_have_unique_names() {
        let methods = [
            Method::Health,
            Method::Status,
            Method::Adapters,
            Method::Start,
            Method::Stop,
            Method::SetLimit,
            Method::ModeBStatus,
            Method::SetModeB,
        ];
        let serialized: Vec<String> = methods
            .iter()
            .map(|m| serde_json::to_string(m).unwrap())
            .collect();
        assert_eq!(serialized.len(), 8);
        // All method names should be unique when serialized.
        for i in 0..serialized.len() {
            for j in i + 1..serialized.len() {
                assert_ne!(
                    serialized[i], serialized[j],
                    "method names collide: {:?} vs {:?}",
                    methods[i], methods[j]
                );
            }
        }
    }

    #[test]
    fn start_params_default_weights_none() {
        let params = StartParams {
            adapters: vec!["Wi-Fi".into()],
            weights: None,
            auto_weight: true,
            ipv6: "auto".into(),
            mtu: 1500,
        };
        assert!(params.weights.is_none());
        assert_eq!(params.mtu, 1500);
    }

    #[test]
    fn adapter_status_roundtrips() {
        let status = AdapterStatus {
            name: "Wi-Fi".into(),
            index: 7,
            ipv4: Some("192.168.1.10".into()),
            ipv6: None,
            state: "up".into(),
            weight: 1.0,
            active_flows: 3,
            total_flows: 42,
            failed_flows: 0,
            bytes_in: 1_000_000,
            bytes_out: 500_000,
            rate_in_bps: 10_000_000.0,
            rate_out_bps: 5_000_000.0,
            link_rate_in_bps: 50_000_000.0,
            link_rate_out_bps: 10_000_000.0,
            limit_mbps: 0.0,
        };
        let json = serde_json::to_string(&status).unwrap();
        let back: AdapterStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Wi-Fi");
        assert_eq!(back.index, 7);
        assert_eq!(back.ipv4, Some("192.168.1.10".into()));
        assert_eq!(back.state, "up");
        assert_eq!(back.weight, 1.0);
        assert_eq!(back.active_flows, 3);
        assert_eq!(back.limit_mbps, 0.0);
    }

    #[test]
    fn health_result_roundtrips() {
        let health = HealthResult {
            service: "net-merge".into(),
            version: "0.3.7-multinet".into(),
            pipe_version: super::PIPE_VERSION,
            mode_a_active: true,
            mode_b_active: false,
            on: true,
        };
        let json = serde_json::to_string(&health).unwrap();
        let back: HealthResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.service, "net-merge");
        assert_eq!(back.pipe_version, super::PIPE_VERSION);
        assert!(back.mode_a_active);
        assert!(!back.mode_b_active);
        assert!(back.on);
    }
}
