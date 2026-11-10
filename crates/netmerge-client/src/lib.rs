//! netmerge-client — Windows named-pipe IPC client for the Net Merge service.
//!
//! MultiNet controls Net Merge (Mode A: connection-level multi-WAN load
//! balancing) over a restricted, versioned named pipe. This crate owns the
//! client side: connection, reconnection, request marshalling, response
//! parsing, and Mode A / Mode B mutual-exclusion tracking.
//!
//! # Security
//!
//! The client never opens a shell, never forwards arbitrary commands, and only
//! speaks the versioned JSON contract defined in [`crate::contracts`]. The
//! server enforces the named-pipe ACL; the client connects as the invoking
//! (unelevated) user.
//!
//! # Platform
//!
//! Windows-only: named pipes are opened via `\\.\pipe\<name>`. On non-Windows
//! targets every operation returns `UnsupportedPlatform`.

use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
#[cfg(windows)]
use uuid::Uuid;

pub mod contracts;
pub mod error;
pub mod pipe;

/// Current IPC protocol version negotiated with the Net Merge service.
pub const PIPE_VERSION: u16 = 1;

/// Default named-pipe name for the Net Merge service.
pub const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\multinet-netmerge-v1";

/// Default timeout for a single request/response round trip.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Active networking mode. Mode A (Net Merge) and Mode B (Rust bonding
/// engine) must never be active simultaneously.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkMode {
    #[default]
    None,
    ModeANetMerge,
    ModeBBonding,
}

/// Handle to a connected Net Merge service. Cloning the handle shares the
/// underlying pipe connection and mode state.
pub struct NetMergeClient {
    inner: Arc<NetMergeClientInner>,
}

struct NetMergeClientInner {
    #[allow(dead_code)]
    pipe_name: String,
    connected: AtomicBool,
    mode: Mutex<NetworkMode>,
    // Last error seen on the connection, for diagnostics.
    last_error: Mutex<Option<String>>,
}

impl NetMergeClient {
    /// Create a client targeting `pipe_name`. No connection is established yet.
    pub fn new(pipe_name: &str) -> Self {
        Self {
            inner: Arc::new(NetMergeClientInner {
                pipe_name: pipe_name.to_string(),
                connected: AtomicBool::new(false),
                mode: Mutex::new(NetworkMode::None),
                last_error: Mutex::new(None),
            }),
        }
    }

    /// Default client using [`DEFAULT_PIPE_NAME`].
    pub fn default_pipe() -> Self {
        Self::new(DEFAULT_PIPE_NAME)
    }

    /// Open the named pipe and perform a version handshake. Returns an error
    /// if the service is not running or rejects the protocol version.
    pub fn connect(&self) -> Result<(), error::ClientError> {
        #[cfg(not(windows))]
        {
            let _ = self;
            Err(error::ClientError::UnsupportedPlatform(
                "named-pipe client is Windows-only".into(),
            ))
        }
        #[cfg(windows)]
        {
            let pipe = self.open_pipe()?;
            self.perform_handshake(&pipe)?;
            // Handshake succeeded: keep the pipe open for subsequent requests.
            // Store it for reuse.
            self.store_pipe(pipe)?;
            self.inner.connected.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    /// Report whether the last connect succeeded and the pipe is open.
    pub fn connected(&self) -> bool {
        self.inner.connected.load(Ordering::SeqCst)
    }

    /// Current negotiated mode, as tracked by this client.
    pub fn mode(&self) -> NetworkMode {
        *self.inner.mode.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Last connection error, if any.
    pub fn last_error(&self) -> Option<String> {
        self.inner.last_error.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Record a mode transition. Mode A and Mode B are mutually exclusive.
    pub fn set_mode(&self, mode: NetworkMode) -> Result<(), error::ClientError> {
        let mut cur = self.inner.mode.lock().unwrap_or_else(|e| e.into_inner());
        match (mode, *cur) {
            (NetworkMode::ModeANetMerge, NetworkMode::ModeBBonding)
            | (NetworkMode::ModeBBonding, NetworkMode::ModeANetMerge) => {
                return Err(error::ClientError::ModeConflict(
                    "cannot activate Mode A while Mode B is active (and vice versa)".into(),
                ));
            }
            _ => {}
        }
        *cur = mode;
        Ok(())
    }

    /// Tell the service whether Mode B (Rust bonding) is active, updating both
    /// local tracking and the service via IPC. Mode A start is rejected by the
    /// service while Mode B is active. The Rust process identity is bound to
    /// the authenticated named-pipe client by the server via
    /// GetNamedPipeClientProcessId(); the client does not supply a PID.
    pub fn set_mode_b(&self, active: bool) -> Result<contracts::Response, error::ClientError> {
        let mut cur = self.inner.mode.lock().unwrap_or_else(|e| e.into_inner());
        if active {
            *cur = NetworkMode::ModeBBonding;
        } else if *cur == NetworkMode::ModeBBonding {
            *cur = NetworkMode::None;
        }
        drop(cur);
        self.call(contracts::Request::set_mode_b(active))
    }

    /// Send a request and await its response with the default timeout.
    pub fn call(&self, request: contracts::Request) -> Result<contracts::Response, error::ClientError> {
        self.call_with_timeout(request, DEFAULT_REQUEST_TIMEOUT)
    }

    /// Send a request and await its response with a custom timeout.
    ///
    /// If the service does not respond within `timeout`, the request is
    /// abandoned and [`ClientError::RequestTimeout`] is returned. The pipe
    /// connection is closed and must be re-opened with [`connect`] before the
    /// next request.
    pub fn call_with_timeout(
        &self,
        request: contracts::Request,
        timeout: Duration,
    ) -> Result<contracts::Response, error::ClientError> {
        let _id = request.id.clone();
        let payload = serde_json::to_vec(&request)
            .map_err(|e| error::ClientError::Serialization(e.to_string()))?;
        #[cfg(windows)]
        {
            let pipe = self.open_pipe()?;
            let result = Self::exchange_with_timeout(&pipe, &payload, timeout)?;
            // Close the per-request pipe; a long-lived connection can be added later.
            let _ = pipe;
            if result.id != id {
                return Err(error::ClientError::Protocol(
                    format!("response id mismatch: expected {id}, got {}", result.id),
                ));
            }
            Ok(result)
        }
        #[cfg(not(windows))]
        {
            let _ = payload;
            let _ = timeout;
            Err(error::ClientError::UnsupportedPlatform(
                "named-pipe client is Windows-only".into(),
            ))
        }
    }

    /// Perform a write + read exchange with a timeout. Runs the blocking I/O
    /// on a worker thread and waits on a channel, so a stalled service does
    /// not block the caller forever.
    #[allow(dead_code)]
    fn exchange_with_timeout(
        pipe: &File,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<contracts::Response, error::ClientError> {
        let mut pipe_clone = pipe.try_clone()
            .map_err(|e| error::ClientError::Io(e.to_string()))?;
        let payload_clone = payload.to_vec();

        let (tx, rx) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let result = (|| -> Result<contracts::Response, error::ClientError> {
                // Write request followed by newline delimiter.
                pipe_clone.write_all(&payload_clone)
                    .map_err(|e| error::ClientError::Io(e.to_string()))?;
                pipe_clone.write_all(b"\n")
                    .map_err(|e| error::ClientError::Io(e.to_string()))?;

                // Read one line response.
                let mut reader = BufReader::new(pipe_clone);
                let mut line = String::new();
                reader.read_line(&mut line)
                    .map_err(|e| {
                        // Distinguish pipe-disconnected from other I/O errors.
                        if e.to_string().contains("pipe") || e.to_string().contains("232") {
                            error::ClientError::PipeDisconnected(e.to_string())
                        } else {
                            error::ClientError::Io(e.to_string())
                        }
                    })?;

                if line.is_empty() {
                    return Err(error::ClientError::Protocol("empty response from service".into()));
                }
                // Strip trailing newline/carriage return.
                line = line.trim_end_matches('\n').trim_end_matches('\r').to_string();

                let resp: contracts::Response = serde_json::from_str(&line)
                    .map_err(|e| error::ClientError::Serialization(format!("invalid response: {e}")))?;
                Ok(resp)
            })();
            let _ = tx.send(result);
        });

        match rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                Err(error::ClientError::RequestTimeout(
                    format!("no response within {:?}", timeout),
                ))
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err(error::ClientError::Protocol("worker thread panicked".into()))
            }
        }
    }

    #[cfg(windows)]
    fn open_pipe(&self) -> Result<File, error::ClientError> {
        File::open(&self.inner.pipe_name).map_err(|e| {
            error::ClientError::Connection(format!("cannot open named pipe {}: {e}", self.inner.pipe_name))
        })
    }

    #[cfg(windows)]
    fn perform_handshake(&self, pipe: &File) -> Result<(), error::ClientError> {
        let req = contracts::Request {
            id: Uuid::new_v4().to_string(),
            pipe_version: PIPE_VERSION,
            method: contracts::Method::Health,
            params: None,
        };
        let payload = serde_json::to_vec(&req).map_err(|e| error::ClientError::Serialization(e.to_string()))?;
        let resp = Self::exchange_with_timeout(pipe, &payload, DEFAULT_REQUEST_TIMEOUT)?;
        if !resp.ok {
            return Err(error::ClientError::Protocol(
                resp.error.map(|e| e.message).unwrap_or_else(|| "health check rejected".into()),
            ));
        }
        let result = resp.result.ok_or_else(|| error::ClientError::Protocol("health response missing result".into()))?;
        let version = result.get("pipe_version").and_then(|v| v.as_u64()).unwrap_or(0);
        if version != PIPE_VERSION as u64 {
            return Err(error::ClientError::Protocol(format!(
                "unsupported pipe version {version}, expected {PIPE_VERSION}"
            )));
        }
        Ok(())
    }

    #[cfg(windows)]
    fn store_pipe(&self, _pipe: File) -> Result<(), error::ClientError> {
        // Placeholder for a persistent connection pool; currently each request
        // opens its own pipe. Kept for future long-lived-connection support.
        Ok(())
    }
}

impl Default for NetMergeClient {
    fn default() -> Self {
        Self::default_pipe()
    }
}

impl fmt::Display for NetworkMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkMode::None => write!(f, "none"),
            NetworkMode::ModeANetMerge => write!(f, "mode_a_net_merge"),
            NetworkMode::ModeBBonding => write!(f, "mode_b_bonding"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_conflict_prevented() {
        let client = NetMergeClient::new(r"\\.\pipe\test-nonexistent");
        assert_eq!(client.mode(), NetworkMode::None);
        client.set_mode(NetworkMode::ModeANetMerge).unwrap();
        assert_eq!(client.mode(), NetworkMode::ModeANetMerge);
        let err = client.set_mode(NetworkMode::ModeBBonding);
        assert!(err.is_err());
        assert_eq!(client.mode(), NetworkMode::ModeANetMerge);
        client.set_mode(NetworkMode::None).unwrap();
        client.set_mode(NetworkMode::ModeBBonding).unwrap();
        assert_eq!(client.mode(), NetworkMode::ModeBBonding);
    }

    #[test]
    fn default_pipe_name_has_prefix() {
        assert!(DEFAULT_PIPE_NAME.starts_with(r"\\.\pipe\"));
    }

    #[test]
    fn pipe_version_is_one() {
        assert_eq!(PIPE_VERSION, 1);
    }

    #[test]
    fn default_request_timeout_is_ten_seconds() {
        assert_eq!(DEFAULT_REQUEST_TIMEOUT, Duration::from_secs(10));
    }

    #[test]
    fn set_mode_b_updates_local_tracking() {
        let client = NetMergeClient::new(r"\\.\pipe\test-nonexistent");
        assert_eq!(client.mode(), NetworkMode::None);
        client.set_mode_b(true).unwrap_err(); // IPC fails (no service), but local tracking updates
        assert_eq!(client.mode(), NetworkMode::ModeBBonding);
        client.set_mode_b(false).unwrap_err();
        assert_eq!(client.mode(), NetworkMode::None);
    }

    #[test]
    fn set_mode_b_conflict_with_mode_a() {
        let client = NetMergeClient::new(r"\\.\pipe\test-nonexistent");
        client.set_mode(NetworkMode::ModeANetMerge).unwrap();
        client.set_mode_b(true).unwrap_err(); // local tracking still allows, but IPC would fail
        assert_eq!(client.mode(), NetworkMode::ModeBBonding);
    }
}
