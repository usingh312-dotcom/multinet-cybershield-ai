use async_trait::async_trait;
use audit_logger::{AuditEvent, AuditSink};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("action denied: {0}")]
    Denied(String),
    #[error("approval required: {0}")]
    ApprovalRequired(String),
    #[error("rollback failed: {0}")]
    Rollback(String),
    #[error("audit failure: {0}")]
    Audit(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionRequest {
    pub id: Uuid,
    pub action: String,
    pub target: String,
    pub reason: String,
    pub requested_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny { reason: String },
    RequiresApproval { request_id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Approval {
    pub request_id: Uuid,
    pub approved_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionResult {
    pub request_id: Uuid,
    pub status: String,
}

#[async_trait]
pub trait SecurityPolicy: Send + Sync {
    async fn validate_action(&self, request: &ActionRequest) -> Result<Decision, PolicyError>;
    async fn request_approval(&self, request: &ActionRequest) -> Result<Uuid, PolicyError>;
    async fn execute_approved_action(
        &self,
        request: &ActionRequest,
        approval: &Approval,
    ) -> Result<ActionResult, PolicyError>;
    async fn rollback_action(&self, request_id: Uuid) -> Result<(), PolicyError>;
}

pub struct HumanApprovalPolicy<A: AuditSink> {
    audit: A,
}

impl<A: AuditSink> HumanApprovalPolicy<A> {
    pub fn new(audit: A) -> Self {
        Self { audit }
    }
}

fn is_defensive_system_action(action: &str) -> bool {
    let a = action.to_ascii_lowercase();
    a.contains("firewall")
        || a.contains("route")
        || a.contains("network")
        || a.contains("dns")
        || a.contains("quarantine")
        || a.contains("block")
        || a.contains("shell")
}

#[async_trait]
impl<A: AuditSink + Send + Sync> SecurityPolicy for HumanApprovalPolicy<A> {
    async fn validate_action(&self, request: &ActionRequest) -> Result<Decision, PolicyError> {
        if request.action.to_ascii_lowercase().contains("shell") {
            let reason = "arbitrary shell execution is prohibited".to_string();
            self.audit
                .record(AuditEvent::policy_denied(request.id, &reason))
                .await
                .map_err(|e| PolicyError::Audit(e.to_string()))?;
            return Ok(Decision::Deny { reason });
        }
        if is_defensive_system_action(&request.action) {
            Ok(Decision::RequiresApproval {
                request_id: request.id,
            })
        } else {
            Ok(Decision::Allow)
        }
    }

    async fn request_approval(&self, request: &ActionRequest) -> Result<Uuid, PolicyError> {
        self.audit
            .record(AuditEvent::approval_requested(request.id, &request.action))
            .await
            .map_err(|e| PolicyError::Audit(e.to_string()))?;
        info!(request_id = %request.id, "approval requested; no automatic execution");
        Ok(request.id)
    }

    async fn execute_approved_action(
        &self,
        request: &ActionRequest,
        approval: &Approval,
    ) -> Result<ActionResult, PolicyError> {
        if approval.request_id != request.id {
            return Err(PolicyError::Denied(
                "approval does not match request".into(),
            ));
        }
        self.audit
            .record(AuditEvent::action_approved(
                request.id,
                &approval.approved_by,
            ))
            .await
            .map_err(|e| PolicyError::Audit(e.to_string()))?;
        Err(PolicyError::Denied(
            "foundation phase: no system/network/firewall mutations executed".into(),
        ))
    }

    async fn rollback_action(&self, request_id: Uuid) -> Result<(), PolicyError> {
        self.audit
            .record(AuditEvent::rollback_recorded(request_id))
            .await
            .map_err(|e| PolicyError::Audit(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audit_logger::MemoryAuditSink;

    #[tokio::test]
    async fn denies_shell() {
        let policy = HumanApprovalPolicy::new(MemoryAuditSink::default());
        let req = ActionRequest {
            id: Uuid::new_v4(),
            action: "run shell command".into(),
            target: "host".into(),
            reason: "test".into(),
            requested_by: "test".into(),
        };
        let d = policy.validate_action(&req).await.unwrap();
        assert!(matches!(d, Decision::Deny { .. }));
    }

    #[tokio::test]
    async fn firewall_requires_approval() {
        let policy = HumanApprovalPolicy::new(MemoryAuditSink::default());
        let req = ActionRequest {
            id: Uuid::new_v4(),
            action: "apply firewall rule".into(),
            target: "local".into(),
            reason: "test".into(),
            requested_by: "test".into(),
        };
        let d = policy.validate_action(&req).await.unwrap();
        assert!(matches!(d, Decision::RequiresApproval { .. }));
    }
}
