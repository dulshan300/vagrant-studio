use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ids::{EnvironmentId, ProjectId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Active,
    Archived,
}

impl ProjectStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }
}

/// Application metadata about a project. Files on disk and VMs are not owned
/// by this record: removing it never touches either.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub slug: String,
    pub description: String,
    /// Absolute path of the project folder on this computer.
    pub root_path: String,
    pub repository_url: Option<String>,
    pub status: ProjectStatus,
    pub favorite: bool,
    pub tags: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub archived_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_opened_at: Option<OffsetDateTime>,
    /// Number of environments registered for the project.
    pub environment_count: i64,
}

/// Validated input for inserting a project.
#[derive(Debug, Clone, PartialEq)]
pub struct NewProject {
    pub name: String,
    pub description: String,
    pub root_path: String,
    pub repository_url: Option<String>,
    pub tags: Vec<String>,
}

/// Editable project metadata. `None` leaves a field unchanged.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChanges {
    pub name: Option<String>,
    pub description: Option<String>,
    /// `Some(None)` clears the URL.
    #[serde(default, deserialize_with = "double_option")]
    pub repository_url: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
}

fn double_option<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentType {
    Development,
    Testing,
    Staging,
    Other,
}

impl EnvironmentType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Testing => "testing",
            Self::Staging => "staging",
            Self::Other => "other",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "development" => Some(Self::Development),
            "testing" => Some(Self::Testing),
            "staging" => Some(Self::Staging),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

/// A Vagrant working directory belonging to a project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: EnvironmentId,
    pub project_id: ProjectId,
    pub name: String,
    /// Absolute path of the Vagrant working directory.
    pub root_path: String,
    /// Vagrantfile path relative to `root_path`, when one was detected.
    pub vagrantfile_path: Option<String>,
    pub default_provider: Option<String>,
    pub environment_type: EnvironmentType,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewEnvironment {
    pub name: String,
    pub root_path: String,
    pub vagrantfile_path: Option<String>,
    pub default_provider: Option<String>,
    pub environment_type: EnvironmentType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_changes_distinguishes_missing_and_null_url() {
        let missing: ProjectChanges = serde_json::from_str(r#"{"name":"x"}"#).unwrap();
        assert_eq!(missing.repository_url, None);

        let cleared: ProjectChanges = serde_json::from_str(r#"{"repositoryUrl":null}"#).unwrap();
        assert_eq!(cleared.repository_url, Some(None));

        let set: ProjectChanges =
            serde_json::from_str(r#"{"repositoryUrl":"https://example.com/r.git"}"#).unwrap();
        assert_eq!(
            set.repository_url,
            Some(Some("https://example.com/r.git".into()))
        );
    }

    #[test]
    fn enums_round_trip_through_strings() {
        for s in [ProjectStatus::Active, ProjectStatus::Archived] {
            assert_eq!(ProjectStatus::parse(s.as_str()), Some(s));
        }
        for t in [
            EnvironmentType::Development,
            EnvironmentType::Testing,
            EnvironmentType::Staging,
            EnvironmentType::Other,
        ] {
            assert_eq!(EnvironmentType::parse(t.as_str()), Some(t));
        }
        assert_eq!(ProjectStatus::parse("deleted"), None);
    }
}
