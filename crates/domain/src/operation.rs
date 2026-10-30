use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ids::{EnvironmentId, OperationId, ProjectId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    CancelRequested,
    Cancelled,
    /// The app stopped (crash or quit) while the operation was running.
    Interrupted,
}

impl OperationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::CancelRequested => "cancel_requested",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancel_requested" => Self::CancelRequested,
            "cancelled" => Self::Cancelled,
            "interrupted" => Self::Interrupted,
            _ => return None,
        })
    }

    pub fn is_finished(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Interrupted
        )
    }
}

/// What an operation does. Stored as text; unknown values from newer
/// versions are kept as `Other` rather than failing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    Status,
    Validate,
    BoxList,
    BoxAdd,
    BoxUpdate,
    BoxRemove,
    Up,
    Halt,
    Reload,
    Suspend,
    Resume,
    Provision,
    Destroy,
    SnapshotSave,
    SnapshotRestore,
    SnapshotDelete,
    Package,
    #[serde(untagged)]
    Other(String),
}

impl OperationType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Status => "status",
            Self::Validate => "validate",
            Self::BoxList => "box_list",
            Self::BoxAdd => "box_add",
            Self::BoxUpdate => "box_update",
            Self::BoxRemove => "box_remove",
            Self::Up => "up",
            Self::Halt => "halt",
            Self::Reload => "reload",
            Self::Suspend => "suspend",
            Self::Resume => "resume",
            Self::Provision => "provision",
            Self::Destroy => "destroy",
            Self::SnapshotSave => "snapshot_save",
            Self::SnapshotRestore => "snapshot_restore",
            Self::SnapshotDelete => "snapshot_delete",
            Self::Package => "package",
            Self::Other(s) => s,
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "status" => Self::Status,
            "validate" => Self::Validate,
            "box_list" => Self::BoxList,
            "box_add" => Self::BoxAdd,
            "box_update" => Self::BoxUpdate,
            "box_remove" => Self::BoxRemove,
            "up" => Self::Up,
            "halt" => Self::Halt,
            "reload" => Self::Reload,
            "suspend" => Self::Suspend,
            "resume" => Self::Resume,
            "provision" => Self::Provision,
            "destroy" => Self::Destroy,
            "snapshot_save" => Self::SnapshotSave,
            "snapshot_restore" => Self::SnapshotRestore,
            "snapshot_delete" => Self::SnapshotDelete,
            "package" => Self::Package,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// A recorded Vagrant invocation. Logs live in files; this is the index row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: OperationId,
    pub project_id: Option<ProjectId>,
    pub environment_id: Option<EnvironmentId>,
    pub machine_name: Option<String>,
    pub target_label: String,
    pub operation_type: OperationType,
    pub status: OperationStatus,
    /// Display form of the argv, e.g. `vagrant status --machine-readable`.
    pub command_line: String,
    pub working_dir: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub started_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub finished_at: Option<OffsetDateTime>,
    pub exit_code: Option<i32>,
    pub log_path: Option<String>,
    pub error_summary: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewOperation {
    pub project_id: Option<ProjectId>,
    pub environment_id: Option<EnvironmentId>,
    /// Set when the operation names exactly one machine.
    pub machine_name: Option<String>,
    /// Every machine the operation names; empty for environment-wide
    /// commands such as `status` without a machine.
    pub machines: Vec<String>,
    pub target_label: String,
    pub operation_type: OperationType,
    pub command_line: String,
    pub working_dir: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_round_trip_and_know_when_finished() {
        for s in [
            OperationStatus::Queued,
            OperationStatus::Running,
            OperationStatus::Succeeded,
            OperationStatus::Failed,
            OperationStatus::CancelRequested,
            OperationStatus::Cancelled,
            OperationStatus::Interrupted,
        ] {
            assert_eq!(OperationStatus::parse(s.as_str()), Some(s));
            assert_eq!(
                serde_json::to_string(&s).unwrap(),
                format!("\"{}\"", s.as_str())
            );
        }
        assert!(OperationStatus::Interrupted.is_finished());
        assert!(!OperationStatus::CancelRequested.is_finished());
    }

    #[test]
    fn unknown_types_are_preserved() {
        assert_eq!(OperationType::parse("box_list"), OperationType::BoxList);
        assert_eq!(OperationType::parse("box_remove"), OperationType::BoxRemove);
        let future = OperationType::parse("snapshot_save");
        assert_eq!(future.as_str(), "snapshot_save");
        assert_eq!(serde_json::to_string(&future).unwrap(), "\"snapshot_save\"");
        assert_eq!(serde_json::to_string(&OperationType::Up).unwrap(), "\"up\"");
    }
}
