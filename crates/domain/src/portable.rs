//! Portable project metadata: `.vagrant-studio/project.json`.
//!
//! The file describes a project so it can be imported on another computer.
//! It holds no secrets, absolute paths or runtime identifiers, and it is not
//! a replacement for the Vagrantfile. See `docs/project-format.md`.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::project::EnvironmentType;
use crate::validation::{
    ValidationError, normalize_tags, validate_description, validate_environment_name,
    validate_name, validate_repository_url,
};

pub const SCHEMA_VERSION: u32 = 1;
pub const DIR_NAME: &str = ".vagrant-studio";
pub const FILE_NAME: &str = "project.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableProject {
    pub schema_version: u32,
    pub project: PortableProjectInfo,
    #[serde(default)]
    pub environments: Vec<PortableEnvironment>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableProjectInfo {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableEnvironment {
    pub name: String,
    /// Directory relative to the project root, using `/` separators. `.` is the root.
    #[serde(default = "dot")]
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vagrantfile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_type: Option<EnvironmentType>,
    /// Informational hints only; Vagrant remains the source of truth for machines.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub machines: Vec<PortableMachine>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableMachine {
    pub name: String,
    #[serde(default, rename = "box", skip_serializing_if = "Option::is_none")]
    pub box_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_hint: Option<String>,
}

fn dot() -> String {
    ".".to_owned()
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PortableError {
    #[error("project.json is not valid JSON: {0}")]
    Syntax(String),
    #[error("project.json has no schemaVersion")]
    MissingVersion,
    #[error(
        "project.json uses schema version {0}, which is newer than this version of Vagrant Studio supports ({SCHEMA_VERSION}). Update Vagrant Studio to import it."
    )]
    UnsupportedVersion(u64),
    #[error("project.json does not match schema version {SCHEMA_VERSION}: {0}")]
    Shape(String),
    #[error("project.json: {0}")]
    Invalid(#[from] ValidationError),
}

impl PortableProject {
    /// Parses and validates a `project.json`. Older schema versions would be
    /// migrated here; version 1 is the first.
    pub fn parse(json: &str) -> Result<Self, PortableError> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| PortableError::Syntax(e.to_string()))?;
        let version = value
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
            .ok_or(PortableError::MissingVersion)?;
        if version > u64::from(SCHEMA_VERSION) || version == 0 {
            return Err(PortableError::UnsupportedVersion(version));
        }
        let parsed: Self =
            serde_json::from_value(value).map_err(|e| PortableError::Shape(e.to_string()))?;
        parsed.validated()
    }

    pub fn to_json(&self) -> String {
        let mut json = serde_json::to_string_pretty(self).expect("portable project serializes");
        json.push('\n');
        json
    }

    fn validated(mut self) -> Result<Self, PortableError> {
        self.project.name = validate_name(&self.project.name)?;
        self.project.description = validate_description(&self.project.description)?;
        self.project.repository_url =
            validate_repository_url(self.project.repository_url.as_deref())?;
        self.project.tags = normalize_tags(&self.project.tags)?;

        let mut names = HashSet::new();
        for env in &mut self.environments {
            env.name = validate_environment_name(&env.name)?;
            if !names.insert(env.name.to_lowercase()) {
                return Err(ValidationError::new(
                    "environments",
                    format!("Environment \"{}\" is listed twice.", env.name),
                )
                .into());
            }
            env.path = normalize_relative_path(&env.path).ok_or_else(|| {
                ValidationError::new(
                    "environments",
                    format!(
                        "Environment \"{}\" path must be relative to the project folder.",
                        env.name
                    ),
                )
            })?;
            if let Some(vf) = &env.vagrantfile {
                env.vagrantfile = Some(normalize_relative_path(vf).ok_or_else(|| {
                    ValidationError::new(
                        "environments",
                        format!("Environment \"{}\" Vagrantfile path is invalid.", env.name),
                    )
                })?);
            }
        }
        Ok(self)
    }
}

/// Normalizes a relative path to `/` separators. Returns `None` for absolute
/// paths, drive letters, UNC paths or anything that escapes the root with `..`.
pub fn normalize_relative_path(path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    let unified = path.replace('\\', "/");
    if unified.starts_with('/') || unified.chars().nth(1) == Some(':') {
        return None;
    }
    let mut parts = Vec::new();
    for part in unified.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            p => parts.push(p),
        }
    }
    if parts.is_empty() {
        Some(".".to_owned())
    } else {
        Some(parts.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC_EXAMPLE: &str = r#"{
      "schemaVersion": 1,
      "project": { "name": "Web Platform", "description": "Local multi-server development environment" },
      "environments": [{
        "name": "development",
        "vagrantfile": "Vagrantfile",
        "machines": [
          { "name": "web", "box": "generic/ubuntu2404", "providerHint": "virtualbox" },
          { "name": "db", "box": "generic/ubuntu2404", "providerHint": "virtualbox" }
        ]
      }]
    }"#;

    #[test]
    fn parses_example_and_round_trips() {
        let p = PortableProject::parse(SPEC_EXAMPLE).unwrap();
        assert_eq!(p.project.name, "Web Platform");
        assert_eq!(p.environments[0].path, ".");
        assert_eq!(
            p.environments[0].machines[1].box_name.as_deref(),
            Some("generic/ubuntu2404")
        );
        assert_eq!(PortableProject::parse(&p.to_json()).unwrap(), p);
    }

    #[test]
    fn rejects_unknown_versions_and_shapes() {
        assert_eq!(
            PortableProject::parse(r#"{"project":{"name":"x"}}"#),
            Err(PortableError::MissingVersion)
        );
        assert_eq!(
            PortableProject::parse(r#"{"schemaVersion":2,"project":{"name":"x"}}"#),
            Err(PortableError::UnsupportedVersion(2))
        );
        assert!(matches!(
            PortableProject::parse(r#"{"schemaVersion":1,"project":{"name":"x"},"secret":1}"#),
            Err(PortableError::Shape(_))
        ));
        assert!(matches!(
            PortableProject::parse("{nope"),
            Err(PortableError::Syntax(_))
        ));
    }

    #[test]
    fn rejects_paths_escaping_the_project() {
        for bad in [
            "../other",
            "/etc",
            "C:\\Users",
            "\\\\server\\share",
            "a/../../b",
        ] {
            let json = format!(
                r#"{{"schemaVersion":1,"project":{{"name":"x"}},"environments":[{{"name":"e","path":{}}}]}}"#,
                serde_json::to_string(bad).unwrap()
            );
            assert!(
                matches!(
                    PortableProject::parse(&json),
                    Err(PortableError::Invalid(_))
                ),
                "{bad}"
            );
        }
    }

    #[test]
    fn rejects_duplicate_environment_names_and_credentials() {
        let dup = r#"{"schemaVersion":1,"project":{"name":"x"},"environments":[{"name":"dev"},{"name":"DEV","path":"b"}]}"#;
        assert!(PortableProject::parse(dup).is_err());
        let cred =
            r#"{"schemaVersion":1,"project":{"name":"x","repositoryUrl":"https://u:p@h/r"}}"#;
        assert!(PortableProject::parse(cred).is_err());
    }

    #[test]
    fn normalizes_relative_paths() {
        assert_eq!(
            normalize_relative_path("envs\\dev\\").as_deref(),
            Some("envs/dev")
        );
        assert_eq!(normalize_relative_path("./a/./b").as_deref(), Some("a/b"));
        assert_eq!(normalize_relative_path(".").as_deref(), Some("."));
        assert_eq!(normalize_relative_path("").as_deref(), None);
    }
}
