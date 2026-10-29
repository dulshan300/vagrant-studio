//! Readiness of Vagrant itself: is it installed, which version, and which
//! plugins does it have? Provider checks build on this in
//! `vagrant-studio-provider-registry`. Checks are read-only and never install
//! or start anything; findings explain how to fix problems.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tokio_util::sync::CancellationToken;
use vagrant_studio_platform::{HostOs, PlatformAdapter};

use crate::machine_readable::{self, PluginInfo};
use crate::{Invocation, Runner, VagrantCommand, detect_version, locate};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Ok,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub level: Level,
    pub message: String,
    /// How to resolve it, when there is something to do.
    pub fix: Option<String>,
}

impl Finding {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            level: Level::Ok,
            message: message.into(),
            fix: None,
        }
    }
    pub fn problem(level: Level, message: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            fix: Some(fix.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VagrantCheck {
    pub path: Option<String>,
    pub version: Option<String>,
    pub findings: Vec<Finding>,
}

/// Locates Vagrant and reads its version. Returns the executable only when it
/// could be run.
pub async fn check_vagrant(
    runner: &dyn Runner,
    platform: &dyn PlatformAdapter,
    vagrant_override: Option<&Path>,
) -> (VagrantCheck, Option<PathBuf>) {
    let Some(path) = locate(vagrant_override, platform) else {
        let fix = match vagrant_override {
            Some(p) => format!(
                "The configured path {} doesn't exist. Fix it in Settings, or clear it to search automatically.",
                p.display()
            ),
            None => install_hint(
                platform.host_info().os,
                "Vagrant",
                "https://developer.hashicorp.com/vagrant/install",
            ),
        };
        return (
            VagrantCheck {
                path: None,
                version: None,
                findings: vec![Finding::problem(
                    Level::Error,
                    "Vagrant was not found.",
                    fix,
                )],
            },
            None,
        );
    };
    let display = path.display().to_string();
    match detect_version(runner, &path).await {
        Ok(version) => (
            VagrantCheck {
                path: Some(display),
                findings: vec![Finding::ok(format!("Vagrant {version} is installed."))],
                version: Some(version),
            },
            Some(path),
        ),
        Err(e) => (
            VagrantCheck {
                path: Some(display),
                version: None,
                findings: vec![Finding::problem(
                    Level::Error,
                    format!("Vagrant was found but couldn't be run: {e}"),
                    "Reinstall Vagrant, or check that the configured path points to the Vagrant executable.",
                )],
            },
            None,
        ),
    }
}

/// `vagrant plugin list`; empty when it can't be read.
pub async fn plugin_list(runner: &dyn Runner, program: &Path) -> Vec<PluginInfo> {
    let mut output = String::new();
    let result = runner
        .run(
            Invocation {
                program: program.to_path_buf(),
                args: VagrantCommand::PluginList.args(),
                working_dir: std::env::temp_dir(),
                timeout: Some(Duration::from_secs(60)),
                env: vec![],
            },
            Box::new(|line| {
                output.push_str(&line.text);
                output.push('\n');
            }),
            CancellationToken::new(),
        )
        .await;
    if result.is_err() {
        return Vec::new();
    }
    machine_readable::parse_plugin_list(&machine_readable::parse(&output))
}

pub fn install_hint(os: HostOs, product: &str, url: &str) -> String {
    let os_name = match os {
        HostOs::Windows => "Windows",
        HostOs::Linux => "Linux",
        HostOs::Macos => "macOS",
        HostOs::Other => "your system",
    };
    format!("Install {product} for {os_name} from {url}, then refresh this page.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{LineSink, RunFuture};
    use crate::{OutputLine, RunResult, Stream, Termination};
    use vagrant_studio_platform::{Arch, HostInfo};

    struct FakePlatform(bool);
    impl PlatformAdapter for FakePlatform {
        fn host_info(&self) -> HostInfo {
            HostInfo {
                os: HostOs::Windows,
                arch: Arch::X86_64,
                is_wsl: false,
            }
        }
        fn find_executable(&self, name: &str) -> Option<PathBuf> {
            (self.0 && name == "vagrant").then(|| PathBuf::from("/bin/vagrant"))
        }
    }

    struct ScriptedRunner;
    impl Runner for ScriptedRunner {
        fn run<'a>(
            &'a self,
            inv: Invocation,
            mut on_line: LineSink<'a>,
            _: CancellationToken,
        ) -> RunFuture<'a> {
            let output = if inv.args == ["--version"] {
                "Vagrant 2.4.9".to_owned()
            } else {
                std::fs::read_to_string(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../fixtures/command-output/windows-vagrant-2.4.9/plugin-list.txt"
                ))
                .unwrap()
            };
            Box::pin(async move {
                for line in output.lines() {
                    on_line(OutputLine {
                        stream: Stream::Stdout,
                        text: line.to_owned(),
                    });
                }
                Ok(RunResult {
                    termination: Termination::Exited,
                    exit_code: Some(0),
                })
            })
        }
    }

    #[tokio::test]
    async fn installed_vagrant_reports_version_and_plugins() {
        let (check, program) = check_vagrant(&ScriptedRunner, &FakePlatform(true), None).await;
        assert_eq!(check.version.as_deref(), Some("2.4.9"));
        let plugins = plugin_list(&ScriptedRunner, &program.unwrap()).await;
        assert_eq!(plugins[0].name, "vagrant-vmware-desktop");
    }

    #[tokio::test]
    async fn missing_vagrant_is_explained() {
        let (check, program) = check_vagrant(&ScriptedRunner, &FakePlatform(false), None).await;
        assert_eq!((check.path, program), (None, None));
        assert_eq!(check.findings[0].level, Level::Error);
        assert!(
            check.findings[0]
                .fix
                .as_deref()
                .unwrap()
                .contains("developer.hashicorp.com")
        );
    }
}
