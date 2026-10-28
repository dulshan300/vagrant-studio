use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::host::{HostInfo, HostOs};

/// Inputs for executable lookup, separated from the process environment so
/// the rules can be tested on any host.
#[derive(Debug, Clone)]
pub struct ExecutableSearch {
    /// Directories to search, in order.
    pub dirs: Vec<PathBuf>,
    /// Extensions to try on Windows (from `PATHEXT`), e.g. `[".EXE", ".BAT"]`.
    /// Empty on Linux, where the bare name is used.
    pub extensions: Vec<String>,
    /// Skip Windows drive mounts (`/mnt/c/...`) when running inside WSL so a
    /// Linux process never picks up a Windows executable.
    pub skip_windows_mounts: bool,
}

impl ExecutableSearch {
    pub fn from_env(host: &HostInfo) -> Self {
        Self::from_values(host, std::env::var_os("PATH"), std::env::var_os("PATHEXT"))
    }

    pub fn from_values(host: &HostInfo, path: Option<OsString>, pathext: Option<OsString>) -> Self {
        let mut dirs: Vec<PathBuf> = path
            .map(|p| std::env::split_paths(&p).collect())
            .unwrap_or_default();
        dirs.extend(well_known_dirs(host.os));

        let extensions = if host.os == HostOs::Windows {
            pathext
                .and_then(|v| v.into_string().ok())
                .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_owned())
                .split(';')
                .filter(|e| !e.is_empty())
                .map(str::to_owned)
                .collect()
        } else {
            Vec::new()
        };

        Self {
            dirs,
            extensions,
            skip_windows_mounts: host.is_wsl,
        }
    }
}

/// Default install locations checked after `PATH`.
fn well_known_dirs(os: HostOs) -> Vec<PathBuf> {
    match os {
        HostOs::Windows => {
            let program_files =
                std::env::var_os("ProgramFiles").unwrap_or_else(|| r"C:\Program Files".into());
            vec![PathBuf::from(program_files).join("Vagrant").join("bin")]
        }
        HostOs::Linux | HostOs::Macos => vec![
            PathBuf::from("/usr/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/opt/vagrant/bin"),
        ],
        HostOs::Other => Vec::new(),
    }
}

/// Returns the first matching executable, or `None` if not found.
pub fn find_executable(name: &str, search: &ExecutableSearch) -> Option<PathBuf> {
    if name.is_empty() || Path::new(name).components().count() != 1 {
        return None;
    }
    search
        .dirs
        .iter()
        .filter(|dir| !(search.skip_windows_mounts && is_windows_mount(dir)))
        .flat_map(|dir| candidates(dir, name, &search.extensions))
        .find(|candidate| is_executable(candidate))
}

fn candidates(dir: &Path, name: &str, extensions: &[String]) -> Vec<PathBuf> {
    if extensions.is_empty() {
        return vec![dir.join(name)];
    }
    let has_extension = extensions.iter().any(|ext| {
        name.to_ascii_lowercase()
            .ends_with(&ext.to_ascii_lowercase())
    });
    if has_extension {
        vec![dir.join(name)]
    } else {
        extensions
            .iter()
            .map(|ext| dir.join(format!("{name}{ext}")))
            .collect()
    }
}

/// `/mnt/<drive letter>` and anything below it.
fn is_windows_mount(dir: &Path) -> bool {
    let mut parts = dir.components().map(|c| c.as_os_str().to_string_lossy());
    matches!(
        (parts.next().as_deref(), parts.next().as_deref(), parts.next()),
        (Some("/"), Some("mnt"), Some(drive)) if drive.len() == 1 && drive.chars().all(|c| c.is_ascii_alphabetic())
    )
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::Arch;

    fn linux(is_wsl: bool) -> HostInfo {
        HostInfo {
            os: HostOs::Linux,
            arch: Arch::X86_64,
            is_wsl,
        }
    }

    fn search(dirs: Vec<PathBuf>, extensions: &[&str], skip: bool) -> ExecutableSearch {
        ExecutableSearch {
            dirs,
            extensions: extensions.iter().map(|e| (*e).to_owned()).collect(),
            skip_windows_mounts: skip,
        }
    }

    fn touch(path: &Path) {
        std::fs::write(path, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[test]
    fn finds_first_match_in_order() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        touch(&b.path().join("vagrant"));
        touch(&a.path().join("vagrant"));
        let s = search(vec![a.path().into(), b.path().into()], &[], false);
        assert_eq!(
            find_executable("vagrant", &s),
            Some(a.path().join("vagrant"))
        );
    }

    #[test]
    fn handles_directories_with_spaces_and_non_ascii() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("Program Files").join("Vägrant bin");
        std::fs::create_dir_all(&dir).unwrap();
        touch(&dir.join("vagrant"));
        let s = search(vec![dir.clone()], &[], false);
        assert_eq!(find_executable("vagrant", &s), Some(dir.join("vagrant")));
    }

    #[test]
    fn returns_none_when_missing() {
        let a = tempfile::tempdir().unwrap();
        let s = search(vec![a.path().into()], &[], false);
        assert_eq!(find_executable("vagrant", &s), None);
    }

    #[test]
    fn rejects_names_with_path_components() {
        let s = search(vec![PathBuf::from("/usr/bin")], &[], false);
        assert_eq!(find_executable("../vagrant", &s), None);
        assert_eq!(find_executable("bin/vagrant", &s), None);
        assert_eq!(find_executable("", &s), None);
    }

    #[cfg(unix)]
    #[test]
    fn ignores_non_executable_files() {
        let a = tempfile::tempdir().unwrap();
        std::fs::write(a.path().join("vagrant"), "").unwrap();
        let s = search(vec![a.path().into()], &[], false);
        assert_eq!(find_executable("vagrant", &s), None);
    }

    #[test]
    fn applies_windows_extensions() {
        let a = tempfile::tempdir().unwrap();
        touch(&a.path().join("vagrant.EXE"));
        let s = search(vec![a.path().into()], &[".COM", ".EXE"], false);
        assert_eq!(
            find_executable("vagrant", &s),
            Some(a.path().join("vagrant.EXE"))
        );
    }

    #[test]
    fn detects_windows_mounts() {
        assert!(is_windows_mount(Path::new(
            "/mnt/c/Program Files/Vagrant/bin"
        )));
        assert!(is_windows_mount(Path::new("/mnt/d")));
        assert!(!is_windows_mount(Path::new("/mnt/data/bin")));
        assert!(!is_windows_mount(Path::new("/usr/bin")));
    }

    #[test]
    fn wsl_search_skips_windows_mounts_and_keeps_linux_dirs() {
        let s = ExecutableSearch::from_values(
            &linux(true),
            Some("/mnt/c/Program Files/Vagrant/bin:/usr/bin".into()),
            None,
        );
        assert!(s.skip_windows_mounts);
        assert!(s.extensions.is_empty());
        assert_eq!(s.dirs[0], PathBuf::from("/mnt/c/Program Files/Vagrant/bin"));
        assert!(s.dirs.contains(&PathBuf::from("/opt/vagrant/bin")));

        let native = ExecutableSearch::from_values(&linux(false), None, None);
        assert!(!native.skip_windows_mounts);
    }

    #[test]
    fn windows_defaults_pathext() {
        let host = HostInfo {
            os: HostOs::Windows,
            arch: Arch::X86_64,
            is_wsl: false,
        };
        let s = ExecutableSearch::from_values(&host, None, None);
        assert_eq!(s.extensions, vec![".COM", ".EXE", ".BAT", ".CMD"]);
    }
}
