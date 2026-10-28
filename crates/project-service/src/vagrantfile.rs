//! Reading and saving an environment's Vagrantfile (ADR-0005). Every save
//! checks that the file hasn't changed since the editor read it, keeps a
//! timestamped backup of the previous content in the app data folder, and
//! replaces the file atomically (temp file + rename).

use std::io::Write as _;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::FormatItem;
use time::macros::format_description;
use vagrant_studio_config_model::{Analysis, analyze, checksum};
use vagrant_studio_domain::{Environment, EnvironmentId};

use crate::{ProjectService, Result, ServiceError, canonical_dir, inspect, io, path_string};

/// Larger files are refused: a Vagrantfile is a small text file.
pub const MAX_VAGRANTFILE_BYTES: usize = 1024 * 1024;
/// Backups kept per environment; older ones are deleted after a save.
pub const KEEP_BACKUPS: usize = 50;
const BACKUP_PREFIX: &str = "Vagrantfile.";
const BACKUP_TIME: &[FormatItem<'static>] =
    format_description!("[year][month][day]T[hour][minute][second].[subsecond digits:3]Z");

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VagrantfileDocument {
    pub environment_id: EnvironmentId,
    /// Absolute path of the file (which may not exist yet).
    pub path: String,
    pub exists: bool,
    pub content: String,
    /// SHA-256 of the file as read; `None` when it doesn't exist.
    pub revision: Option<String>,
    pub analysis: Analysis,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveVagrantfileRequest {
    pub content: String,
    /// Revision the edit started from; `None` when the file didn't exist.
    pub base_revision: Option<String>,
    /// Allows replacing a Vagrantfile the visual editor didn't write with
    /// generated content (the user has seen the diff).
    #[serde(default)]
    pub replace_unmanaged: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveVagrantfileOutcome {
    pub document: VagrantfileDocument,
    /// Backup of the previous content, if there was one.
    pub backup: Option<VagrantfileBackup>,
    /// False when the content was already on disk (nothing was written).
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VagrantfileBackup {
    pub name: String,
    pub path: String,
    pub size: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

struct Location {
    path: PathBuf,
    /// File name relative to the environment folder.
    name: String,
}

impl ProjectService {
    /// The environment's Vagrantfile and what the visual editor makes of it.
    pub async fn vagrantfile(&self, id: EnvironmentId) -> Result<VagrantfileDocument> {
        let (_, env) = self.environment(id).await?;
        let loc = locate(&env)?;
        document(id, &loc)
    }

    /// Saves `req.content` as the environment's Vagrantfile, creating it if
    /// it doesn't exist. Refused when the file changed since `base_revision`.
    pub async fn save_vagrantfile(
        &self,
        id: EnvironmentId,
        req: SaveVagrantfileRequest,
    ) -> Result<SaveVagrantfileOutcome> {
        let (_, env) = self.environment(id).await?;
        let loc = locate(&env)?;
        if req.content.len() > MAX_VAGRANTFILE_BYTES {
            return Err(ServiceError::Unsafe(
                "The Vagrantfile is larger than 1 MB, which Vagrant Studio doesn't save.".into(),
            ));
        }
        if req.content.contains('\0') {
            return Err(ServiceError::Unsafe(
                "The content contains a NUL character and can't be saved as a Vagrantfile.".into(),
            ));
        }

        let current = read(&loc.path)?;
        let current_revision = current.as_ref().map(|c| checksum(c.as_bytes()));
        if current_revision != req.base_revision {
            return Err(ServiceError::Stale {
                message: match &current {
                    Some(_) => format!(
                        "{} changed on disk after you opened it.",
                        loc.path.display()
                    ),
                    None => format!("{} no longer exists.", loc.path.display()),
                },
                existing: current,
            });
        }
        if current.as_deref() == Some(req.content.as_str()) {
            return Ok(SaveVagrantfileOutcome {
                document: document(id, &loc)?,
                backup: None,
                changed: false,
            });
        }
        if let Some(existing) = &current
            && !req.replace_unmanaged
            && analyze(&req.content).managed
            && !analyze(existing).managed
        {
            return Err(ServiceError::ConfirmationRequired {
                message: "This replaces a Vagrantfile that the visual editor didn't write. Review the changes and confirm the replacement.".into(),
                existing: Some(existing.clone()),
            });
        }

        let backup = match &current {
            Some(existing) => Some(self.write_backup(id, existing)?),
            None => None,
        };
        write_atomically(&loc.path, &req.content)?;
        if env.vagrantfile_path.as_deref() != Some(loc.name.as_str()) {
            self.store
                .set_environment_vagrantfile(id, Some(&loc.name))
                .await?;
        }
        if let Err(e) = self.prune_backups(id) {
            tracing::warn!(error = %e, "could not prune Vagrantfile backups");
        }
        Ok(SaveVagrantfileOutcome {
            document: document(id, &loc)?,
            backup,
            changed: true,
        })
    }

    /// Backups of the environment's Vagrantfile, newest first.
    pub async fn vagrantfile_backups(&self, id: EnvironmentId) -> Result<Vec<VagrantfileBackup>> {
        self.environment(id).await?;
        self.list_backups(id)
    }

    /// The content of one backup.
    pub async fn vagrantfile_backup(&self, id: EnvironmentId, name: &str) -> Result<String> {
        self.environment(id).await?;
        let backup = self
            .list_backups(id)?
            .into_iter()
            .find(|b| b.name == name)
            .ok_or_else(|| ServiceError::NotFound("Backup not found.".into()))?;
        std::fs::read_to_string(&backup.path).map_err(io(format!("Could not read {}", backup.path)))
    }

    fn backups_dir(&self, id: EnvironmentId) -> Result<PathBuf> {
        let root = self.backups.as_ref().ok_or_else(|| {
            ServiceError::Unsafe("No backup folder is configured, so nothing is saved.".into())
        })?;
        Ok(root.join(id.to_string()))
    }

    fn write_backup(&self, id: EnvironmentId, content: &str) -> Result<VagrantfileBackup> {
        let dir = self.backups_dir(id)?;
        std::fs::create_dir_all(&dir).map_err(io(format!("Could not create {}", dir.display())))?;
        let now = OffsetDateTime::now_utc();
        let stamp = now.format(BACKUP_TIME).expect("valid format");
        // Two saves within a millisecond get a numeric suffix.
        for n in 0..100 {
            let name = match n {
                0 => format!("{BACKUP_PREFIX}{stamp}"),
                n => format!("{BACKUP_PREFIX}{stamp}-{n}"),
            };
            let path = dir.join(&name);
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut file) => {
                    file.write_all(content.as_bytes())
                        .and_then(|()| file.sync_all())
                        .map_err(io(format!("Could not write the backup {}", path.display())))?;
                    return Ok(VagrantfileBackup {
                        name,
                        path: path_string(&path)?,
                        size: content.len() as u64,
                        created_at: now,
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(io(format!("Could not create {}", path.display()))(e)),
            }
        }
        Err(ServiceError::Unsafe(
            "Could not find a free backup file name.".into(),
        ))
    }

    fn list_backups(&self, id: EnvironmentId) -> Result<Vec<VagrantfileBackup>> {
        let dir = self.backups_dir(id)?;
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(io(format!("Could not read {}", dir.display()))(e)),
        };
        let mut backups: Vec<VagrantfileBackup> = entries
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                let created_at = backup_time(&name)?;
                let meta = std::fs::metadata(entry.path()).ok()?;
                let path = entry.path().to_str()?.to_owned();
                meta.is_file().then_some(VagrantfileBackup {
                    path,
                    size: meta.len(),
                    created_at,
                    name,
                })
            })
            .collect();
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.name.cmp(&a.name)));
        Ok(backups)
    }

    fn prune_backups(&self, id: EnvironmentId) -> Result<()> {
        for old in self.list_backups(id)?.iter().skip(KEEP_BACKUPS) {
            std::fs::remove_file(&old.path)
                .map_err(io(format!("Could not remove {}", old.path)))?;
        }
        Ok(())
    }
}

/// Parses the timestamp out of `Vagrantfile.<stamp>[-n]`.
fn backup_time(name: &str) -> Option<OffsetDateTime> {
    let stamp = name.strip_prefix(BACKUP_PREFIX)?;
    let stamp = stamp.split_once('-').map_or(stamp, |(s, _)| s);
    time::PrimitiveDateTime::parse(stamp, BACKUP_TIME)
        .ok()
        .map(|t| t.assume_utc())
}

/// Where the environment's Vagrantfile is, or would be created.
fn locate(env: &Environment) -> Result<Location> {
    let root = canonical_dir(&env.root_path, "rootPath")?;
    // The recorded name, unless that file is gone and another spelling exists.
    let name = env
        .vagrantfile_path
        .clone()
        .filter(|n| root.join(n).exists())
        .or_else(|| inspect::vagrantfile_in(&root))
        .or_else(|| env.vagrantfile_path.clone())
        .unwrap_or_else(|| "Vagrantfile".to_owned());
    let mut parts = Path::new(&name).components();
    if !matches!(
        (parts.next(), parts.next()),
        (Some(Component::Normal(_)), None)
    ) {
        return Err(ServiceError::Unsafe(format!(
            "The recorded Vagrantfile name \"{name}\" isn't a file in the environment folder."
        )));
    }
    let path = root.join(&name);
    match std::fs::symlink_metadata(&path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(ServiceError::Unsafe(format!(
            "{} is a symbolic link. Edit the file it points to instead.",
            path.display()
        ))),
        Ok(meta) if !meta.is_file() => Err(ServiceError::Unsafe(format!(
            "{} is not a file.",
            path.display()
        ))),
        _ => Ok(Location { path, name }),
    }
}

fn read(path: &Path) -> Result<Option<String>> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io(format!("Could not read {}", path.display()))(e)),
    };
    if bytes.len() > MAX_VAGRANTFILE_BYTES {
        return Err(ServiceError::Unsafe(format!(
            "{} is larger than 1 MB, which is too large to edit here.",
            path.display()
        )));
    }
    String::from_utf8(bytes).map(Some).map_err(|_| {
        ServiceError::Unsafe(format!(
            "{} isn't UTF-8 text, so it can't be edited here.",
            path.display()
        ))
    })
}

fn document(id: EnvironmentId, loc: &Location) -> Result<VagrantfileDocument> {
    let current = read(&loc.path)?;
    Ok(VagrantfileDocument {
        environment_id: id,
        path: path_string(&loc.path)?,
        exists: current.is_some(),
        revision: current.as_ref().map(|c| checksum(c.as_bytes())),
        analysis: analyze(current.as_deref().unwrap_or("")),
        content: current.unwrap_or_default(),
    })
}

/// Writes a temp file next to `path`, flushes it, then renames it over
/// `path`, so readers see either the old or the new file, never a mix.
fn write_atomically(path: &Path, content: &str) -> Result<()> {
    let dir = path.parent().expect("file in a folder");
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Vagrantfile");
    let tmp = dir.join(format!(".{file_name}.vagrant-studio-tmp"));
    let result = (|| {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        drop(file);
        #[cfg(unix)]
        if let Ok(meta) = std::fs::metadata(path) {
            std::fs::set_permissions(&tmp, meta.permissions())?;
        }
        std::fs::rename(&tmp, path)
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(io(format!("Could not save {}", path.display()))(e));
    }
    Ok(())
}
