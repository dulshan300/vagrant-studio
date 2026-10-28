use serde::Serialize;
use time::OffsetDateTime;
use vagrant_studio_domain::{EnvironmentId, Operation, OperationId};

/// One event in the protocol described in docs/architecture.md §5.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationEvent {
    /// e.g. `operation.started`, `operation.stdout`, `environment.status_updated`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
    pub operation_id: Option<OperationId>,
    pub environment_id: Option<EnvironmentId>,
    pub machine_name: Option<String>,
    pub payload: serde_json::Value,
}

impl OperationEvent {
    pub fn new(kind: &'static str, op: Option<&Operation>, payload: serde_json::Value) -> Self {
        Self {
            kind,
            timestamp: OffsetDateTime::now_utc(),
            operation_id: op.map(|o| o.id),
            environment_id: op.and_then(|o| o.environment_id),
            machine_name: op.and_then(|o| o.machine_name.clone()),
            payload,
        }
    }
}

/// Where events go: the Tauri bridge in the app, a recorder in tests.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: OperationEvent);
}
