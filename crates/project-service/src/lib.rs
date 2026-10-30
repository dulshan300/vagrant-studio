//! Project use cases. This is the only layer allowed to touch project files,
//! and it does so conservatively:
//!
//! * importing and inspecting never write to the project folder;
//! * creating writes only into a new or empty folder and never overwrites;
//! * deleting removes metadata only, unless the user explicitly asks to move the
//!   folder to the OS trash, typed the project name, and no Vagrant machines
//!   appear to exist;
//! * portable metadata is shown before writing and never overwritten silently;
//! * Vagrantfile saves check for changes on disk, keep a backup in the app
//!   data folder and replace the file atomically (`vagrantfile.rs`).

mod gitignore;
mod inspect;
pub mod templates;
mod vagrantfile;
mod workspace;

use std::collections::HashSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vagrant_studio_domain::portable::{
    self, PortableEnvironment, PortableError, PortableProject, PortableProjectInfo,
    normalize_relative_path,
};
use vagrant_studio_domain::{
    Environment, EnvironmentId, EnvironmentType, NewEnvironment, NewProject, Project,
    ProjectChanges, ProjectId, ValidationError, normalize_tags, slugify, validate_description,
    validate_environment_name, validate_name, validate_provider_name, validate_repository_url,
};
use vagrant_studio_project_store::{ProjectFilter, ProjectUpdate, Store, StoreError};

pub use gitignore::GITIGNORE;
pub use inspect::{DetectedEnvironment, FolderInspection};
pub use templates::Template;
pub use vagrantfile::{
    KEEP_BACKUPS, MAX_VAGRANTFILE_BYTES, SaveVagrantfileOutcome, SaveVagrantfileRequest,
    VagrantfileBackup, VagrantfileDocument,
};
pub use workspace::TaskChanges;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    /// The action needs explicit user confirmation; `existing` carries the
    /// current content where useful (e.g. an existing project.json).
    #[error("{message}")]
    ConfirmationRequired {
        message: String,
        existing: Option<String>,
    },
    /// The file changed on disk since the user read it; `existing` is the
    /// current content (`None` if it was deleted).
    #[error("{message}")]
    Stale {
        message: String,
        existing: Option<String>,
    },
    /// Refused because it could lose data or affect running machines.
    #[error("{0}")]
    Unsafe(String),
    #[error("{context}: {source}")]
    Io {
        context: String,
        source: std::io::Error,
    },
    #[error(transparent)]
    Portable(#[from] PortableError),
    #[error(transparent)]
    Store(StoreError),
}

impl From<StoreError> for ServiceError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::Conflict(msg) => Self::Conflict(msg),
            other => Self::Store(other),
        }
    }
}

pub type Result<T> = std::result::Result<T, ServiceError>;

fn io(context: impl Into<String>) -> impl FnOnce(std::io::Error) -> ServiceError {
    let context = context.into();
    move |source| ServiceError::Io { context, source }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Existing folder in which the project folder is created.
    pub location: String,
    pub repository_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub template_id: Option<String>,
    /// Vagrant provider the template is rendered for (required with a template).
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProjectRequest {
    pub root_path: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub repository_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub environments: Vec<EnvironmentRequest>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentRequest {
    pub name: String,
    /// Relative to the project folder (`.` is the folder itself), or an
    /// absolute path inside it (as returned by a folder picker).
    pub relative_path: String,
    pub environment_type: Option<EnvironmentType>,
    /// Provider Vagrant should use for this environment's machines.
    #[serde(default)]
    pub default_provider: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteProjectRequest {
    #[serde(default)]
    pub delete_files: bool,
    /// Must equal the project name when `delete_files` is set.
    pub confirm_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteOutcome {
    pub files_moved_to_trash: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetails {
    pub project: Project,
    pub environments: Vec<Environment>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortablePreview {
    pub path: String,
    pub content: String,
    /// Current file content, if the file exists.
    pub existing: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// Folder pre-selected when creating projects.
    pub default_project_location: Option<String>,
    /// Explicit Vagrant executable; when unset Vagrant is searched for.
    #[serde(default)]
    pub vagrant_path: Option<String>,
    /// Provider pre-selected for new projects and environments.
    #[serde(default)]
    pub default_provider: Option<String>,
    /// Code editor executable; when unset the first installed known editor
    /// is used.
    #[serde(default)]
    pub editor_path: Option<String>,
    /// Where SSH sessions open.
    #[serde(default)]
    pub ssh_terminal: SshTerminal,
}

/// Where `vagrant ssh` opens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SshTerminal {
    /// The app's terminal panel.
    #[default]
    App,
    /// A new window of the system terminal.
    External,
}

const SETTINGS_KEY: &str = "app";

type TrashFn = dyn Fn(&Path) -> std::io::Result<()> + Send + Sync;

pub struct ProjectService {
    store: Store,
    trash: Box<TrashFn>,
    /// Vagrantfile backups go to `<backups>/<environment id>/`.
    backups: Option<PathBuf>,
}

impl ProjectService {
    pub fn new(store: Store) -> Self {
        Self::with_trash(store, |path| {
            trash::delete(path).map_err(|e| std::io::Error::other(e.to_string()))
        })
    }

    /// Lets tests observe trash requests without touching the real trash.
    pub fn with_trash(
        store: Store,
        trash: impl Fn(&Path) -> std::io::Result<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            store,
            trash: Box::new(trash),
            backups: None,
        }
    }

    /// Folder (in the app data directory) for Vagrantfile backups. Saving a
    /// Vagrantfile is refused until one is set.
    pub fn with_backups_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.backups = Some(dir.into());
        self
    }

    pub fn templates(&self) -> Vec<Template> {
        templates::all()
    }

    pub async fn list(&self, filter: ProjectFilter) -> Result<Vec<Project>> {
        Ok(self.store.list_projects(filter).await?)
    }

    pub async fn list_tags(&self) -> Result<Vec<(String, i64)>> {
        Ok(self.store.list_tags().await?)
    }

    pub async fn details(&self, id: ProjectId) -> Result<ProjectDetails> {
        let project = self.require(id).await?;
        let environments = self.store.list_environments(id).await?;
        Ok(ProjectDetails {
            project,
            environments,
        })
    }

    /// Creates a project folder (optionally with a template Vagrantfile) and registers it.
    pub async fn create(&self, req: CreateProjectRequest) -> Result<ProjectDetails> {
        let name = validate_name(&req.name)?;
        let new = NewProject {
            description: validate_description(&req.description)?,
            repository_url: validate_repository_url(req.repository_url.as_deref())?,
            tags: normalize_tags(&req.tags)?,
            root_path: String::new(),
            name,
        };
        let provider = validate_provider_name(req.provider.as_deref())?;
        let rendered = match req.template_id.as_deref().filter(|t| !t.is_empty()) {
            Some(id) => {
                let template = templates::find(id).ok_or_else(|| {
                    ValidationError::new("templateId", format!("Unknown template \"{id}\"."))
                })?;
                let provider = provider.clone().ok_or_else(|| {
                    ValidationError::new("provider", "Choose a provider for the template.")
                })?;
                let content = template
                    .render(&provider)
                    .map_err(|e| ValidationError::new("provider", e.to_string()))?;
                Some((content, provider))
            }
            None => None,
        };

        let location = canonical_dir(&req.location, "location")?;
        let folder = location.join(slugify(&new.name));
        let folder_str = path_string(&folder)?;
        if self
            .store
            .find_project_by_root(&folder_str)
            .await?
            .is_some()
        {
            return Err(ServiceError::Conflict(format!(
                "{folder_str} is already registered as a project."
            )));
        }

        let mut undo = Undo::default();
        if folder.exists() {
            if !folder.is_dir() || !is_empty_dir(&folder)? {
                return Err(ServiceError::Conflict(format!(
                    "{folder_str} already exists and isn't empty. Import it instead, or choose another name or location."
                )));
            }
        } else {
            std::fs::create_dir(&folder).map_err(io(format!("Could not create {folder_str}")))?;
            undo.created_dir = Some(folder.clone());
        }

        let result = async {
            if let Some((content, _)) = &rendered {
                let path = folder.join("Vagrantfile");
                write_new_file(&path, content)?;
                undo.created_file = Some(path);
            }
            let project = self
                .store
                .insert_project(&NewProject {
                    root_path: folder_str.clone(),
                    ..new
                })
                .await?;
            if let Some((_, provider)) = &rendered {
                self.store
                    .insert_environment(
                        project.id,
                        &NewEnvironment {
                            name: "default".into(),
                            root_path: folder_str.clone(),
                            vagrantfile_path: Some("Vagrantfile".into()),
                            default_provider: Some(provider.clone()),
                            environment_type: EnvironmentType::Development,
                        },
                    )
                    .await?;
            }
            self.details(project.id).await
        }
        .await;

        if result.is_err() {
            undo.run();
        }
        result
    }

    /// Read-only look at a folder before importing it.
    pub async fn inspect_folder(&self, path: &str) -> Result<FolderInspection> {
        let root = canonical_dir(path, "rootPath")?;
        let mut inspection = inspect::inspect(&root);
        inspection.existing_project_id = self
            .store
            .find_project_by_root(&path_string(&root)?)
            .await?;
        Ok(inspection)
    }

    /// Registers an existing folder. Never writes to the folder.
    pub async fn import(&self, req: ImportProjectRequest) -> Result<ProjectDetails> {
        let root = canonical_dir(&req.root_path, "rootPath")?;
        let new = NewProject {
            name: validate_name(&req.name)?,
            description: validate_description(&req.description)?,
            repository_url: validate_repository_url(req.repository_url.as_deref())?,
            tags: normalize_tags(&req.tags)?,
            root_path: path_string(&root)?,
        };
        let mut names = HashSet::new();
        let envs = req
            .environments
            .iter()
            .map(|e| {
                let env = resolve_environment(&root, e)?;
                if !names.insert(env.name.to_lowercase()) {
                    return Err(ValidationError::new(
                        "environments",
                        format!("Environment \"{}\" is listed twice.", env.name),
                    )
                    .into());
                }
                Ok(env)
            })
            .collect::<Result<Vec<_>>>()?;

        let project = self.store.insert_project(&new).await?;
        for env in &envs {
            if let Err(e) = self.store.insert_environment(project.id, env).await {
                self.store.delete_project(project.id).await?;
                return Err(e.into());
            }
        }
        self.details(project.id).await
    }

    pub async fn add_environment(
        &self,
        project_id: ProjectId,
        req: EnvironmentRequest,
    ) -> Result<Environment> {
        let project = self.require(project_id).await?;
        let root = canonical_dir(&project.root_path, "rootPath")?;
        let env = resolve_environment(&root, &req)?;
        Ok(self.store.insert_environment(project_id, &env).await?)
    }

    /// Sets (or clears) the provider used for an environment's machines.
    /// Existing machines keep the provider they were created with; Vagrant
    /// reports those regardless of this setting.
    pub async fn set_environment_provider(
        &self,
        id: EnvironmentId,
        provider: Option<String>,
    ) -> Result<Environment> {
        let provider = validate_provider_name(provider.as_deref())?;
        if self.store.get_environment(id).await?.is_none() {
            return Err(ServiceError::NotFound("Environment not found.".into()));
        }
        Ok(self
            .store
            .set_environment_provider(id, provider.as_deref())
            .await?)
    }

    /// Removes environment metadata only; files and machines are untouched.
    pub async fn remove_environment(&self, id: EnvironmentId) -> Result<()> {
        if !self.store.delete_environment(id).await? {
            return Err(ServiceError::NotFound("Environment not found.".into()));
        }
        Ok(())
    }

    pub async fn update(&self, id: ProjectId, changes: ProjectChanges) -> Result<Project> {
        let update = ProjectUpdate {
            name: changes.name.as_deref().map(validate_name).transpose()?,
            description: changes
                .description
                .as_deref()
                .map(validate_description)
                .transpose()?,
            repository_url: changes
                .repository_url
                .map(|url| validate_repository_url(url.as_deref()))
                .transpose()?,
            tags: changes.tags.as_deref().map(normalize_tags).transpose()?,
        };
        self.require(id).await?;
        Ok(self.store.update_project(id, &update).await?)
    }

    pub async fn set_favorite(&self, id: ProjectId, favorite: bool) -> Result<Project> {
        self.require(id).await?;
        Ok(self.store.set_favorite(id, favorite).await?)
    }

    pub async fn set_archived(&self, id: ProjectId, archived: bool) -> Result<Project> {
        self.require(id).await?;
        Ok(self.store.set_archived(id, archived).await?)
    }

    pub async fn mark_opened(&self, id: ProjectId) -> Result<()> {
        Ok(self.store.mark_opened(id).await?)
    }

    /// Deletes project metadata; optionally moves the folder to the OS trash first.
    pub async fn delete(&self, id: ProjectId, req: DeleteProjectRequest) -> Result<DeleteOutcome> {
        let project = self.require(id).await?;
        let mut moved = false;
        if req.delete_files {
            if req.confirm_name.as_deref().map(str::trim) != Some(project.name.as_str()) {
                return Err(ValidationError::new(
                    "confirmName",
                    "Type the project name exactly to confirm deleting its files.",
                )
                .into());
            }
            let root = PathBuf::from(&project.root_path);
            if root.exists() {
                let root = dunce::canonicalize(&root)
                    .map_err(io(format!("Could not resolve {}", project.root_path)))?;
                ensure_safe_to_trash(&root)?;
                let environments = self.store.list_environments(id).await?;
                let mut dirs = vec![root.clone()];
                dirs.extend(environments.iter().map(|e| PathBuf::from(&e.root_path)));
                if let Some(dir) = dirs.iter().find(|d| has_vagrant_machines(d)) {
                    return Err(ServiceError::Unsafe(format!(
                        "Vagrant machines may still exist for {} (found .vagrant/machines). Destroy them with Vagrant first, or remove the project from Vagrant Studio without deleting files.",
                        dir.display()
                    )));
                }
                (self.trash)(&root).map_err(io(format!(
                    "Could not move {} to the trash",
                    root.display()
                )))?;
                moved = true;
            }
        }
        self.store.delete_project(id).await?;
        Ok(DeleteOutcome {
            files_moved_to_trash: moved,
        })
    }

    /// What `.vagrant-studio/project.json` would contain, plus the current file.
    pub async fn preview_portable(&self, id: ProjectId) -> Result<PortablePreview> {
        let details = self.details(id).await?;
        let root = canonical_dir(&details.project.root_path, "rootPath")?;
        let path = root.join(portable::DIR_NAME).join(portable::FILE_NAME);
        let existing = match std::fs::read_to_string(&path) {
            Ok(s) => Some(s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(io(format!("Could not read {}", path.display()))(e)),
        };
        Ok(PortablePreview {
            path: path_string(&path)?,
            content: build_portable(&root, &details).to_json(),
            existing,
        })
    }

    /// Writes project.json. Refuses to replace a different existing file unless `overwrite`.
    pub async fn write_portable(&self, id: ProjectId, overwrite: bool) -> Result<PortablePreview> {
        let preview = self.preview_portable(id).await?;
        match &preview.existing {
            Some(existing) if *existing == preview.content => return Ok(preview),
            Some(existing) if !overwrite => {
                return Err(ServiceError::ConfirmationRequired {
                    message: format!("{} already exists with different content.", preview.path),
                    existing: Some(existing.clone()),
                });
            }
            _ => {}
        }
        let path = PathBuf::from(&preview.path);
        let dir = path.parent().expect("project.json has a parent");
        std::fs::create_dir_all(dir).map_err(io(format!("Could not create {}", dir.display())))?;
        let tmp = dir.join(format!("{}.tmp", portable::FILE_NAME));
        std::fs::write(&tmp, &preview.content)
            .map_err(io(format!("Could not write {}", tmp.display())))?;
        std::fs::rename(&tmp, &path).map_err(io(format!("Could not write {}", path.display())))?;
        Ok(PortablePreview {
            existing: Some(preview.content.clone()),
            ..preview
        })
    }

    pub async fn settings(&self) -> Result<AppSettings> {
        Ok(self
            .store
            .get_setting(SETTINGS_KEY)
            .await?
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default())
    }

    pub async fn save_settings(&self, mut settings: AppSettings) -> Result<AppSettings> {
        settings.default_provider = validate_provider_name(settings.default_provider.as_deref())?;
        if let Some(loc) = &settings.default_project_location {
            canonical_dir(loc, "defaultProjectLocation")?;
        }
        if let Some(path) = settings.vagrant_path.as_deref().map(str::trim) {
            let p = Path::new(path);
            if !p.is_absolute() || !p.is_file() {
                return Err(ValidationError::new(
                    "vagrantPath",
                    "Choose the Vagrant executable (an existing file), or leave empty to search automatically.",
                )
                .into());
            }
        }
        if let Some(path) = settings.editor_path.as_deref().map(str::trim) {
            let p = Path::new(path);
            // A .cmd or .bat wrapper would run through cmd.exe, which parses
            // the folder path; only real executables are started.
            let windows_wrapper =
                cfg!(windows) && !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"));
            if !p.is_absolute() || !p.is_file() || windows_wrapper {
                return Err(ValidationError::new(
                    "editorPath",
                    "Choose the editor's program file (on Windows, its .exe), or leave empty to use a detected editor.",
                )
                .into());
            }
        }
        let json = serde_json::to_string(&settings).expect("settings serialize");
        self.store.set_setting(SETTINGS_KEY, &json).await?;
        Ok(settings)
    }

    /// Every environment with its project's name.
    pub async fn all_environments(&self) -> Result<Vec<(Environment, String)>> {
        Ok(self.store.list_all_environments().await?)
    }

    /// For each folder, the registered environment rooted there (if any),
    /// with its project's name. Folders that don't exist match nothing.
    pub async fn environments_at(
        &self,
        folders: &[&str],
    ) -> Result<Vec<Option<(Environment, String)>>> {
        let environments: Vec<(PathBuf, (Environment, String))> = self
            .all_environments()
            .await?
            .into_iter()
            .filter_map(|e| Some((dunce::canonicalize(&e.0.root_path).ok()?, e)))
            .collect();
        Ok(folders
            .iter()
            .map(|folder| {
                let folder = dunce::canonicalize(folder).ok()?;
                environments
                    .iter()
                    .find(|(root, _)| *root == folder)
                    .map(|(_, e)| e.clone())
            })
            .collect())
    }

    /// An environment together with its project.
    pub async fn environment(&self, id: EnvironmentId) -> Result<(Project, Environment)> {
        let env = self
            .store
            .get_environment(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound("Environment not found.".into()))?;
        Ok((self.require(env.project_id).await?, env))
    }

    async fn require(&self, id: ProjectId) -> Result<Project> {
        self.store
            .get_project(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound("Project not found.".into()))
    }
}

fn build_portable(root: &Path, details: &ProjectDetails) -> PortableProject {
    let p = &details.project;
    PortableProject {
        schema_version: portable::SCHEMA_VERSION,
        project: PortableProjectInfo {
            name: p.name.clone(),
            description: p.description.clone(),
            repository_url: p.repository_url.clone(),
            tags: p.tags.clone(),
        },
        environments: details
            .environments
            .iter()
            .filter_map(|e| {
                // Environments outside the project folder can't be described portably.
                let dir = dunce::canonicalize(&e.root_path).ok()?;
                dir.starts_with(root).then(|| PortableEnvironment {
                    name: e.name.clone(),
                    path: inspect::relative_slash_path(root, &dir),
                    vagrantfile: e.vagrantfile_path.clone(),
                    default_provider: e.default_provider.clone(),
                    environment_type: Some(e.environment_type),
                    machines: Vec::new(),
                })
            })
            .collect(),
    }
}

fn resolve_environment(root: &Path, req: &EnvironmentRequest) -> Result<NewEnvironment> {
    let name = validate_environment_name(&req.name)?;
    let invalid = || {
        ValidationError::new(
            "environments",
            format!("Environment \"{name}\" must be a folder inside the project folder."),
        )
    };
    let requested = Path::new(req.relative_path.trim());
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(normalize_relative_path(&req.relative_path).ok_or_else(invalid)?)
    };
    let dir = dunce::canonicalize(candidate).map_err(|_| invalid())?;
    if !dir.is_dir() || !dir.starts_with(root) {
        return Err(invalid().into());
    }
    Ok(NewEnvironment {
        vagrantfile_path: inspect::vagrantfile_in(&dir),
        root_path: path_string(&dir)?,
        default_provider: validate_provider_name(req.default_provider.as_deref())?,
        environment_type: req.environment_type.unwrap_or(EnvironmentType::Development),
        name,
    })
}

/// Absolute, existing directory, canonicalized without Windows `\\?\` prefixes.
fn canonical_dir(path: &str, field: &'static str) -> Result<PathBuf> {
    let p = Path::new(path.trim());
    if path.trim().is_empty() || !p.is_absolute() {
        return Err(ValidationError::new(field, "Choose an absolute folder path.").into());
    }
    let dir = dunce::canonicalize(p)
        .map_err(|_| ValidationError::new(field, format!("{} does not exist.", p.display())))?;
    if !dir.is_dir() {
        return Err(
            ValidationError::new(field, format!("{} is not a folder.", dir.display())).into(),
        );
    }
    Ok(dir)
}

fn path_string(path: &Path) -> Result<String> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        ServiceError::Unsafe(format!(
            "{} contains characters that can't be stored safely. Rename the folder and try again.",
            path.display()
        ))
    })
}

fn is_empty_dir(dir: &Path) -> Result<bool> {
    Ok(std::fs::read_dir(dir)
        .map_err(io(format!("Could not read {}", dir.display())))?
        .next()
        .is_none())
}

/// Creates a file, failing if it already exists.
fn write_new_file(path: &Path, content: &str) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(io(format!("Could not create {}", path.display())))?;
    file.write_all(content.as_bytes())
        .map_err(io(format!("Could not write {}", path.display())))
}

/// `.vagrant/machines/<name>/<provider>/id` exists when Vagrant created a machine.
fn has_vagrant_machines(dir: &Path) -> bool {
    let Ok(machines) = std::fs::read_dir(dir.join(".vagrant").join("machines")) else {
        return false;
    };
    machines.flatten().any(|machine| {
        std::fs::read_dir(machine.path())
            .map(|providers| providers.flatten().any(|p| p.path().join("id").is_file()))
            .unwrap_or(false)
    })
}

/// Refuses filesystem roots, top-level folders (`/home`, `C:\Users`) and the home folder.
fn ensure_safe_to_trash(root: &Path) -> Result<()> {
    let normal = root
        .components()
        .filter(|c| matches!(c, std::path::Component::Normal(_)))
        .count();
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .and_then(|h| dunce::canonicalize(h).ok());
    if normal < 2 || home.as_deref() == Some(root) {
        return Err(ServiceError::Unsafe(format!(
            "Refusing to delete {}: it is a top-level or home folder.",
            root.display()
        )));
    }
    Ok(())
}

/// Reverts filesystem changes made by a failed create.
#[derive(Default)]
struct Undo {
    created_dir: Option<PathBuf>,
    created_file: Option<PathBuf>,
}

impl Undo {
    fn run(self) {
        if let Some(file) = self.created_file {
            let _ = std::fs::remove_file(file);
        }
        if let Some(dir) = self.created_dir {
            // remove_dir only succeeds on an empty directory.
            let _ = std::fs::remove_dir(dir);
        }
    }
}

#[cfg(test)]
mod tests;
