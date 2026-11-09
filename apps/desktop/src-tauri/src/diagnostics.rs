//! Diagnostics backend: real traffic sampling, SQLite history and the real
//! speed test engine. Every value surfaced here comes from the operating system
//! or from bytes that crossed a socket.

use crate::IpcError;
use interface_discovery::classify_if_type;
use network_core::{InterfaceKind, NetworkInterface};
use network_monitor::{RateTracker, InterfaceSample};
use serde::{Deserialize, Serialize};
use speed_test::{SpeedTestConfig, SpeedTestProgress, SpeedTestReport, SpeedTestStage};
use sqlx::Row;
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const AGGREGATE_INTERFACE_ID: &str = "*";
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const TRAFFIC_RETENTION_HOURS: i64 = 24;
const SPEED_RESULT_RETENTION: i64 = 200;
const PRUNE_EVERY_TICKS: u64 = 300;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn kind_label(interface: Option<&NetworkInterface>, if_type: u32) -> String {
    let kind = interface.map(|i| i.kind).unwrap_or_else(|| classify_if_type(if_type));
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "other".into())
}

fn is_loopback(interface: Option<&NetworkInterface>, sample: &InterfaceSample) -> bool {
    if let Some(interface) = interface {
        return interface.kind == InterfaceKind::Loopback;
    }
    let text = format!("{} {}", sample.name, sample.description).to_ascii_lowercase();
    sample.if_type == 24 || text.contains("loopback")
}

fn is_physical(interface: Option<&NetworkInterface>, sample: &InterfaceSample) -> bool {
    match interface {
        Some(interface) => interface.is_physical,
        None => {
            matches!(sample.if_type, 6 | 71)
                && !interface_discovery::looks_virtual(&sample.description, &sample.name)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficInterfaceDto {
    pub interface_id: String,
    pub interface_index: u32,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub is_physical: bool,
    pub is_up: bool,
    pub rx_bps: Option<f64>,
    pub tx_bps: Option<f64>,
    pub session_rx_bytes: u64,
    pub session_tx_bytes: u64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
    pub counter_reset: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficSnapshotDto {
    pub sampled_at: String,
    pub source: String,
    pub monitor_active: bool,
    pub interfaces: Vec<TrafficInterfaceDto>,
    pub physical_rx_bps: Option<f64>,
    pub physical_tx_bps: Option<f64>,
    pub session_rx_bytes: u64,
    pub session_tx_bytes: u64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
    pub peak_combined_bps: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficSampleDto {
    pub interface_id: String,
    pub interface_index: u32,
    pub rx_rate_bps: Option<f64>,
    pub tx_rate_bps: Option<f64>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub counter_reset: bool,
    pub sampled_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficMonitorStatusDto {
    pub running: bool,
    pub last_sample_age_ms: Option<u64>,
    pub source: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub interface_index: u32,
    pub is_up: bool,
    pub is_physical: bool,
    pub ipv4_addresses: Vec<String>,
    pub ipv6_addresses: Vec<String>,
    pub mac_address: Option<String>,
    pub mtu: Option<u32>,
    pub link_speed_bps: Option<u64>,
}

pub struct DiagnosticsState {
    running: AtomicBool,
    latest: Mutex<Option<TrafficSnapshotDto>>,
    last_sample: Mutex<Option<Instant>>,
    speed: SpeedTestStore,
}

impl DiagnosticsState {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(true),
            latest: Mutex::new(None),
            last_sample: Mutex::new(None),
            speed: SpeedTestStore::default(),
        }
    }
}

impl Default for DiagnosticsState {
    fn default() -> Self {
        Self::new()
    }
}

/// Start the always-on traffic sampler. It owns its rate tracker and writes one
/// row per live interface plus an aggregate row every second.
pub fn spawn_traffic_monitor(state: Arc<DiagnosticsState>, pool: sqlx::SqlitePool) {
    tauri::async_runtime::spawn(async move {
        let mut tracker = RateTracker::new();
        let mut session: HashMap<u32, (u64, u64)> = HashMap::new();
        let mut previous_tick = Instant::now();
        let mut ticks: u64 = 0;

        loop {
            tokio::time::sleep(SAMPLE_INTERVAL).await;
            if !state.running.load(Ordering::Relaxed) {
                continue;
            }

            let elapsed = previous_tick.elapsed();
            previous_tick = Instant::now();
            if elapsed < Duration::from_millis(200) {
                continue;
            }

            let counters = match network_monitor::snapshot_counters() {
                Ok(counters) => counters,
                Err(error) => {
                    tracing::warn!(%error, "interface counter snapshot failed");
                    continue;
                }
            };
            let discovered = interface_discovery::discover_interfaces().unwrap_or_default();
            let by_index: HashMap<u32, &NetworkInterface> =
                discovered.iter().map(|i| (i.interface_index, i)).collect();

            let samples = tracker.update(&counters, elapsed);
            let sampled_at = now_rfc3339();
            let mut interfaces = Vec::new();
            let mut aggregate_rx: Option<f64> = None;
            let mut aggregate_tx: Option<f64> = None;
            let mut session_rx = 0u64;
            let mut session_tx = 0u64;
            let mut total_rx = 0u64;
            let mut total_tx = 0u64;
            let mut rows: Vec<TrafficSampleDto> = Vec::new();

            for sample in &samples {
                let interface = by_index.get(&sample.interface_index).copied();
                if is_loopback(interface, sample) {
                    continue;
                }
                let physical = is_physical(interface, sample);

                let entry = session.entry(sample.interface_index).or_insert((0, 0));
                if sample.counter_reset {
                    *entry = (sample.in_octets, sample.out_octets);
                } else {
                    entry.0 = entry.0.saturating_add(sample.rx_delta_bytes);
                    entry.1 = entry.1.saturating_add(sample.tx_delta_bytes);
                }
                let interface_session = *entry;

                if physical {
                    if let Some(rate) = sample.rx_bps {
                        aggregate_rx = Some(aggregate_rx.unwrap_or(0.0) + rate);
                    }
                    if let Some(rate) = sample.tx_bps {
                        aggregate_tx = Some(aggregate_tx.unwrap_or(0.0) + rate);
                    }
                    session_rx = session_rx.saturating_add(interface_session.0);
                    session_tx = session_tx.saturating_add(interface_session.1);
                    total_rx = total_rx.saturating_add(sample.in_octets);
                    total_tx = total_tx.saturating_add(sample.out_octets);
                }

                let interface_id = interface
                    .map(|i| i.id.clone())
                    .unwrap_or_else(|| format!("if-{}", sample.interface_index));

                if sample.rx_bps.is_some() || sample.tx_bps.is_some() {
                    rows.push(TrafficSampleDto {
                        interface_id: interface_id.clone(),
                        interface_index: sample.interface_index,
                        rx_rate_bps: sample.rx_bps,
                        tx_rate_bps: sample.tx_bps,
                        rx_bytes: sample.in_octets,
                        tx_bytes: sample.out_octets,
                        counter_reset: sample.counter_reset,
                        sampled_at: sampled_at.clone(),
                    });
                }

                interfaces.push(TrafficInterfaceDto {
                    interface_id,
                    interface_index: sample.interface_index,
                    name: sample.name.clone(),
                    description: sample.description.clone(),
                    kind: kind_label(interface, sample.if_type),
                    is_physical: physical,
                    is_up: sample.is_up,
                    rx_bps: sample.rx_bps,
                    tx_bps: sample.tx_bps,
                    session_rx_bytes: interface_session.0,
                    session_tx_bytes: interface_session.1,
                    total_rx_bytes: sample.in_octets,
                    total_tx_bytes: sample.out_octets,
                    counter_reset: sample.counter_reset,
                });
            }

            interfaces.sort_by(|a, b| a.interface_index.cmp(&b.interface_index));

            let snapshot = TrafficSnapshotDto {
                sampled_at: sampled_at.clone(),
                source: "GetIfTable2".into(),
                monitor_active: state.running.load(Ordering::Relaxed),
                interfaces,
                physical_rx_bps: aggregate_rx,
                physical_tx_bps: aggregate_tx,
                session_rx_bytes: session_rx,
                session_tx_bytes: session_tx,
                total_rx_bytes: total_rx,
                total_tx_bytes: total_tx,
                peak_combined_bps: 0.0,
            };

            if aggregate_rx.is_some() || aggregate_tx.is_some() {
                rows.push(TrafficSampleDto {
                    interface_id: AGGREGATE_INTERFACE_ID.into(),
                    interface_index: 0,
                    rx_rate_bps: aggregate_rx,
                    tx_rate_bps: aggregate_tx,
                    rx_bytes: total_rx,
                    tx_bytes: total_tx,
                    counter_reset: false,
                    sampled_at: sampled_at.clone(),
                });
            }

            if !rows.is_empty() {
                let mut transaction = match pool.begin().await {
                    Ok(transaction) => transaction,
                    Err(error) => {
                        tracing::warn!(%error, "traffic history transaction failed");
                        *lock(&state.latest) = Some(snapshot);
                        *lock(&state.last_sample) = Some(Instant::now());
                        continue;
                    }
                };
                let mut write_failed = false;
                for row in &rows {
                    let result = sqlx::query(
                        "INSERT INTO traffic_samples \
                         (interface_id, interface_index, rx_bytes, tx_bytes, rx_rate_bps, tx_rate_bps, counter_reset, sampled_at, source) \
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'GetIfTable2')",
                    )
                    .bind(&row.interface_id)
                    .bind(row.interface_index)
                    .bind(row.rx_bytes as i64)
                    .bind(row.tx_bytes as i64)
                    .bind(row.rx_rate_bps)
                    .bind(row.tx_rate_bps)
                    .bind(row.counter_reset as i64)
                    .bind(&row.sampled_at)
                    .execute(&mut *transaction)
                    .await;
                    if let Err(error) = result {
                        tracing::warn!(%error, "traffic history insert failed");
                        write_failed = true;
                        break;
                    }
                }
                if write_failed || transaction.commit().await.is_err() {
                    tracing::warn!("traffic history batch was not committed");
                }
            }

            *lock(&state.latest) = Some(snapshot);
            *lock(&state.last_sample) = Some(Instant::now());

            ticks = ticks.wrapping_add(1);
            if ticks % PRUNE_EVERY_TICKS == 0 {
                let cutoff = OffsetDateTime::now_utc() - time::Duration::hours(TRAFFIC_RETENTION_HOURS);
                if let Ok(cutoff) = cutoff.format(&Rfc3339) {
                    let _ = sqlx::query("DELETE FROM traffic_samples WHERE sampled_at < $1")
                        .bind(cutoff)
                        .execute(&pool)
                        .await;
                }
                let _ = sqlx::query(
                    "DELETE FROM speed_test_results WHERE id NOT IN (\
                     SELECT id FROM speed_test_results ORDER BY completed_at DESC LIMIT $1)",
                )
                .bind(SPEED_RESULT_RETENTION)
                .execute(&pool)
                .await;
            }
        }
    });
}

#[tauri::command]
pub fn get_traffic_monitor_status(state: tauri::State<'_, DiagnosticsState>) -> TrafficMonitorStatusDto {
    TrafficMonitorStatusDto {
        running: state.running.load(Ordering::Relaxed),
        last_sample_age_ms: lock(&state.last_sample).map(|t| t.elapsed().as_millis() as u64),
        source: "GetIfTable2",
    }
}

#[tauri::command]
pub fn set_traffic_monitor_running(
    state: tauri::State<'_, DiagnosticsState>,
    running: bool,
) -> TrafficMonitorStatusDto {
    state.running.store(running, Ordering::Relaxed);
    TrafficMonitorStatusDto {
        running,
        last_sample_age_ms: lock(&state.last_sample).map(|t| t.elapsed().as_millis() as u64),
        source: "GetIfTable2",
    }
}

#[tauri::command]
pub async fn get_traffic_snapshot(
    state: tauri::State<'_, DiagnosticsState>,
    pool: tauri::State<'_, crate::db::DatabaseState>,
) -> Result<TrafficSnapshotDto, IpcError> {
    let mut snapshot = (*lock(&state.latest)).clone().unwrap_or_else(|| TrafficSnapshotDto {
        sampled_at: now_rfc3339(),
        source: "GetIfTable2".into(),
        monitor_active: state.running.load(Ordering::Relaxed),
        interfaces: Vec::new(),
        physical_rx_bps: None,
        physical_tx_bps: None,
        session_rx_bytes: 0,
        session_tx_bytes: 0,
        total_rx_bytes: 0,
        total_tx_bytes: 0,
        peak_combined_bps: 0.0,
    });
    snapshot.monitor_active = state.running.load(Ordering::Relaxed);

    let cutoff = (OffsetDateTime::now_utc() - time::Duration::hours(TRAFFIC_RETENTION_HOURS))
        .format(&Rfc3339)
        .map_err(|e| IpcError::internal(format!("timestamp encode failed: {e}")))?;
    let peak = sqlx::query(
        "SELECT COALESCE(MAX(COALESCE(rx_rate_bps, 0) + COALESCE(tx_rate_bps, 0)), 0) AS peak \
         FROM traffic_samples WHERE interface_id = $1 AND sampled_at >= $2",
    )
    .bind(AGGREGATE_INTERFACE_ID)
    .bind(cutoff)
    .fetch_one(&pool.pool)
    .await
    .map_err(|e| IpcError::database_unavailable(e.to_string()))?;
    snapshot.peak_combined_bps = peak
        .try_get::<f64, _>("peak")
        .map_err(|e| IpcError::database_unavailable(e.to_string()))?;

    Ok(snapshot)
}

#[tauri::command]
pub async fn get_traffic_history(
    pool: tauri::State<'_, crate::db::DatabaseState>,
    interface_id: Option<String>,
    seconds: u32,
) -> Result<Vec<TrafficSampleDto>, IpcError> {
    if !(60..=604_800).contains(&seconds) {
        return Err(IpcError::invalid_input(
            "seconds must be between 60 and 604800",
        ));
    }
    let interface_id = interface_id.unwrap_or_else(|| AGGREGATE_INTERFACE_ID.to_string());
    let cutoff = (OffsetDateTime::now_utc() - time::Duration::seconds(seconds as i64))
        .format(&Rfc3339)
        .map_err(|e| IpcError::internal(format!("timestamp encode failed: {e}")))?;
    let bucket = (seconds / 600).max(1);

    let rows = if bucket <= 1 {
        sqlx::query(
            "SELECT interface_id, interface_index, rx_rate_bps, tx_rate_bps, rx_bytes, tx_bytes, \
                    counter_reset, sampled_at \
             FROM traffic_samples WHERE interface_id = $1 AND sampled_at >= $2 \
             ORDER BY sampled_at ASC LIMIT 4000",
        )
        .bind(&interface_id)
        .bind(&cutoff)
        .fetch_all(&pool.pool)
        .await
    } else {
        sqlx::query(
            "SELECT interface_id, 0 AS interface_index, \
                    AVG(rx_rate_bps) AS rx_rate_bps, AVG(tx_rate_bps) AS tx_rate_bps, \
                    MAX(rx_bytes) AS rx_bytes, MAX(tx_bytes) AS tx_bytes, \
                    MAX(counter_reset) AS counter_reset, MIN(sampled_at) AS sampled_at \
             FROM traffic_samples WHERE interface_id = $1 AND sampled_at >= $2 \
             GROUP BY CAST(strftime('%s', sampled_at) AS INTEGER) / $3 \
             ORDER BY sampled_at ASC LIMIT 4000",
        )
        .bind(&interface_id)
        .bind(&cutoff)
        .bind(bucket)
        .fetch_all(&pool.pool)
        .await
    }
    .map_err(|e| IpcError::database_unavailable(e.to_string()))?;

    let mut samples = Vec::with_capacity(rows.len());
    for row in rows.iter() {
        samples.push(TrafficSampleDto {
            interface_id: row.try_get("interface_id").map_err(db_read)?,
            interface_index: row.try_get("interface_index").map_err(db_read)?,
            rx_rate_bps: row.try_get("rx_rate_bps").map_err(db_read)?,
            tx_rate_bps: row.try_get("tx_rate_bps").map_err(db_read)?,
            rx_bytes: row.try_get::<i64, _>("rx_bytes").map_err(db_read)? as u64,
            tx_bytes: row.try_get::<i64, _>("tx_bytes").map_err(db_read)? as u64,
            counter_reset: row.try_get::<i64, _>("counter_reset").map_err(db_read)? != 0,
            sampled_at: row.try_get("sampled_at").map_err(db_read)?,
        });
    }
    Ok(samples)
}

#[tauri::command]
pub fn list_network_interfaces() -> Result<Vec<InterfaceDto>, IpcError> {
    let interfaces = interface_discovery::discover_interfaces()
        .map_err(|e| IpcError::internal(format!("interface discovery failed: {e}")))?;
    Ok(interfaces
        .into_iter()
        .map(|interface| {
            let kind = kind_label(Some(&interface), interface.if_type);
            InterfaceDto {
                id: interface.id,
                name: interface.name,
                description: interface.description,
                kind,
                interface_index: interface.interface_index,
                is_up: interface.is_up,
                is_physical: interface.is_physical,
                ipv4_addresses: interface
                    .addresses
                    .iter()
                    .filter(|a| a.is_ipv4())
                    .map(|a| a.to_string())
                    .collect(),
                ipv6_addresses: interface
                    .addresses
                    .iter()
                    .filter(|a| a.is_ipv6())
                    .map(|a| a.to_string())
                    .collect(),
                mac_address: interface.mac_address,
                mtu: interface.mtu,
                link_speed_bps: interface
                    .transmit_link_speed_bps
                    .max(interface.receive_link_speed_bps),
            }
        })
        .collect())
}

fn db_read(error: sqlx::Error) -> IpcError {
    IpcError {
        code: "database_read_failed".into(),
        message: error.to_string(),
        retryable: true,
    }
}

// ---------------------------------------------------------------------------
// Speed test
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct SpeedTestStore {
    runs: Mutex<HashMap<String, Arc<SpeedTestRun>>>,
    order: Mutex<VecDeque<String>>,
}

#[derive(Debug, Default)]
struct SpeedTestRun {
    cancel: AtomicBool,
    progress: Mutex<Option<SpeedTestProgress>>,
    outcome: Mutex<SpeedTestOutcome>,
}

#[derive(Debug, Default)]
enum SpeedTestOutcome {
    #[default]
    Running,
    Completed(Box<SpeedTestReport>),
    Failed(String),
    Cancelled,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSpeedTestRequest {
    pub server_url: String,
    pub interface_id: Option<String>,
    pub interface_local_ip: Option<String>,
    pub download_bytes: u64,
    pub upload_bytes: u64,
    pub ping_samples: u32,
    pub timeout_secs: u64,
    pub allow_self_signed_loopback: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedTestStatusDto {
    pub id: String,
    pub state: String,
    pub progress: Option<SpeedTestProgress>,
    pub report: Option<SpeedTestReport>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedTestHistoryDto {
    pub id: String,
    pub server_url: String,
    pub server_name: String,
    pub interface_id: Option<String>,
    pub interface_local_ip: Option<String>,
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub ping_ms: f64,
    pub jitter_ms: f64,
    pub loss_pct: f64,
    pub duration_ms: i64,
    pub started_at: String,
    pub completed_at: String,
}

fn validate_speed_request(request: &StartSpeedTestRequest) -> Result<SpeedTestConfig, IpcError> {
    let url = request.server_url.trim();
    if url.len() > 512 || url.contains(char::is_whitespace) || url.contains('@') {
        return Err(IpcError::invalid_input("invalid speed test server URL"));
    }
    let lower = url.to_ascii_lowercase();
    if !lower.starts_with("https://") {
        return Err(IpcError::invalid_input("speed test server must use HTTPS"));
    }
    if request.allow_self_signed_loopback && !url_is_loopback(url) {
        return Err(IpcError::invalid_input(
            "self-signed certificates are only accepted for loopback servers",
        ));
    }
    if !(1..=speed_test::MAX_TRANSFER_BYTES).contains(&request.download_bytes)
        || !(1..=speed_test::MAX_TRANSFER_BYTES).contains(&request.upload_bytes)
    {
        return Err(IpcError::invalid_input(format!(
            "transfer size must be between 1 and {} bytes",
            speed_test::MAX_TRANSFER_BYTES
        )));
    }
    if request.ping_samples == 0 || request.ping_samples > speed_test::MAX_PING_SAMPLES {
        return Err(IpcError::invalid_input("invalid ping sample count"));
    }
    if request.timeout_secs == 0 || request.timeout_secs > 300 {
        return Err(IpcError::invalid_input("timeout must be between 1 and 300 seconds"));
    }
    let interface_local_ip = match &request.interface_local_ip {
        Some(raw) => Some(
            raw.parse::<IpAddr>()
                .map_err(|_| IpcError::invalid_input("invalid interface address"))?,
        ),
        None => None,
    };

    Ok(SpeedTestConfig {
        server_url: url.to_string(),
        interface_local_ip,
        download_bytes: request.download_bytes,
        upload_bytes: request.upload_bytes,
        ping_samples: request.ping_samples,
        timeout_secs: request.timeout_secs,
        allow_self_signed_loopback: request.allow_self_signed_loopback,
    })
}

fn url_is_loopback(url: &str) -> bool {
    let rest = url.trim_start_matches("https://");
    let authority = rest.split('/').next().unwrap_or("");
    let host = if authority.starts_with('[') {
        authority
            .split(']')
            .next()
            .unwrap_or("")
            .trim_start_matches('[')
    } else {
        authority.split(':').next().unwrap_or("")
    };
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false)
}

#[tauri::command]
pub async fn start_speed_test(
    state: tauri::State<'_, DiagnosticsState>,
    pool: tauri::State<'_, crate::db::DatabaseState>,
    request: StartSpeedTestRequest,
) -> Result<String, IpcError> {
    let config = validate_speed_request(&request)?;
    let id = uuid::Uuid::new_v4().to_string();
    let run = Arc::new(SpeedTestRun::default());
    let interface_id = request.interface_id.clone();

    {
        let mut runs = lock(&state.speed.runs);
        runs.insert(id.clone(), Arc::clone(&run));
        let mut order = lock(&state.speed.order);
        order.push_back(id.clone());
        while order.len() > 10 {
            let Some(oldest) = order.pop_front() else {
                break;
            };
            if runs
                .get(&oldest)
                .map(|r| matches!(*lock(&r.outcome), SpeedTestOutcome::Running))
                .unwrap_or(false)
            {
                order.push_back(oldest);
                break;
            }
            runs.remove(&oldest);
        }
    }

    let db = pool.pool.clone();
    let task_run = Arc::clone(&run);
    let task_id = id.clone();
    tauri::async_runtime::spawn(async move {
        let report = speed_test::run_speed_test(&config, &task_run.cancel, |progress| {
            *lock(&task_run.progress) = Some(progress.clone());
        })
        .await;

        match report {
            Ok(report) => {
                let created_at = now_rfc3339();
                let insert = sqlx::query(
                    "INSERT INTO speed_test_results \
                     (id, server_url, server_name, server_observed_client_ip, interface_id, interface_local_ip, \
                      download_mbps, upload_mbps, ping_ms, jitter_ms, loss_pct, download_bytes, upload_bytes, \
                      ping_samples, ping_received, duration_ms, started_at, completed_at, created_at) \
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)",
                )
                .bind(&task_id)
                .bind(&report.server_url)
                .bind(&report.server_name)
                .bind(&report.server_observed_client_ip)
                .bind(&interface_id)
                .bind(report.interface_local_ip.map(|ip| ip.to_string()))
                .bind(report.download_mbps)
                .bind(report.upload_mbps)
                .bind(report.ping_ms)
                .bind(report.jitter_ms)
                .bind(report.loss_pct)
                .bind(report.download_bytes as i64)
                .bind(report.upload_bytes as i64)
                .bind(report.ping_samples as i64)
                .bind(report.ping_received as i64)
                .bind(report.duration_ms as i64)
                .bind(&report.started_at)
                .bind(&report.completed_at)
                .bind(created_at)
                .execute(&db)
                .await;
                if let Err(error) = insert {
                    tracing::warn!(%error, "speed test result could not be persisted");
                }
                *lock(&task_run.outcome) = SpeedTestOutcome::Completed(Box::new(report));
            }
            Err(speed_test::SpeedTestError::Cancelled) => {
                *lock(&task_run.outcome) = SpeedTestOutcome::Cancelled;
            }
            Err(error) => {
                *lock(&task_run.outcome) = SpeedTestOutcome::Failed(error.to_string());
            }
        }
        *lock(&task_run.progress) = Some(SpeedTestProgress {
            stage: match *lock(&task_run.outcome) {
                SpeedTestOutcome::Completed(_) => SpeedTestStage::Complete,
                SpeedTestOutcome::Cancelled => SpeedTestStage::Cancelled,
                SpeedTestOutcome::Failed(_) => SpeedTestStage::Failed,
                SpeedTestOutcome::Running => SpeedTestStage::Finalizing,
            },
            fraction: 1.0,
            current_mbps: None,
            detail: String::new(),
        });
    });

    Ok(id)
}

#[tauri::command]
pub fn get_speed_test_status(
    state: tauri::State<'_, DiagnosticsState>,
    id: String,
) -> Result<SpeedTestStatusDto, IpcError> {
    let run = lock(&state.speed.runs)
        .get(&id)
        .cloned()
        .ok_or_else(|| IpcError::not_found("speed test run not found"))?;
    let progress = (*lock(&run.progress)).clone();
    let outcome = lock(&run.outcome);
    let (state_label, report, error) = match &*outcome {
        SpeedTestOutcome::Running => ("running", None, None),
        SpeedTestOutcome::Completed(report) => ("completed", Some((**report).clone()), None),
        SpeedTestOutcome::Failed(message) => ("failed", None, Some(message.clone())),
        SpeedTestOutcome::Cancelled => ("cancelled", None, None),
    };
    drop(outcome);
    Ok(SpeedTestStatusDto {
        id,
        state: state_label.into(),
        progress,
        report,
        error,
    })
}

#[tauri::command]
pub fn cancel_speed_test(
    state: tauri::State<'_, DiagnosticsState>,
    id: String,
) -> Result<(), IpcError> {
    let run = lock(&state.speed.runs)
        .get(&id)
        .cloned()
        .ok_or_else(|| IpcError::not_found("speed test run not found"))?;
    run.cancel.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn get_speed_test_history(
    pool: tauri::State<'_, crate::db::DatabaseState>,
) -> Result<Vec<SpeedTestHistoryDto>, IpcError> {
    let rows = sqlx::query(
        "SELECT id, server_url, server_name, interface_id, interface_local_ip, download_mbps, \
                upload_mbps, ping_ms, jitter_ms, loss_pct, duration_ms, started_at, completed_at \
         FROM speed_test_results ORDER BY completed_at DESC LIMIT 100",
    )
    .fetch_all(&pool.pool)
    .await
    .map_err(|e| IpcError::database_unavailable(e.to_string()))?;

    let mut results = Vec::with_capacity(rows.len());
    for row in rows.iter() {
        results.push(SpeedTestHistoryDto {
            id: row.try_get("id").map_err(db_read)?,
            server_url: row.try_get("server_url").map_err(db_read)?,
            server_name: row.try_get("server_name").map_err(db_read)?,
            interface_id: row.try_get("interface_id").map_err(db_read)?,
            interface_local_ip: row.try_get("interface_local_ip").map_err(db_read)?,
            download_mbps: row.try_get("download_mbps").map_err(db_read)?,
            upload_mbps: row.try_get("upload_mbps").map_err(db_read)?,
            ping_ms: row.try_get("ping_ms").map_err(db_read)?,
            jitter_ms: row.try_get("jitter_ms").map_err(db_read)?,
            loss_pct: row.try_get("loss_pct").map_err(db_read)?,
            duration_ms: row.try_get("duration_ms").map_err(db_read)?,
            started_at: row.try_get("started_at").map_err(db_read)?,
            completed_at: row.try_get("completed_at").map_err(db_read)?,
        });
    }
    Ok(results)
}
