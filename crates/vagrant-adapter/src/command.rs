//! Typed Vagrant commands → argument vectors. User input is only ever passed
//! as separate arguments, never interpolated into a shell string.

use serde::{Deserialize, Serialize};

use crate::boxes::{BoxAdd, BoxRef, BoxSource};

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
    /// `box outdated --global`: checks every installed box's catalog.
    BoxOutdated,
    BoxAdd(BoxAdd),
    /// Updates one box from its catalog; `box update` without `--box` would
    /// update the box of the current environment instead.
    BoxUpdate {
        name: String,
        provider: String,
        architecture: Option<String>,
    },
    /// Removes exactly one installed box. Always `--force`: the UI asks for
    /// confirmation first and shows which machines use the box.
    BoxRemove(BoxRef),
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
            Self::BoxOutdated => vec![
                "box".into(),
                "outdated".into(),
                "--global".into(),
                "--machine-readable".into(),
            ],
            Self::BoxAdd(add) => {
                let mut args = vec!["box".to_owned(), "add".to_owned(), "--no-tty".to_owned()];
                push_option(&mut args, "--provider", add.provider.as_deref());
                push_option(&mut args, "--box-version", add.version.as_deref());
                push_option(&mut args, "--architecture", add.architecture.as_deref());
                push_option(&mut args, "--name", add.name.as_deref());
                args.push(match &add.source {
                    BoxSource::Catalog(s) | BoxSource::Url(s) => s.clone(),
                    BoxSource::File(p) => p.to_string_lossy().into_owned(),
                });
                args
            }
            Self::BoxUpdate {
                name,
                provider,
                architecture,
            } => {
                let mut args = vec!["box".to_owned(), "update".to_owned(), "--no-tty".to_owned()];
                push_option(&mut args, "--box", Some(name));
                push_option(&mut args, "--provider", Some(provider));
                push_option(&mut args, "--architecture", architecture.as_deref());
                args
            }
            Self::BoxRemove(b) => {
                let mut args = vec![
                    "box".to_owned(),
                    "remove".to_owned(),
                    "--force".to_owned(),
                    "--no-tty".to_owned(),
                ];
                push_option(&mut args, "--provider", Some(&b.provider));
                push_option(&mut args, "--box-version", Some(&b.version));
                push_option(&mut args, "--architecture", b.architecture.as_deref());
                args.push(b.name.clone());
                args
            }
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
        matches!(self, Self::Lifecycle { .. })
    }

    pub fn is_destructive(&self) -> bool {
        match self {
            Self::Lifecycle { action, .. } => action.is_destructive(),
            Self::BoxRemove(_) => true,
            _ => false,
        }
    }

    /// Whether the command may read or add boxes in the shared box store.
    pub fn uses_boxes(&self) -> bool {
        matches!(
            self,
            Self::BoxList
                | Self::BoxOutdated
                | Self::Lifecycle {
                    action: LifecycleAction::Up,
                    ..
                }
        ) || self.changes_boxes()
    }

    /// Whether the command adds, replaces or removes boxes, and so needs
    /// the box store to itself.
    pub fn changes_boxes(&self) -> bool {
        matches!(
            self,
            Self::BoxAdd(_) | Self::BoxUpdate { .. } | Self::BoxRemove(_)
        )
    }
}

fn push_option(args: &mut Vec<String>, flag: &str, value: Option<&str>) {
    if let Some(v) = value {
        args.push(flag.to_owned());
        args.push(v.to_owned());
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
        assert!(!lifecycle(LifecycleAction::Up, &[]).changes_boxes());
        assert!(VagrantCommand::BoxRemove(box_ref()).is_destructive());
        assert!(VagrantCommand::BoxRemove(box_ref()).changes_boxes());
        assert!(!VagrantCommand::BoxRemove(box_ref()).is_mutating());
        assert!(VagrantCommand::BoxOutdated.uses_boxes());
        assert!(!VagrantCommand::BoxOutdated.changes_boxes());
    }

    fn box_ref() -> BoxRef {
        serde_json::from_value(serde_json::json!({
            "name": "almalinux/9", "provider": "vmware_desktop",
            "version": "9.6.20250522", "architecture": "amd64",
        }))
        .unwrap()
    }

    #[test]
    fn box_commands_name_exactly_one_box() {
        assert_eq!(
            VagrantCommand::BoxRemove(box_ref()).args(),
            [
                "box",
                "remove",
                "--force",
                "--no-tty",
                "--provider",
                "vmware_desktop",
                "--box-version",
                "9.6.20250522",
                "--architecture",
                "amd64",
                "almalinux/9"
            ]
        );
        assert_eq!(
            VagrantCommand::BoxUpdate {
                name: "almalinux/9".into(),
                provider: "vmware_desktop".into(),
                architecture: None
            }
            .args(),
            [
                "box",
                "update",
                "--no-tty",
                "--box",
                "almalinux/9",
                "--provider",
                "vmware_desktop"
            ]
        );
        assert_eq!(
            VagrantCommand::BoxOutdated.args(),
            ["box", "outdated", "--global", "--machine-readable"]
        );
        let add = crate::boxes::BoxAdd {
            source: BoxSource::Catalog("bento/ubuntu-24.04".into()),
            provider: Some("virtualbox".into()),
            version: Some("202510.26.0".into()),
            architecture: None,
            name: None,
        };
        assert_eq!(
            VagrantCommand::BoxAdd(add).args(),
            [
                "box",
                "add",
                "--no-tty",
                "--provider",
                "virtualbox",
                "--box-version",
                "202510.26.0",
                "bento/ubuntu-24.04"
            ]
        );
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
