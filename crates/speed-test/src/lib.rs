//! speed-test — real network measurements against a deployed speed test service.
//!
//! Every value in a [`SpeedTestReport`] is derived from bytes that actually
//! crossed the socket, timed with the monotonic [`std::time::Instant`]. Nothing
//! is estimated, and an aborted transfer never produces a report.

use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;

pub const DEFAULT_DOWNLOAD_BYTES: u64 = 16 * 1024 * 1024;
pub const DEFAULT_UPLOAD_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_TRANSFER_BYTES: u64 = 64 * 1024 * 1024;
pub const DEFAULT_PING_SAMPLES: u32 = 12;
pub const MAX_PING_SAMPLES: u32 = 50;

#[derive(Debug, Error)]
pub enum SpeedTestError {
    #[error("invalid speed test configuration: {0}")]
    InvalidConfig(String),
    #[error("speed test server rejected the request: {0}")]
    Server(String),
    #[error("speed test transport failed: {0}")]
    Transport(String),
    #[error("speed test was cancelled")]
    Cancelled,
}

impl From<reqwest::Error> for SpeedTestError {
    fn from(value: reqwest::Error) -> Self {
        SpeedTestError::Transport(value.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedTestConfig {
    /// Base URL of the speed test service, for example `https://127.0.0.1:9443`.
    pub server_url: String,
    /// Local address of the selected interface. The socket is bound to it before
    /// any transfer, so the OS routes traffic out of that adapter.
    pub interface_local_ip: Option<IpAddr>,
    pub download_bytes: u64,
    pub upload_bytes: u64,
    pub ping_samples: u32,
    pub timeout_secs: u64,
    /// Accept a self-signed certificate, but only when the server is loopback.
    pub allow_self_signed_loopback: bool,
}

impl Default for SpeedTestConfig {
    fn default() -> Self {
        Self {
            server_url: "https://127.0.0.1:9443".into(),
            interface_local_ip: None,
            download_bytes: DEFAULT_DOWNLOAD_BYTES,
            upload_bytes: DEFAULT_UPLOAD_BYTES,
            ping_samples: DEFAULT_PING_SAMPLES,
            timeout_secs: 30,
            allow_self_signed_loopback: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedTestReport {
    pub server_url: String,
    pub server_name: String,
    pub server_observed_client_ip: String,
    pub interface_local_ip: Option<IpAddr>,
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub ping_ms: f64,
    pub jitter_ms: f64,
    pub loss_pct: f64,
    pub download_bytes: u64,
    pub upload_bytes: u64,
    pub ping_samples: u32,
    pub ping_received: u32,
    pub rtt_samples: Vec<f64>,
    pub duration_ms: u64,
    pub started_at: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeedTestStage {
    Preparing,
    Latency,
    Download,
    Upload,
    Finalizing,
    Complete,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedTestProgress {
    pub stage: SpeedTestStage,
    pub fraction: f64,
    pub current_mbps: Option<f64>,
    pub detail: String,
}

impl SpeedTestProgress {
    fn new(stage: SpeedTestStage, fraction: f64, detail: impl Into<String>) -> Self {
        Self {
            stage,
            fraction: fraction.clamp(0.0, 1.0),
            current_mbps: None,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct PingResponse {
    server: String,
    observed_ip: String,
    nonce: String,
}

#[derive(Debug, Deserialize)]
struct UploadResponse {
    received: u64,
}

fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

pub fn is_loopback_host(url: &reqwest::Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false)
}

pub fn is_loopback_url_str(url_str: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(url_str) else {
        return false;
    };
    is_loopback_host(&url)
}

fn validate(config: &SpeedTestConfig) -> Result<reqwest::Url, SpeedTestError> {
    let url = reqwest::Url::parse(&config.server_url)
        .map_err(|e| SpeedTestError::InvalidConfig(format!("server URL: {e}")))?;
    if url.scheme() != "https" {
        return Err(SpeedTestError::InvalidConfig(
            "speed test server must use HTTPS".into(),
        ));
    }
    if config.allow_self_signed_loopback && !is_loopback_host(&url) {
        return Err(SpeedTestError::InvalidConfig(
            "self-signed certificates are only accepted for loopback servers".into(),
        ));
    }
    if !(1..=MAX_TRANSFER_BYTES).contains(&config.download_bytes) {
        return Err(SpeedTestError::InvalidConfig(format!(
            "download size must be between 1 and {MAX_TRANSFER_BYTES} bytes"
        )));
    }
    if !(1..=MAX_TRANSFER_BYTES).contains(&config.upload_bytes) {
        return Err(SpeedTestError::InvalidConfig(format!(
            "upload size must be between 1 and {MAX_TRANSFER_BYTES} bytes"
        )));
    }
    if config.ping_samples == 0 || config.ping_samples > MAX_PING_SAMPLES {
        return Err(SpeedTestError::InvalidConfig(format!(
            "ping samples must be between 1 and {MAX_PING_SAMPLES}"
        )));
    }
    if config.timeout_secs == 0 || config.timeout_secs > 300 {
        return Err(SpeedTestError::InvalidConfig(
            "timeout must be between 1 and 300 seconds".into(),
        ));
    }
    Ok(url)
}

fn cancelled(cancel: &AtomicBool) -> Result<(), SpeedTestError> {
    if cancel.load(Ordering::Relaxed) {
        Err(SpeedTestError::Cancelled)
    } else {
        Ok(())
    }
}

/// Execute a complete measurement. `progress` is called as the test advances so
/// the UI can show a live stage and rate.
pub async fn run_speed_test<F>(
    config: &SpeedTestConfig,
    cancel: &AtomicBool,
    mut progress: F,
) -> Result<SpeedTestReport, SpeedTestError>
where
    F: FnMut(&SpeedTestProgress),
{
    let base = validate(config)?;
    progress(&SpeedTestProgress::new(
        SpeedTestStage::Preparing,
        0.0,
        "Opening a socket on the selected interface",
    ));

    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .user_agent("multinet-cybershield-speedtest/0.1");
    if let Some(ip) = config.interface_local_ip {
        builder = builder.local_address(ip);
    }
    if config.allow_self_signed_loopback {
        builder = builder.danger_accept_invalid_certs(true);
    }
    let client = builder
        .build()
        .map_err(|e| SpeedTestError::Transport(e.to_string()))?;

    let started = Instant::now();
    let started_at = now_rfc3339();
    cancelled(cancel)?;

    // --- Latency / jitter / loss -------------------------------------------
    progress(&SpeedTestProgress::new(
        SpeedTestStage::Latency,
        0.05,
        "Sending latency probes",
    ));

    let ping_url = base
        .join("ping")
        .map_err(|e| SpeedTestError::InvalidConfig(e.to_string()))?;
    let mut rtts: Vec<f64> = Vec::with_capacity(config.ping_samples as usize);
    let mut failures = 0u32;
    let mut server_name = String::new();
    let mut observed_ip = String::new();

    for sample in 0..config.ping_samples {
        cancelled(cancel)?;
        let nonce = format!("{}-{}", started_at, sample);
        let mut url = ping_url.clone();
        url.query_pairs_mut().append_pair("nonce", &nonce);
        let probe = Instant::now();
        match client.get(url).send().await {
            Ok(response) => {
                let status = response.status();
                let body = response.bytes().await;
                match body {
                    Ok(body) if status.is_success() => {
                        match serde_json::from_slice::<PingResponse>(&body) {
                            Ok(parsed) if parsed.nonce == nonce => {
                                rtts.push(probe.elapsed().as_secs_f64() * 1000.0);
                                server_name = parsed.server;
                                observed_ip = parsed.observed_ip;
                            }
                            _ => failures += 1,
                        }
                    }
                    _ => failures += 1,
                }
            }
            Err(_) => failures += 1,
        }
        let fraction = 0.05 + 0.20 * ((sample + 1) as f64 / config.ping_samples as f64);
        progress(&SpeedTestProgress::new(
            SpeedTestStage::Latency,
            fraction,
            format!("Probe {}/{}", sample + 1, config.ping_samples),
        ));
    }

    let ping_received = rtts.len() as u32;
    if ping_received == 0 {
        return Err(SpeedTestError::Server(
            "every latency probe failed; the speed test server is unreachable".into(),
        ));
    }
    let ping_ms = rtts.iter().sum::<f64>() / ping_received as f64;
    let jitter_ms = if rtts.len() > 1 {
        rtts.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (rtts.len() - 1) as f64
    } else {
        0.0
    };
    let loss_pct = failures as f64 / config.ping_samples as f64 * 100.0;

    // --- Download -----------------------------------------------------------
    cancelled(cancel)?;
    progress(&SpeedTestProgress::new(
        SpeedTestStage::Download,
        0.28,
        "Downloading measurement payload",
    ));

    let mut download_url = base
        .join("download")
        .map_err(|e| SpeedTestError::InvalidConfig(e.to_string()))?;
    download_url
        .query_pairs_mut()
        .append_pair("bytes", &config.download_bytes.to_string());
    let mut response = client.get(download_url).send().await?;
    if !response.status().is_success() {
        return Err(SpeedTestError::Server(format!(
            "download endpoint returned HTTP {}",
            response.status()
        )));
    }

    let expected = response.content_length().unwrap_or(config.download_bytes);
    let mut received_bytes = 0u64;
    let transfer_start = Instant::now();
    while let Some(chunk) = response.chunk().await? {
        cancelled(cancel)?;
        received_bytes += chunk.len() as u64;
        if expected > 0 && received_bytes <= expected {
            let fraction = 0.28 + 0.30 * (received_bytes as f64 / expected as f64);
            let elapsed = transfer_start.elapsed().as_secs_f64();
            let current = if elapsed > 0.0 {
                Some(received_bytes as f64 * 8.0 / elapsed / 1_000_000.0)
            } else {
                None
            };
            let mut p = SpeedTestProgress::new(
                SpeedTestStage::Download,
                fraction,
                format!("{:.1} MiB received", received_bytes as f64 / 1_048_576.0),
            );
            p.current_mbps = current;
            progress(&p);
        }
    }
    let download_secs = transfer_start.elapsed().as_secs_f64();
    if received_bytes == 0 {
        return Err(SpeedTestError::Server(
            "download endpoint returned no bytes".into(),
        ));
    }
    if received_bytes != config.download_bytes {
        return Err(SpeedTestError::Server(format!(
            "download endpoint sent {received_bytes} bytes, expected {}",
            config.download_bytes
        )));
    }
    let download_mbps = received_bytes as f64 * 8.0 / download_secs / 1_000_000.0;

    // --- Upload -------------------------------------------------------------
    cancelled(cancel)?;
    progress(&SpeedTestProgress::new(
        SpeedTestStage::Upload,
        0.60,
        "Uploading measurement payload",
    ));

    let mut payload = vec![0u8; config.upload_bytes as usize];
    // Non-trivial, incompressible-enough content so the transfer is real work.
    for (index, byte) in payload.iter_mut().enumerate() {
        *byte = (index as u32).wrapping_mul(7_919) as u8;
    }
    let upload_url = base
        .join("upload")
        .map_err(|e| SpeedTestError::InvalidConfig(e.to_string()))?;
    let upload_start = Instant::now();
    let response = client.post(upload_url).body(payload).send().await?;
    if !response.status().is_success() {
        return Err(SpeedTestError::Server(format!(
            "upload endpoint returned HTTP {}",
            response.status()
        )));
    }
    let upload_secs = upload_start.elapsed().as_secs_f64();
    let acknowledged: UploadResponse = response
        .json()
        .await
        .map_err(|e| SpeedTestError::Server(format!("upload response: {e}")))?;
    if acknowledged.received != config.upload_bytes {
        return Err(SpeedTestError::Server(format!(
            "upload server acknowledged {} of {} bytes",
            acknowledged.received, config.upload_bytes
        )));
    }
    let upload_mbps = config.upload_bytes as f64 * 8.0 / upload_secs / 1_000_000.0;

    progress(&SpeedTestProgress::new(
        SpeedTestStage::Finalizing,
        0.95,
        "Writing the verified result",
    ));

    Ok(SpeedTestReport {
        server_url: base.as_str().trim_end_matches('/').to_string(),
        server_name,
        server_observed_client_ip: observed_ip,
        interface_local_ip: config.interface_local_ip,
        download_mbps,
        upload_mbps,
        ping_ms,
        jitter_ms,
        loss_pct,
        download_bytes: received_bytes,
        upload_bytes: config.upload_bytes,
        ping_samples: config.ping_samples,
        ping_received,
        rtt_samples: rtts,
        duration_ms: started.elapsed().as_millis() as u64,
        started_at,
        completed_at: now_rfc3339(),
    })
}
