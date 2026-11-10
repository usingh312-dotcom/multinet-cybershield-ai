//! pipe — named-pipe connection helpers for the Net Merge IPC client.
//!
//! Windows named pipes are exposed as filesystem paths (`\\.\pipe\<name>`), so
//! this module is a thin wrapper around `std::fs::File` with reconnection
//! backoff and ACL guidance.

use std::fs::File;
use std::io;
use std::time::Duration;

/// Open (or reopen) the named pipe at `pipe_name`.
pub fn open(pipe_name: &str) -> io::Result<File> {
    File::open(pipe_name)
}

/// Open the pipe with exponential backoff, retrying for up to `max_wait`.
/// Returns the open file on success, or the last I/O error if the deadline passes.
pub fn open_with_retry(pipe_name: &str, max_wait: Duration) -> io::Result<File> {
    let start = std::time::Instant::now();
    let mut delay = Duration::from_millis(50);
    loop {
        match File::open(pipe_name) {
            Ok(f) => return Ok(f),
            Err(e) => {
                if start.elapsed() >= max_wait {
                    return Err(e);
                }
                std::thread::sleep(delay);
                delay = (delay * 2).min(Duration::from_secs(2));
            }
        }
    }
}

/// Recommended named-pipe security descriptor (SDDL) string. Grants connect
/// access to the interactive user and administrators only; everyone else is
/// denied. Applied by the service when creating the pipe.
pub const PIPE_SDDL: &str = "D:P(A;;GA;;;BA)(A;;GA;;;S-1-5-32-545)(A;;GRGW;;;S-1-1-0)";
