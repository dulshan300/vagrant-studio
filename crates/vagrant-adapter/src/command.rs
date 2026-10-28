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

/// Machine lifecycle commands, run with explicit machine names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleAction {
    Up,
    Halt,
    Reload,
    Suspend,
    Resume,
    Provision,
    Destroy,
}

impl LifecycleAction {
    pub fn verb(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Halt => "halt",
            Self::Reload => "reload",
            Self::Suspend => "suspend",
            Self::Resume => "resume",
            Self::Provision => "provision",
            Self::Destroy => "destroy",
        }
    }

    /// Needs a typed confirmation in the UI and a backend confirmation token.
    pub fn is_destructive(self) -> bool {
        matches!(self, Self::Destroy)
    }

    /// The state the targeted machines should be in once the command succeeds.
    pub fn expected_state(self) -> StateClass {
        match self {
            Self::Up | Self::Reload | Self::Resume | Self::Provision => StateClass::Running,
            Self::Halt => StateClass::Stopped,
            Self::Suspend => StateClass::Suspended,
            Self::Destroy => StateClass::NotCreated,
        }
    }
}

/// Provider-specific state ids grouped into what the UI shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateClass {
    Running,
    Stopped,
    Suspended,
    NotCreated,
    Unknown,
}

impl StateClass {
    /// Keep in sync with `machineState` in apps/desktop/src/lib/operations.ts.
    pub fn of(state: &str) -> Self {
        match state {
            "running" => Self::Running,
            "poweroff" | "not_running" | "shutoff" | "stopped" | "halted" | "aborted" => {
                Self::Stopped
            }
            "saved" | "suspended" | "paused" => Self::Suspended,
            "not_created" => Self::NotCreated,
            _ => Self::Unknown,
        }
    }
}

/// Commands Vagrant Studio runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VagrantCommand {
    Version,
    Status {
        machine: Option<MachineName>,
    },
    Validate,
    BoxList,
    PluginList,
    /// An empty `machines` list means Vagrant's default targeting, which
    /// skips `autostart: false` machines for `up`; callers name machines.
    /// `up` has no `--provider`: it fails for machines already created with
    /// another provider, so `VAGRANT_DEFAULT_PROVIDER` is used instead.
    /// `destroy` always gets `--force`: the UI asks for confirmation first.
    Lifecycle {
        action: LifecycleAction,
        machines: Vec<MachineName>,
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
            Self::Lifecycle { action, machines } => {
                let mut args = vec![action.verb().to_owned()];
                if *action == LifecycleAction::Destroy {
                    args.push("--force".into());
                }
                args.push("--no-tty".into());
                args.extend(machines.iter().map(|m| m.as_str().to_owned()));
                args
            }
        }
    }

    /// Whether the command changes machines (and so needs the environment lock).
    pub fn is_mutating(&self) -> bool {
        !matches!(
            self,
            Self::Version | Self::Status { .. } | Self::Validate | Self::BoxList | Self::PluginList
        )
    }

    pub fn is_destructive(&self) -> bool {
        matches!(self, Self::Lifecycle { action, .. } if action.is_destructive())
    }

    /// Whether the command may read or add boxes in the shared box store.
    pub fn uses_boxes(&self) -> bool {
        matches!(
            self,
            Self::BoxList
                | Self::Lifecycle {
                    action: LifecycleAction::Up,
                    ..
                }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(name: &str) -> Option<MachineName> {
        Some(MachineName::try_from(name.to_owned()).unwrap())
    }

    fn lifecycle(action: LifecycleAction, names: &[&str]) -> VagrantCommand {
        VagrantCommand::Lifecycle {
            action,
            machines: names.iter().filter_map(|n| m(n)).collect(),
        }
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
            lifecycle(LifecycleAction::Up, &["web", "db"]).args(),
            ["up", "--no-tty", "web", "db"]
        );
        assert_eq!(
            lifecycle(LifecycleAction::Destroy, &["web"]).args(),
            ["destroy", "--force", "--no-tty", "web"]
        );
        assert_eq!(
            lifecycle(LifecycleAction::Halt, &[]).args(),
            ["halt", "--no-tty"]
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
        assert!(lifecycle(LifecycleAction::Halt, &[]).is_mutating());
        assert!(lifecycle(LifecycleAction::Destroy, &[]).is_destructive());
        assert!(!lifecycle(LifecycleAction::Halt, &[]).is_destructive());
        assert!(lifecycle(LifecycleAction::Up, &[]).uses_boxes());
        assert!(!lifecycle(LifecycleAction::Reload, &[]).uses_boxes());
    }

    #[test]
    fn lifecycle_actions_use_vagrant_verbs_and_snake_case_json() {
        assert_eq!(LifecycleAction::Provision.verb(), "provision");
        assert_eq!(
            serde_json::from_str::<LifecycleAction>("\"suspend\"").unwrap(),
            LifecycleAction::Suspend
        );
    }

    #[test]
    fn classifies_provider_states() {
        assert_eq!(StateClass::of("poweroff"), StateClass::Stopped);
        assert_eq!(StateClass::of("not_running"), StateClass::Stopped);
        assert_eq!(StateClass::of("saved"), StateClass::Suspended);
        assert_eq!(StateClass::of("not_created"), StateClass::NotCreated);
        assert_eq!(StateClass::of("host_state_unknown"), StateClass::Unknown);
        assert_eq!(
            LifecycleAction::Destroy.expected_state(),
            StateClass::NotCreated
        );
    }
}
