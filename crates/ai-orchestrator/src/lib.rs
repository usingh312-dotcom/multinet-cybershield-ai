use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("provider unavailable: {0}")]
    Provider(String),
    #[error("analysis refused by policy: {0}")]
    Policy(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiInput {
    pub kind: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiOutput {
    pub summary: String,
    pub recommendations: Vec<String>,
    pub confidence_note: String,
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn analyze(&self, input: AiInput) -> Result<AiOutput, AiError>;
    async fn explain(&self, input: AiInput) -> Result<AiOutput, AiError>;
    async fn recommend(&self, input: AiInput) -> Result<AiOutput, AiError>;
}

pub struct MockAiProvider;

fn foundation_output(action: &str) -> AiOutput {
    AiOutput {
        summary: format!("{action}: foundation stub; no external model called"),
        recommendations: vec![
            "Collect verified telemetry before conclusions".to_string(),
            "Require human approval before defensive actions".to_string(),
        ],
        confidence_note: "No production analysis performed in foundation phase".to_string(),
    }
}

#[async_trait]
impl AiProvider for MockAiProvider {
    async fn analyze(&self, _input: AiInput) -> Result<AiOutput, AiError> {
        Ok(foundation_output("analyze"))
    }
    async fn explain(&self, _input: AiInput) -> Result<AiOutput, AiError> {
        Ok(foundation_output("explain"))
    }
    async fn recommend(&self, _input: AiInput) -> Result<AiOutput, AiError> {
        Ok(foundation_output("recommend"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_ai_never_claims_production() {
        let p = MockAiProvider;
        let out = p
            .analyze(AiInput {
                kind: "test".into(),
                content: serde_json::json!({}),
            })
            .await
            .unwrap();
        assert!(out.confidence_note.contains("foundation"));
    }
}
