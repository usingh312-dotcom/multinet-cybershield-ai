use async_trait::async_trait;
use network_core::InterfaceMetrics;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub mod path_manager;
pub mod scheduler;
pub mod socket;

pub use path_manager::{PathConfig, PathHealth, PathId, PathManager, PathStats};
pub use scheduler::{SchedulerStats, ScheduledPacket, WeightedPacketScheduler};
pub use socket::{BondedSocket, SocketError};

#[derive(Debug, Error)]
pub enum BondingError {
    #[error("session not started: {0}")]
    Session(String),
    #[error("not implemented in foundation phase: {0}")]
    NotImplemented(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BondingState {
    Uninitialized,
    Initialized,
    Running,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BondingStatus {
    pub state: BondingState,
    pub session_id: Option<Uuid>,
    pub active_paths: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathMetrics {
    pub path_id: String,
    pub interface_metrics: InterfaceMetrics,
}

#[async_trait]
pub trait BondingEngine: Send + Sync {
    async fn initialize(&self) -> Result<(), BondingError>;
    async fn start_session(&self) -> Result<Uuid, BondingError>;
    async fn stop_session(&self, session_id: Uuid) -> Result<(), BondingError>;
    async fn get_status(&self) -> Result<BondingStatus, BondingError>;
    async fn get_path_metrics(&self) -> Result<Vec<PathMetrics>, BondingError>;
}

pub struct MockBondingEngine;

#[async_trait]
impl BondingEngine for MockBondingEngine {
    async fn initialize(&self) -> Result<(), BondingError> {
        Ok(())
    }
    async fn start_session(&self) -> Result<Uuid, BondingError> {
        Err(BondingError::NotImplemented(
            "session creation requires relay/protocol; disabled in foundation phase".to_string(),
        ))
    }
    async fn stop_session(&self, _session_id: Uuid) -> Result<(), BondingError> {
        Ok(())
    }
    async fn get_status(&self) -> Result<BondingStatus, BondingError> {
        Ok(BondingStatus {
            state: BondingState::Initialized,
            session_id: None,
            active_paths: 0,
        })
    }
    async fn get_path_metrics(&self) -> Result<Vec<PathMetrics>, BondingError> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_bonding_reports_foundation_state() {
        let e = MockBondingEngine;
        e.initialize().await.unwrap();
        let s = e.get_status().await.unwrap();
        assert_eq!(s.state, BondingState::Initialized);
        assert_eq!(s.active_paths, 0);
    }
}
