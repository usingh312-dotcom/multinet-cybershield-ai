//! multiNet speed test service.
//!
//! A standalone HTTPS service that streams a bounded payload for download
//! measurement, acknowledges uploaded bytes, and answers latency probes with the
//! source address it observed. Abuse protection is built in: transfer size is
//! capped, concurrent transfers are capped and every request has a timeout.

use axum::body::Bytes;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tower_http::timeout::TimeoutLayer;

const DEFAULT_MAX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CONCURRENT_TRANSFERS: usize = 2;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone)]
struct AppState {
    server_name: String,
    max_bytes: u64,
    transfers: Arc<Semaphore>,
}

#[derive(Deserialize)]
struct PingParams {
    #[serde(default)]
    nonce: String,
}

#[derive(Deserialize)]
struct DownloadParams {
    bytes: Option<u64>,
}

enum ApiError {
    Invalid(String),
    Busy,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Invalid(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::Busy => (
                StatusCode::TOO_MANY_REQUESTS,
                "too many concurrent transfers".to_string(),
            ),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

fn acquire(state: &AppState) -> Result<OwnedSemaphorePermit, ApiError> {
    state
        .transfers
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::Busy)
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "service": "multinet-speedtest",
        "server": state.server_name,
        "max_transfer_bytes": state.max_bytes,
        "max_concurrent_transfers": MAX_CONCURRENT_TRANSFERS,
    }))
}

async fn ping(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Query(params): Query<PingParams>,
) -> Json<serde_json::Value> {
    Json(json!({
        "server": state.server_name,
        "observed_ip": peer.ip().to_string(),
        "nonce": params.nonce,
        "server_time": time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
    }))
}

async fn download(
    State(state): State<AppState>,
    Query(params): Query<DownloadParams>,
) -> Result<Response, ApiError> {
    let _permit = acquire(&state)?;
    let requested = params.bytes.unwrap_or(1024 * 1024);
    if requested == 0 || requested > state.max_bytes {
        return Err(ApiError::Invalid(format!(
            "bytes must be between 1 and {}",
            state.max_bytes
        )));
    }

    let mut payload = vec![0u8; requested as usize];
    for (index, byte) in payload.iter_mut().enumerate() {
        *byte = (index as u32).wrapping_mul(7_919) as u8;
    }

    Ok((
        [(header::CONTENT_TYPE, "application/octet-stream")],
        payload,
    )
        .into_response())
}

async fn upload(State(state): State<AppState>, body: Bytes) -> Result<Json<serde_json::Value>, ApiError> {
    let _permit = acquire(&state)?;
    if body.is_empty() {
        return Err(ApiError::Invalid("empty upload body".into()));
    }
    if body.len() as u64 > state.max_bytes {
        return Err(ApiError::Invalid(format!(
            "upload exceeds {} bytes",
            state.max_bytes
        )));
    }
    Ok(Json(json!({
        "received": body.len() as u64,
        "server": state.server_name,
    })))
}

type ServerConfigTuple = (SocketAddr, Option<String>, Option<String>, u64, String);

fn parse_args() -> Result<ServerConfigTuple, String> {
    let mut bind = "127.0.0.1:9443".to_string();
    let mut cert = None;
    let mut key = None;
    let mut max_bytes = DEFAULT_MAX_BYTES;
    let mut name = "MultiNet Local Speed Test".to_string();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bind" => bind = args.next().ok_or("--bind needs a value")?,
            "--cert" => cert = Some(args.next().ok_or("--cert needs a value")?),
            "--key" => key = Some(args.next().ok_or("--key needs a value")?),
            "--max-bytes" => {
                max_bytes = args
                    .next()
                    .ok_or("--max-bytes needs a value")?
                    .parse::<u64>()
                    .map_err(|e| format!("invalid --max-bytes: {e}"))?;
            }
            "--name" => name = args.next().ok_or("--name needs a value")?,
            "--help" | "-h" => {
                println!(
                    "speedtest-server [--bind ADDR] [--cert PEM] [--key PEM] [--max-bytes N] [--name NAME]"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }

    if max_bytes == 0 || max_bytes > DEFAULT_MAX_BYTES {
        return Err(format!(
            "--max-bytes must be between 1 and {DEFAULT_MAX_BYTES}"
        ));
    }

    let bind = bind.parse::<SocketAddr>().map_err(|e| format!("invalid --bind: {e}"))?;
    Ok((bind, cert, key, max_bytes, name))
}

fn load_tls(
    cert: Option<String>,
    key: Option<String>,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    match (cert, key) {
        (Some(cert_path), Some(key_path)) => {
            let cert_pem = std::fs::read(&cert_path)
                .map_err(|e| format!("reading certificate {cert_path}: {e}"))?;
            let key_pem =
                std::fs::read(&key_path).map_err(|e| format!("reading key {key_path}: {e}"))?;
            Ok((cert_pem, key_pem))
        }
        (None, None) => {
            let names = vec![
                "localhost".to_string(),
                "127.0.0.1".to_string(),
                "::1".to_string(),
            ];
            let generated = rcgen::generate_simple_self_signed(names)
                .map_err(|e| format!("generating self-signed certificate: {e}"))?;
            let cert_pem = generated.cert.pem();
            let key_pem = generated.key_pair.serialize_pem();
            eprintln!(
                "speedtest-server: using an ephemeral self-signed certificate (loopback clients only)"
            );
            Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
        }
        _ => Err("--cert and --key must be provided together".into()),
    }
}

async fn run() -> Result<(), String> {
    // This build uses the ring provider for rustls; axum-server's TLS acceptor
    // reads the process default provider.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let (bind, cert, key, max_bytes, name) = parse_args()?;
    let (cert_pem, key_pem) = load_tls(cert, key)?;

    let state = AppState {
        server_name: name.clone(),
        max_bytes,
        transfers: Arc::new(Semaphore::new(MAX_CONCURRENT_TRANSFERS)),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/ping", get(ping))
        .route("/download", get(download))
        .route("/upload", post(upload))
        .layer(DefaultBodyLimit::max(max_bytes as usize))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ))
        .with_state(state);

    let tls = axum_server::tls_rustls::RustlsConfig::from_pem(cert_pem, key_pem)
        .await
        .map_err(|e| format!("loading TLS material: {e}"))?;

    eprintln!("speedtest-server: {name} listening on https://{bind} (max {max_bytes} bytes)");
    axum_server::bind_rustls(bind, tls)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .map_err(|e| format!("server error: {e}"))
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("speedtest-server: {error}");
        std::process::exit(1);
    }
}
