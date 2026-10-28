//! Typed Vagrant commands → argument vectors. User input is only ever passed
//! as separate arguments, never interpolated into a shell string.

use serde::{Deserialize, Serialize};

/// Vagrant machine names: letters, digits, `-`, `_`, `.`, and not starting
/// with `-` (so a name can never be read as an option).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MachineName(String);

impl MachineName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for MachineName {
    type Error = String;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        let valid = !name.is_empty()
            && name.len() <= 100
            && !name.starts_with('-')
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
        if valid {
            Ok(Self(name))
        } else {
            Err(format!("\"{name}\" is not a valid Vagrant machine name"))
        }
    }
}

impl From<MachineName> for String {
    fn from(name: MachineName) -> Self {
        name.0
    }
}

/// Commands Vagrant Studio runs. Lifecycle variants are executed from Phase 4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VagrantCommand {
    Version,
    Status {
        machine: Option<MachineName>,
    },
    Validate,
    BoxList,
    PluginList,
    Up {
        machine: Option<MachineName>,
        provider: Option<String>,
    },
    Halt {
        machine: Option<MachineName>,
    },
    Reload {
        machine: Option<MachineName>,
    },
    Suspend {
        machine: Option<MachineName>,
    },
    Resume {
        machine: Option<MachineName>,
    },
    Provision {
        machine: Option<MachineName>,
    },
    /// Always non-interactive (`--force`): the UI asks for confirmation first.
    Destroy {
        machine: Option<MachineName>,
    },
}

impl VagrantCommand {
    /// Arguments after the executable.
    ///
    /// `--no-tty` makes interactive commands non-interactive. It must NOT be
    /// combined with `--machine-readable`: both select Vagrant's UI and the
    /// last one wins, so `--no-tty` would switch parseable output back to
    /// human text (verified with Vagrant 2.4.9 on Windows).
    pub fn args(&self) -> Vec<String> {
        fn with_machine(mut args: Vec<&str>, machine: &Option<MachineName>) -> Vec<String> {
            args.push("--no-tty");
            let mut out: Vec<String> = args.into_iter().map(str::to_owned).collect();
            if let Some(m) = machine {
                out.push(m.as_str().to_owned());
            }
            out
        }
        match self {
            Self::Version => vec!["--version".into()],
            Self::Status { machine } => {
                let mut args = vec!["status".to_owned(), "--machine-readable".to_owned()];
                if let Some(m) = machine {
                    args.push(m.as_str().to_owned());
                }
                args
            }
            Self::Validate => vec!["validate".into(), "--no-tty".into()],
            Self::BoxList => vec!["box".into(), "list".into(), "--machine-readable".into()],
            Self::PluginList => vec!["plugin".into(), "list".into(), "--machine-readable".into()],
            Self::Up { machine, provider } => {
                let mut args = with_machine(vec!["up"], &None);
                if let Some(p) = provider {
                    args.push(format!("--provider={p}"));
                }
                if let Some(m) = machine {
                    args.push(m.as_str().to_owned());
                }
                args
            }
            Self::Halt { machine } => with_machine(vec!["halt"], machine),
            Self::Reload { machine } => with_machine(vec!["reload"], machine),
            Self::Suspend { machine } => with_machine(vec!["suspend"], machine),
            Self::Resume { machine } => with_machine(vec!["resume"], machine),
            Self::Provision { machine } => with_machine(vec!["provision"], machine),
            Self::Destroy { machine } => with_machine(vec!["destroy", "--force"], machine),
        }
    }

    /// Whether the command changes machines (and so needs the environment lock).
    pub fn is_mutating(&self) -> bool {
        !matches!(
            self,
            Self::Version | Self::Status { .. } | Self::Validate | Self::BoxList | Self::PluginList
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(name: &str) -> Option<MachineName> {
        Some(MachineName::try_from(name.to_owned()).unwrap())
    }

    #[test]
    fn machine_names_cannot_be_options_or_contain_shell_syntax() {
        for bad in ["", "-f", "--provider=x", "a b", "a;rm", "web$(x)", "../x"] {
            assert!(MachineName::try_from(bad.to_owned()).is_err(), "{bad}");
        }
        assert!(MachineName::try_from("web-01.app_x".to_owned()).is_ok());
    }

    #[test]
    fn builds_argument_vectors() {
        assert_eq!(VagrantCommand::Version.args(), ["--version"]);
        assert_eq!(
            VagrantCommand::Status { machine: None }.args(),
            ["status", "--machine-readable"]
        );
        assert_eq!(
            VagrantCommand::Status { machine: m("db") }.args(),
            ["status", "--machine-readable", "db"]
        );
        assert_eq!(
            VagrantCommand::Up {
                machine: m("web"),
                provider: Some("vmware_desktop".into())
            }
            .args(),
            ["up", "--no-tty", "--provider=vmware_desktop", "web"]
        );
        assert_eq!(
            VagrantCommand::Destroy { machine: m("web") }.args(),
            ["destroy", "--force", "--no-tty", "web"]
        );
        assert_eq!(
            VagrantCommand::BoxList.args(),
            ["box", "list", "--machine-readable"]
        );
    }

    #[test]
    fn machine_readable_commands_never_get_no_tty() {
        for cmd in [
            VagrantCommand::Status { machine: m("web") },
            VagrantCommand::BoxList,
            VagrantCommand::PluginList,
        ] {
            let args = cmd.args();
            assert!(args.contains(&"--machine-readable".to_owned()));
            assert!(!args.contains(&"--no-tty".to_owned()), "{args:?}");
        }
    }

    #[test]
    fn classifies_mutating_commands() {
        assert!(!VagrantCommand::Status { machine: None }.is_mutating());
        assert!(!VagrantCommand::Validate.is_mutating());
        assert!(VagrantCommand::Halt { machine: None }.is_mutating());
        assert!(VagrantCommand::Destroy { machine: None }.is_mutating());
    }
}
