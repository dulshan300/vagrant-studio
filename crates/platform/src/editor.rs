//! Finding installed code editors and opening a folder in one.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;

use crate::terminal::spawn_detached;
use crate::{HostOs, PlatformAdapter, SystemPlatform};

/// An installed editor that opens a folder given as its only argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedEditor {
    pub name: String,
    pub path: PathBuf,
}

struct KnownEditor {
    name: &'static str,
    /// Executable names searched on `PATH` on Linux.
    linux: &'static [&'static str],
    /// Windows install locations: an environment variable naming a base
    /// folder, and the executable below it. The `.exe` is used directly;
    /// `code.cmd` and similar wrappers would need `cmd.exe` to run.
    windows: &'static [(&'static str, &'static str)],
}

/// Editors in order of preference.
const KNOWN_EDITORS: &[KnownEditor] = &[
    KnownEditor {
        name: "Visual Studio Code",
        linux: &["code"],
        windows: &[
            ("LOCALAPPDATA", r"Programs\Microsoft VS Code\Code.exe"),
            ("ProgramFiles", r"Microsoft VS Code\Code.exe"),
        ],
    },
    KnownEditor {
        name: "Cursor",
        linux: &["cursor"],
        windows: &[("LOCALAPPDATA", r"Programs\cursor\Cursor.exe")],
    },
    KnownEditor {
        name: "VSCodium",
        linux: &["codium"],
        windows: &[
            ("LOCALAPPDATA", r"Programs\VSCodium\VSCodium.exe"),
            ("ProgramFiles", r"VSCodium\VSCodium.exe"),
        ],
    },
    KnownEditor {
        name: "Zed",
        linux: &["zed", "zeditor"],
        windows: &[("LOCALAPPDATA", r"Programs\Zed\Zed.exe")],
    },
    KnownEditor {
        name: "Sublime Text",
        linux: &["subl"],
        windows: &[("ProgramFiles", r"Sublime Text\sublime_text.exe")],
    },
];

/// The known editors installed on this host. `find` locates an executable on
/// `PATH`; `env` reads an environment variable.
pub fn find_editors(
    os: HostOs,
    find: impl Fn(&str) -> Option<PathBuf>,
    env: impl Fn(&str) -> Option<OsString>,
    exists: impl Fn(&Path) -> bool,
) -> Vec<DetectedEditor> {
    KNOWN_EDITORS
        .iter()
        .filter_map(|e| {
            let path = match os {
                HostOs::Windows => e.windows.iter().find_map(|(var, rel)| {
                    let path = Path::new(&env(var)?).join(rel);
                    exists(&path).then_some(path)
                }),
                HostOs::Linux => e.linux.iter().find_map(|name| find(name)),
                HostOs::Macos | HostOs::Other => None,
            }?;
            Some(DetectedEditor {
                name: e.name.to_owned(),
                path,
            })
        })
        .collect()
}

/// [`find_editors`] for this host.
pub fn system_editors() -> Vec<DetectedEditor> {
    let platform = SystemPlatform;
    find_editors(
        platform.host_info().os,
        |name| platform.find_executable(name),
        |var| std::env::var_os(var),
        Path::is_file,
    )
}

/// Opens `folder` in the editor at `program`, without waiting for it.
pub fn spawn_editor(program: &Path, folder: &Path) -> io::Result<()> {
    let mut command = Command::new(program);
    command
        .arg(folder)
        // Set when Vagrant Studio was started from a VS Code terminal or
        // extension; it makes Electron editors (VS Code, Cursor, VSCodium)
        // run as plain Node.js and exit without opening a window.
        .env_remove("ELECTRON_RUN_AS_NODE")
        .current_dir(folder)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    spawn_detached(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_editors_are_found_in_their_install_folders() {
        let env = |var: &str| match var {
            "LOCALAPPDATA" => Some(OsString::from(r"C:\Users\dev\AppData\Local")),
            "ProgramFiles" => Some(OsString::from(r"C:\Program Files")),
            _ => None,
        };
        let installed = [
            r"C:\Program Files\Microsoft VS Code\Code.exe",
            r"C:\Users\dev\AppData\Local\Programs\Zed\Zed.exe",
        ];
        // Tests also run on Linux, where `join` adds `/`.
        let windows = |p: &Path| p.to_string_lossy().replace('/', "\\");
        let found = find_editors(
            HostOs::Windows,
            |_| None,
            env,
            |p| installed.contains(&windows(p).as_str()),
        );
        let names: Vec<_> = found.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Visual Studio Code", "Zed"]);
        assert_eq!(windows(&found[0].path), installed[0]);
    }

    #[test]
    fn linux_editors_are_found_on_path() {
        let find = |name: &str| (name == "zeditor").then(|| PathBuf::from("/usr/bin/zeditor"));
        let found = find_editors(HostOs::Linux, find, |_| None, |_| false);
        assert_eq!(
            found,
            [DetectedEditor {
                name: "Zed".into(),
                path: "/usr/bin/zeditor".into()
            }]
        );
        assert!(find_editors(HostOs::Macos, |_| None, |_| None, |_| true).is_empty());
    }
}
