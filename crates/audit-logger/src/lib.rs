use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("audit write failed: {0}")]
    Write(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub id: Uuid,
    pub kind: String,
    pub subject_id: Option<Uuid>,
    pub detail: String,
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

impl AuditEvent {
    pub fn policy_denied(subject_id: Uuid, reason: &str) -> Self {
        Self::new("policy_denied", Some(subject_id), reason)
    }
    pub fn approval_requested(subject_id: Uuid, action: &str) -> Self {
        Self::new("approval_requested", Some(subject_id), action)
    }
    pub fn action_approved(subject_id: Uuid, by: &str) -> Self {
        Self::new("action_approved", Some(subject_id), by)
    }
    pub fn rollback_recorded(subject_id: Uuid) -> Self {
        Self::new("rollback_recorded", Some(subject_id), "rollback recorded")
    }
    fn new(kind: &str, subject_id: Option<Uuid>, detail: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind: kind.to_string(),
            subject_id,
            detail: detail.to_string(),
            at: OffsetDateTime::now_utc(),
        }
    }
}

#[async_trait]
pub trait AuditSink: Send + Sync {
    async fn record(&self, event: AuditEvent) -> Result<(), AuditError>;
}

#[derive(Default, Clone)]
pub struct MemoryAuditSink {
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

impl MemoryAuditSink {
    pub async fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().await.clone()
    }
}

#[async_trait]
impl AuditSink for MemoryAuditSink {
    async fn record(&self, event: AuditEvent) -> Result<(), AuditError> {
        self.events.lock().await.push(event);
        Ok(())
    }
}
