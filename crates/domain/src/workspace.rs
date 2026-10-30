//! Notes and tasks kept per project in the app database. They never touch
//! the project folder.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{NoteId, ProjectId, TaskId, ValidationError};

const MAX_TITLE: usize = 200;
const MAX_NOTE_BODY: usize = 100_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectNote {
    pub id: NoteId,
    pub project_id: ProjectId,
    pub title: String,
    /// Plain text; the UI never renders it as HTML.
    pub body: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTask {
    pub id: TaskId,
    pub project_id: ProjectId,
    pub title: String,
    pub done: bool,
    /// Position in the project's list, from 0.
    pub sort_order: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,
}

/// A trimmed, single-line title of 1–200 characters.
pub fn validate_item_title(title: &str) -> Result<String, ValidationError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(ValidationError::new("title", "Title is required."));
    }
    if title.chars().count() > MAX_TITLE {
        return Err(ValidationError::new(
            "title",
            format!("Use at most {MAX_TITLE} characters."),
        ));
    }
    if title.chars().any(char::is_control) {
        return Err(ValidationError::new(
            "title",
            "The title must be a single line.",
        ));
    }
    Ok(title.to_owned())
}

/// Note text, kept as written apart from trailing whitespace.
pub fn validate_note_body(body: &str) -> Result<String, ValidationError> {
    let body = body.trim_end();
    if body.chars().count() > MAX_NOTE_BODY {
        return Err(ValidationError::new(
            "body",
            format!("Use at most {MAX_NOTE_BODY} characters."),
        ));
    }
    Ok(body.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_are_trimmed_single_lines() {
        assert_eq!(
            validate_item_title("  Check backups ").unwrap(),
            "Check backups"
        );
        assert_eq!(validate_item_title(" ").unwrap_err().field, "title");
        assert!(validate_item_title("a\nb").is_err());
        assert!(validate_item_title(&"x".repeat(201)).is_err());
        assert!(validate_item_title(&"é".repeat(200)).is_ok());
    }

    #[test]
    fn note_bodies_keep_their_layout() {
        assert_eq!(
            validate_note_body("  indented\n\nline \n\n").unwrap(),
            "  indented\n\nline"
        );
        assert!(validate_note_body(&"x".repeat(100_001)).is_err());
    }
}
