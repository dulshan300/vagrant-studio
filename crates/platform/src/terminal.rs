//! Opening a command in a new terminal window.
//!
//! The terminal is chosen per host OS from what is installed. The command runs
//! in the given working directory: the spawned process inherits it, so no path
//! is ever passed through a terminal's own argument parsing.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::{HostOs, PlatformAdapter, SystemPlatform};

/// A terminal to start, with the command to run inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalLaunch {
    /// Display name of the terminal, e.g. "Windows Terminal".
    pub terminal: &'static str,
    pub program: PathBuf,
    pub args: Vec<OsString>,
    /// Windows only: start `program` in a new console window of its own.
    pub new_console: bool,
}

/// Linux terminals in order of preference, with the arguments that put the
/// command after them. `x-terminal-emulator` is Debian/Ubuntu's configured
/// default; the rest are tried when it is missing.
const LINUX_TERMINALS: &[(&str, &str, &[&str])] = &[
    ("x-terminal-emulator", "the default terminal", &["-e"]),
    ("gnome-terminal", "GNOME Terminal", &["--"]),
    ("ptyxis", "Ptyxis", &["--new-window", "--"]),
    ("konsole", "Konsole", &["-e"]),
    ("xfce4-terminal", "Xfce Terminal", &["-x"]),
    ("kitty", "kitty", &[]),
    ("alacritty", "Alacritty", &["-e"]),
    ("xterm", "xterm", &["-e"]),
];

/// Plans how to run `program args` in a new terminal window. `find` locates
/// an executable by bare name. Returns `None` when no known terminal is
/// installed, or when the command can't be passed through safely.
///
/// `title` is shown by Windows Terminal; it must not contain `;`, which
/// Windows Terminal treats as a command separator anywhere on its command line.
pub fn terminal_launch(
    os: HostOs,
    find: impl Fn(&str) -> Option<PathBuf>,
    title: &str,
    program: &Path,
    args: &[OsString],
) -> Option<TerminalLaunch> {
    match os {
        HostOs::Windows => Some(windows_launch(find, title, program, args)),
        HostOs::Linux => LINUX_TERMINALS.iter().find_map(|(name, label, prefix)| {
            let terminal = find(name)?;
            let mut all: Vec<OsString> = prefix.iter().map(OsString::from).collect();
            all.push(program.as_os_str().to_owned());
            all.extend(args.iter().cloned());
            Some(TerminalLaunch {
                terminal: label,
                program: terminal,
                args: all,
                new_console: false,
            })
        }),
        HostOs::Macos | HostOs::Other => None,
    }
}

fn windows_launch(
    find: impl Fn(&str) -> Option<PathBuf>,
    title: &str,
    program: &Path,
    args: &[OsString],
) -> TerminalLaunch {
    let separator = |s: &std::ffi::OsStr| s.to_string_lossy().contains(';');
    let wt = find("wt").filter(|_| {
        !separator(title.as_ref())
            && !separator(program.as_os_str())
            && !args.iter().any(|a| separator(a))
    });
    match wt {
        // `-d .` starts the tab in wt's own working directory, which is ours.
        Some(wt) => {
            let mut all: Vec<OsString> = ["-w", "new", "new-tab", "--title", title, "-d", "."]
                .iter()
                .map(OsString::from)
                .collect();
            all.push(program.as_os_str().to_owned());
            all.extend(args.iter().cloned());
            TerminalLaunch {
                terminal: "Windows Terminal",
                program: wt,
                args: all,
                new_console: false,
            }
        }
        None => TerminalLaunch {
            terminal: "a console window",
            program: program.to_owned(),
            args: args.to_vec(),
            new_console: true,
        },
    }
}

/// [`terminal_launch`] for this host. Windows Terminal is an app execution
/// alias in `%LOCALAPPDATA%\Microsoft\WindowsApps`, which is usually but not
/// always on `PATH`, so that folder is checked too.
pub fn system_terminal_launch(
    title: &str,
    program: &Path,
    args: &[OsString],
) -> Option<TerminalLaunch> {
    let platform = SystemPlatform;
    let os = platform.host_info().os;
    let find = |name: &str| {
        platform.find_executable(name).or_else(|| {
            let local = std::env::var_os("LOCALAPPDATA").filter(|_| os == HostOs::Windows)?;
            let alias = Path::new(&local)
                .join(r"Microsoft\WindowsApps")
                .join(format!("{name}.exe"));
            alias.is_file().then_some(alias)
        })
    };
    terminal_launch(os, find, title, program, args)
}

/// Starts the terminal in `working_dir` without waiting for it. Its output is
/// not captured; the window belongs to the user.
pub fn spawn_terminal(launch: &TerminalLaunch, working_dir: &Path) -> io::Result<()> {
    let mut command = Command::new(&launch.program);
    command
        .args(&launch.args)
        .current_dir(working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    if launch.new_console {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        command.creation_flags(CREATE_NEW_CONSOLE);
    }
    let mut child = command.spawn()?;
    // Reap the process when it exits so it doesn't linger as a zombie on Linux.
    std::thread::spawn(move || child.wait());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vagrant() -> PathBuf {
        PathBuf::from(r"C:\Program Files\Vagrant\bin\vagrant.exe")
    }

    fn ssh(machine: &str) -> Vec<OsString> {
        vec!["ssh".into(), machine.into()]
    }

    fn strings(launch: &TerminalLaunch) -> Vec<String> {
        launch
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    fn installed(names: &'static [&'static str]) -> impl Fn(&str) -> Option<PathBuf> {
        move |name| {
            names
                .contains(&name)
                .then(|| PathBuf::from(format!("/usr/bin/{name}")))
        }
    }

    #[test]
    fn windows_terminal_opens_a_new_window_in_the_working_directory() {
        let wt = |name: &str| (name == "wt").then(|| PathBuf::from(r"C:\Apps\wt.exe"));
        let launch = terminal_launch(
            HostOs::Windows,
            wt,
            "web · vagrant ssh",
            &vagrant(),
            &ssh("web"),
        )
        .unwrap();
        assert_eq!(launch.terminal, "Windows Terminal");
        assert_eq!(launch.program, PathBuf::from(r"C:\Apps\wt.exe"));
        assert_eq!(
            strings(&launch),
            [
                "-w",
                "new",
                "new-tab",
                "--title",
                "web · vagrant ssh",
                "-d",
                ".",
                r"C:\Program Files\Vagrant\bin\vagrant.exe",
                "ssh",
                "web"
            ]
        );
        assert!(!launch.new_console);
    }

    #[test]
    fn windows_falls_back_to_a_console_window() {
        let launch =
            terminal_launch(HostOs::Windows, |_| None, "t", &vagrant(), &ssh("web")).unwrap();
        assert_eq!(launch.program, vagrant());
        assert_eq!(strings(&launch), ["ssh", "web"]);
        assert!(launch.new_console);

        // Windows Terminal would split the command at a `;`.
        let wt = |_: &str| Some(PathBuf::from("wt.exe"));
        let odd = PathBuf::from(r"C:\a;b\vagrant.exe");
        let launch = terminal_launch(HostOs::Windows, wt, "t", &odd, &ssh("web")).unwrap();
        assert!(launch.new_console);
    }

    #[test]
    fn linux_uses_the_first_installed_terminal() {
        let program = PathBuf::from("/usr/bin/vagrant");
        let launch = terminal_launch(
            HostOs::Linux,
            installed(&["xterm", "gnome-terminal"]),
            "t",
            &program,
            &ssh("db"),
        )
        .unwrap();
        assert_eq!(launch.terminal, "GNOME Terminal");
        assert_eq!(strings(&launch), ["--", "/usr/bin/vagrant", "ssh", "db"]);

        let launch = terminal_launch(
            HostOs::Linux,
            installed(&["kitty"]),
            "t",
            &program,
            &ssh("db"),
        )
        .unwrap();
        assert_eq!(strings(&launch), ["/usr/bin/vagrant", "ssh", "db"]);

        assert_eq!(
            terminal_launch(HostOs::Linux, |_| None, "t", &program, &ssh("db")),
            None
        );
    }
}
