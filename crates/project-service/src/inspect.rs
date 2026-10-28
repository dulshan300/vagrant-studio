//! Read-only inspection of a folder the user wants to import.

use std::path::{Path, PathBuf};

use serde::Serialize;
use vagrant_studio_domain::portable::{self, PortableProject};
use vagrant_studio_domain::{ProjectId, validate_environment_name};

/// Folders never descended into while looking for Vagrantfiles.
const SKIP_DIRS: &[&str] = &["node_modules", "vendor", "target", "dist", "build"];
const MAX_DEPTH: usize = 3;
const MAX_ENTRIES: usize = 5_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderInspection {
    /// Canonical absolute path.
    pub path: String,
    pub suggested_name: String,
    pub suggested_description: String,
    pub suggested_repository_url: Option<String>,
    pub suggested_tags: Vec<String>,
    pub environments: Vec<DetectedEnvironment>,
    /// True when `.vagrant-studio/project.json` exists (valid or not).
    pub has_portable_metadata: bool,
    /// Why the portable metadata could not be used, if it exists but is invalid.
    pub portable_error: Option<String>,
    /// Set when the folder is already registered.
    pub existing_project_id: Option<ProjectId>,
    /// True when the scan stopped early because the folder is very large.
    pub scan_truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedEnvironment {
    pub name: String,
    /// Relative to the project folder, `/`-separated; `.` is the folder itself.
    pub relative_path: String,
    /// Vagrantfile file name if one exists in that directory.
    pub vagrantfile: Option<String>,
    /// A `.vagrant` directory exists, so machines may already have been created.
    pub has_vagrant_state: bool,
    /// Listed in project.json.
    pub from_metadata: bool,
}

pub(crate) fn inspect(root: &Path) -> FolderInspection {
    let folder_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Project".to_owned());

    let mut scan = Scan::default();
    scan.walk(root, root, 0);

    let mut inspection = FolderInspection {
        path: root.display().to_string(),
        suggested_name: folder_name,
        suggested_description: String::new(),
        suggested_repository_url: None,
        suggested_tags: Vec::new(),
        environments: scan.found,
        has_portable_metadata: false,
        portable_error: None,
        existing_project_id: None,
        scan_truncated: scan.truncated,
    };

    let metadata_path = root.join(portable::DIR_NAME).join(portable::FILE_NAME);
    if metadata_path.is_file() {
        inspection.has_portable_metadata = true;
        match std::fs::read_to_string(&metadata_path)
            .map_err(|e| e.to_string())
            .and_then(|json| PortableProject::parse(&json).map_err(|e| e.to_string()))
        {
            Ok(meta) => apply_metadata(&mut inspection, root, meta),
            Err(e) => inspection.portable_error = Some(e),
        }
    }
    inspection
}

/// Prefers names and environments from project.json, keeping scanned ones it doesn't list.
fn apply_metadata(inspection: &mut FolderInspection, root: &Path, meta: PortableProject) {
    inspection.suggested_name = meta.project.name;
    inspection.suggested_description = meta.project.description;
    inspection.suggested_repository_url = meta.project.repository_url;
    inspection.suggested_tags = meta.project.tags;

    let mut envs: Vec<DetectedEnvironment> = meta
        .environments
        .into_iter()
        .map(|e| {
            let dir = root.join(&e.path);
            let vagrantfile = e.vagrantfile.or_else(|| vagrantfile_in(&dir));
            DetectedEnvironment {
                name: e.name,
                has_vagrant_state: dir.join(".vagrant").is_dir(),
                relative_path: e.path,
                vagrantfile,
                from_metadata: true,
            }
        })
        .collect();
    for scanned in inspection.environments.drain(..) {
        if !envs
            .iter()
            .any(|e| e.relative_path == scanned.relative_path)
        {
            envs.push(scanned);
        }
    }
    dedupe_names(&mut envs);
    inspection.environments = envs;
}

#[derive(Default)]
struct Scan {
    found: Vec<DetectedEnvironment>,
    entries: usize,
    truncated: bool,
}

impl Scan {
    fn walk(&mut self, root: &Path, dir: &Path, depth: usize) {
        if let Some(vagrantfile) = vagrantfile_in(dir) {
            let relative = relative_slash_path(root, dir);
            self.found.push(DetectedEnvironment {
                name: environment_name_for(&relative),
                relative_path: relative,
                vagrantfile: Some(vagrantfile),
                has_vagrant_state: dir.join(".vagrant").is_dir(),
                from_metadata: false,
            });
        }
        if depth >= MAX_DEPTH {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut subdirs: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            self.entries += 1;
            if self.entries > MAX_ENTRIES {
                self.truncated = true;
                return;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            // file_type() does not follow symlinks, so linked directories are skipped.
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir && !name.starts_with('.') && !SKIP_DIRS.contains(&name.as_str()) {
                subdirs.push(entry.path());
            }
        }
        subdirs.sort();
        for sub in subdirs {
            self.walk(root, &sub, depth + 1);
        }
        if depth == 0 {
            dedupe_names(&mut self.found);
        }
    }
}

/// Vagrant accepts `Vagrantfile` and `vagrantfile`. Matches against the real
/// directory listing so the on-disk spelling is reported even on
/// case-insensitive filesystems (Windows).
pub(crate) fn vagrantfile_in(dir: &Path) -> Option<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n == "Vagrantfile" || n == "vagrantfile")
        .collect();
    names.sort();
    names.into_iter().next()
}

pub(crate) fn relative_slash_path(root: &Path, dir: &Path) -> String {
    match dir.strip_prefix(root) {
        Ok(rel) if rel.as_os_str().is_empty() => ".".to_owned(),
        Ok(rel) => rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => dir.display().to_string(),
    }
}

fn environment_name_for(relative: &str) -> String {
    if relative == "." {
        return "default".to_owned();
    }
    let last = relative.rsplit('/').next().unwrap_or(relative);
    let cleaned: String = last
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .take(50)
        .collect();
    validate_environment_name(&cleaned).unwrap_or_else(|_| "environment".to_owned())
}

fn dedupe_names(envs: &mut [DetectedEnvironment]) {
    let mut seen: Vec<String> = Vec::new();
    for env in envs.iter_mut() {
        let base = env.name.clone();
        let mut n = 2;
        while seen.contains(&env.name.to_lowercase()) {
            env.name = format!("{base}-{n}");
            n += 1;
        }
        seen.push(env.name.to_lowercase());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_root_and_nested_environments_and_skips_noise() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("Vagrantfile"), "").unwrap();
        fs::create_dir_all(root.join("envs/testing")).unwrap();
        fs::write(root.join("envs/testing/Vagrantfile"), "").unwrap();
        fs::create_dir_all(root.join("envs/testing/.vagrant")).unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::write(root.join("node_modules/pkg/Vagrantfile"), "").unwrap();
        fs::create_dir_all(root.join(".git/x")).unwrap();
        fs::write(root.join(".git/x/Vagrantfile"), "").unwrap();
        fs::create_dir_all(root.join("a/b/c/d")).unwrap();
        fs::write(root.join("a/b/c/d/Vagrantfile"), "").unwrap();

        let found = inspect(root);
        let envs: Vec<_> = found
            .environments
            .iter()
            .map(|e| {
                (
                    e.name.as_str(),
                    e.relative_path.as_str(),
                    e.has_vagrant_state,
                )
            })
            .collect();
        assert_eq!(
            envs,
            [("default", ".", false), ("testing", "envs/testing", true)]
        );
        assert!(!found.has_portable_metadata);
    }

    #[test]
    fn uses_portable_metadata_when_valid() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".vagrant-studio")).unwrap();
        fs::write(
            root.join(".vagrant-studio/project.json"),
            r#"{"schemaVersion":1,"project":{"name":"Web Platform","tags":["Web"]},"environments":[{"name":"development","path":"."}]}"#,
        )
        .unwrap();
        fs::write(root.join("Vagrantfile"), "").unwrap();
        fs::create_dir_all(root.join("qa")).unwrap();
        fs::write(root.join("qa/vagrantfile"), "").unwrap();

        let found = inspect(root);
        assert_eq!(found.suggested_name, "Web Platform");
        assert_eq!(found.suggested_tags, vec!["web"]);
        let envs: Vec<_> = found
            .environments
            .iter()
            .map(|e| {
                (
                    e.name.as_str(),
                    e.relative_path.as_str(),
                    e.vagrantfile.as_deref(),
                    e.from_metadata,
                )
            })
            .collect();
        assert_eq!(
            envs,
            [
                ("development", ".", Some("Vagrantfile"), true),
                ("qa", "qa", Some("vagrantfile"), false)
            ]
        );
    }

    #[test]
    fn reports_invalid_portable_metadata_without_failing() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".vagrant-studio")).unwrap();
        fs::write(
            dir.path().join(".vagrant-studio/project.json"),
            r#"{"schemaVersion":9,"project":{"name":"x"}}"#,
        )
        .unwrap();
        let found = inspect(dir.path());
        assert!(found.has_portable_metadata);
        assert!(found.portable_error.unwrap().contains("newer"));
        assert_eq!(
            found.suggested_name,
            dir.path().file_name().unwrap().to_string_lossy()
        );
    }

    #[test]
    fn duplicate_names_get_suffixes() {
        let dir = tempfile::tempdir().unwrap();
        for p in ["a/web", "b/web"] {
            fs::create_dir_all(dir.path().join(p)).unwrap();
            fs::write(dir.path().join(p).join("Vagrantfile"), "").unwrap();
        }
        let names: Vec<_> = inspect(dir.path())
            .environments
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(names, ["web", "web-2"]);
    }
}
