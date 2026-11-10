//! Bonded socket: connects the scheduler to independent per-interface UDP
//! sockets, applies authenticated framing, and records per-path bytes.
//!
//! Each path is bound to its own UDP socket bound to that interface's local
//! address, so packets belonging to one logical flow can be spread across
//! multiple physical interfaces.

use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use tunnel_protocol::{encode_frame, FramingLayer};

use crate::path_manager::PathStats;
use crate::scheduler::{ScheduledPacket, WeightedPacketScheduler};

#[derive(Debug, thiserror::Error)]
pub enum SocketError {
    #[error("no schedulable path for {0} byte packet")]
    NoPath(usize),
    #[error("no socket bound to path {0}")]
    NoSocket(String),
    #[error("send failed: {0}")]
    Send(std::io::Error),
    #[error("framing error: {0}")]
    Framing(String),
}

/// One bonded path: a schedulable entry bound to its own UDP socket.
struct PathSocket {
    #[allow(dead_code)]
    path_id: String,
    socket: UdpSocket,
    remote: SocketAddr,
}

/// Sends framed packets over independent per-interface UDP sockets, choosing
/// the path via the weighted deficit scheduler and recording per-path bytes.
pub struct BondedSocket {
    sockets: HashMap<String, PathSocket>,
    scheduler: WeightedPacketScheduler,
    framing: FramingLayer,
}

impl BondedSocket {
    pub fn new(framing_key: [u8; 32]) -> Self {
        Self {
            sockets: HashMap::new(),
            scheduler: WeightedPacketScheduler::new(),
            framing: FramingLayer::new(framing_key),
        }
    }

    /// Register a per-interface UDP socket for `path_id`, bound to `remote`.
    pub fn add_path_socket(
        &mut self,
        path_id: String,
        mtu: usize,
        socket: UdpSocket,
        remote: SocketAddr,
    ) {
        self.scheduler.add_path(path_id.clone(), mtu);
        self.sockets.insert(
            path_id.clone(),
            PathSocket {
                path_id,
                socket,
                remote,
            },
        );
    }

    pub fn remove_path(&mut self, path_id: &str) -> bool {
        self.sockets.remove(path_id).is_some() && self.scheduler.remove_path(path_id)
    }

    pub fn update_metrics(&mut self, stats: &PathStats) {
        self.scheduler.update_metrics(stats);
    }

    /// Send `payload` over the scheduler-selected path. The payload is framed
    /// (sequence number + authentication tag) before transmission, and the
    /// transmitted byte count is recorded against the chosen path.
    pub fn send(&mut self, payload: &[u8]) -> Result<ScheduledPacket, SocketError> {
        let scheduled = self
            .scheduler
            .schedule(payload.len())
            .ok_or(SocketError::NoPath(payload.len()))?;
        let frame = self
            .framing
            .encapsulate(payload.to_vec())
            .map_err(|e| SocketError::Framing(e.to_string()))?;
        let wire = encode_frame(&frame);
        let entry = self
            .sockets
            .get(&scheduled.path_id)
            .ok_or_else(|| SocketError::NoSocket(scheduled.path_id.clone()))?;
        let n = entry
            .socket
            .send_to(&wire, entry.remote)
            .map_err(SocketError::Send)?;
        self.scheduler.record_transmission(&scheduled.path_id, n);
        Ok(scheduled)
    }

    pub fn record_acknowledgement(&mut self, path_id: &str, bytes: usize) {
        self.scheduler.record_acknowledgement(path_id, bytes);
    }

    pub fn stats(&self) -> Vec<crate::scheduler::SchedulerStats> {
        self.scheduler.stats()
    }

    pub fn eligible_count(&self) -> usize {
        self.scheduler.eligible_count()
    }
}
