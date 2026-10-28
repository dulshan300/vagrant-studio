use serde::Serialize;
use vagrant_studio_project_service::ServiceError;

/// Error shape returned to the frontend by every command.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    /// validation | notFound | conflict | confirmationRequired | unsafe | io | unavailable | internal
    pub kind: &'static str,
    pub message: String,
    /// Input field the error refers to (validation errors only).
    pub field: Option<&'static str>,
    /// Current content when confirmation is required (e.g. an existing project.json).
    pub existing: Option<String>,
}

impl CommandError {
    fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            field: None,
            existing: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new("validation", message)
    }

    /// A prerequisite (e.g. Vagrant itself) is missing.
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new("unavailable", message)
    }
}

impl From<ServiceError> for CommandError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::Validation(v) => Self {
                field: Some(v.field),
                ..Self::new("validation", v.message)
            },
            ServiceError::NotFound(m) => Self::new("notFound", m),
            ServiceError::Conflict(m) => Self::new("conflict", m),
            ServiceError::ConfirmationRequired { message, existing } => Self {
                existing,
                ..Self::new("confirmationRequired", message)
            },
            ServiceError::Unsafe(m) => Self::new("unsafe", m),
            ServiceError::Portable(e) => Self::new("validation", e.to_string()),
            err @ ServiceError::Io { .. } => {
                tracing::warn!(error = %err, "file operation failed");
                Self::new("io", err.to_string())
            }
            ServiceError::Store(e) => {
                tracing::error!(error = %e, "database error");
                Self::internal("A database error occurred. See the application log for details.")
            }
        }
    }
}

pub type CommandResult<T> = Result<T, CommandError>;

impl From<vagrant_studio_operation_engine::EngineError> for CommandError {
    fn from(err: vagrant_studio_operation_engine::EngineError) -> Self {
        use vagrant_studio_operation_engine::EngineError;
        match err {
            EngineError::Conflict(m) => Self::new("conflict", m),
            EngineError::Confirmation(m) => Self::new("confirmationRequired", m),
            EngineError::Invalid(m) => Self::validation(m),
            err => {
                tracing::error!(error = %err, "operation engine error");
                Self::internal(err.to_string())
            }
        }
    }
}
