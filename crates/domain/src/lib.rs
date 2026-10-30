//! Vagrant Studio domain model.
//!
//! Plain data types and validation rules shared by persistence, services and
//! the Tauri command layer. Nothing in this crate performs I/O.

mod ids;
mod operation;
pub mod portable;
mod project;
mod validation;
mod workspace;

pub use ids::{EnvironmentId, NoteId, OperationId, ProjectId, TaskId};
pub use operation::{NewOperation, Operation, OperationStatus, OperationType};
pub use project::{
    Environment, EnvironmentType, NewEnvironment, NewProject, Project, ProjectChanges,
    ProjectStatus,
};
pub use validation::{
    ValidationError, normalize_tags, slugify, validate_description, validate_environment_name,
    validate_name, validate_provider_name, validate_repository_url, validate_tag,
};
pub use workspace::{ProjectNote, ProjectTask, validate_item_title, validate_note_body};
