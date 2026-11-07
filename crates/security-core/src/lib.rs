use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("collection unavailable: {0}")]
    Collection(String),
    #[error("not implemented in foundation phase: {0}")]
    NotImplemented(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityEvent {
    pub id: Uuid,
    pub source: String,
    pub severity: String,
    pub summary: String,
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityFinding {
    pub id: Uuid,
    pub title: String,
    pub severity: String,
    pub evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityStatus {
    pub initialized: bool,
    pub monitoring_enabled: bool,
    pub open_findings: usize,
}

#[async_trait]
pub trait SecurityEngine: Send + Sync {
    async fn initialize(&self) -> Result<(), SecurityError>;
    async fn collect_events(&self) -> Result<Vec<SecurityEvent>, SecurityError>;
    async fn get_security_status(&self) -> Result<SecurityStatus, SecurityError>;
    async fn get_findings(&self) -> Result<Vec<SecurityFinding>, SecurityError>;
}

pub struct MockSecurityEngine;

#[async_trait]
impl SecurityEngine for MockSecurityEngine {
    async fn initialize(&self) -> Result<(), SecurityError> {
        Ok(())
    }

    async fn collect_events(&self) -> Result<Vec<SecurityEvent>, SecurityError> {
        Ok(vec![])
    }

    async fn get_security_status(&self) -> Result<SecurityStatus, SecurityError> {
        Ok(SecurityStatus {
            initialized: true,
            monitoring_enabled: false,
            open_findings: 0,
        })
    }

    async fn get_findings(&self) -> Result<Vec<SecurityFinding>, SecurityError> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_engine_initializes() {
        let e = MockSecurityEngine;
        e.initialize().await.unwrap();
        let s = e.get_security_status().await.unwrap();
        assert!(s.initialized);
        assert!(!s.monitoring_enabled);
    }
}
