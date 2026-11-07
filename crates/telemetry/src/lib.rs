use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("invalid event: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredEvent {
    pub source: String,
    pub kind: String,
    pub payload: serde_json::Value,
}

pub fn validate_event(event: &StructuredEvent) -> Result<(), TelemetryError> {
    if event.source.trim().is_empty() || event.kind.trim().is_empty() {
        return Err(TelemetryError::Invalid(
            "source and kind must be non-empty".into(),
        ));
    }
    Ok(())
}
