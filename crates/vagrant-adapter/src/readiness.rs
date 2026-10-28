//! Host readiness: is Vagrant installed, which plugins does it have, and are
//! the supported providers usable? Checks are read-only and never install or
//! start anything; findings explain how to fix problems.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tokio_util::sync::CancellationToken;
use vagrant_studio_platform::{HostOs, PlatformAdapter};

use crate::machine_readable::{self, PluginInfo};
use crate::{Invocation, Runner, VagrantCommand, detect_version, locate};

const VMWARE_UTILITY_ADDR: &str = "127.0.0.1:9922";
const VMWARE_PLUGIN: &str = "vagrant-vmware-desktop";

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
    fn ok(message: impl Into<String>) -> Self {
        Self {
            level: Level::Ok,
            message: message.into(),
            fix: None,
        }
    }
    fn problem(level: Level, message: impl Into<String>, fix: impl Into<String>) -> Self {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderState {
    /// All checks passed.
    Ready,
    /// Installed, but something needs attention before machines can run.
    Attention,
    /// Not installed or not usable.
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCheck {
    /// Vagrant provider name, e.g. `virtualbox`, `vmware_desktop`.
    pub id: &'static str,
    pub name: &'static str,
    pub state: ProviderState,
    pub version: Option<String>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostReadiness {
    pub vagrant: VagrantCheck,
    pub plugins: Vec<PluginInfo>,
    pub providers: Vec<ProviderCheck>,
}

/// Something that can test whether a local TCP service accepts connections.
pub trait PortProbe: Send + Sync {
    fn is_open<'a>(
        &'a self,
        addr: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + 'a>>;
}

pub struct TcpProbe;

impl PortProbe for TcpProbe {
    fn is_open<'a>(
        &'a self,
        addr: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            matches!(
                tokio::time::timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(addr))
                    .await,
                Ok(Ok(_))
            )
        })
    }
}

pub struct Checker<'a> {
    pub runner: &'a dyn Runner,
    pub platform: &'a dyn PlatformAdapter,
    pub probe: &'a dyn PortProbe,
    pub vagrant_override: Option<&'a Path>,
}

impl Checker<'_> {
    pub async fn check(&self) -> HostReadiness {
        let os = self.platform.host_info().os;
        let (vagrant, program) = self.check_vagrant(os).await;
        let plugins = match &program {
            Some(p) => self.plugins(p).await,
            None => Vec::new(),
        };
        let providers = vec![
            self.virtualbox(os).await,
            self.vmware(os, program.is_some(), &plugins).await,
        ];
        HostReadiness {
            vagrant,
            plugins,
            providers,
        }
    }

    async fn check_vagrant(&self, os: HostOs) -> (VagrantCheck, Option<PathBuf>) {
        let Some(path) = locate(self.vagrant_override, self.platform) else {
            let fix = match self.vagrant_override {
                Some(p) => format!(
                    "The configured path {} doesn't exist. Fix it in Settings, or clear it to search automatically.",
                    p.display()
                ),
                None => install_hint(
                    os,
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
        match detect_version(self.runner, &path).await {
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

    async fn plugins(&self, program: &Path) -> Vec<PluginInfo> {
        let mut output = String::new();
        let result = self
            .runner
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

    async fn virtualbox(&self, os: HostOs) -> ProviderCheck {
        let exe = self.find(
            "VBoxManage",
            &[
                (
                    HostOs::Windows,
                    r"%ProgramFiles%\Oracle\VirtualBox\VBoxManage.exe",
                ),
                (HostOs::Linux, "/usr/bin/VBoxManage"),
                (HostOs::Linux, "/usr/lib/virtualbox/VBoxManage"),
            ],
            os,
        );
        let Some(exe) = exe else {
            return ProviderCheck {
                id: "virtualbox",
                name: "VirtualBox",
                state: ProviderState::Unavailable,
                version: None,
                findings: vec![Finding::problem(
                    Level::Error,
                    "VirtualBox is not installed (VBoxManage not found).",
                    install_hint(
                        os,
                        "VirtualBox",
                        "https://www.virtualbox.org/wiki/Downloads",
                    ),
                )],
            };
        };
        let version = self.first_line(&exe, &["--version"]).await.map(|v| {
            // e.g. "7.1.18r173720"
            v.split('r').next().unwrap_or(&v).trim().to_owned()
        });
        let mut findings = vec![Finding::ok(format!(
            "VBoxManage found at {}.",
            exe.display()
        ))];
        let state = match &version {
            Some(v) => {
                findings.push(Finding::ok(format!("VirtualBox {v}.")));
                ProviderState::Ready
            }
            None => {
                findings.push(Finding::problem(
                    Level::Warning,
                    "VBoxManage didn't report a version.",
                    "Reinstall VirtualBox if machines fail to start.",
                ));
                ProviderState::Attention
            }
        };
        ProviderCheck {
            id: "virtualbox",
            name: "VirtualBox",
            state,
            version,
            findings,
        }
    }

    async fn vmware(&self, os: HostOs, vagrant_ok: bool, plugins: &[PluginInfo]) -> ProviderCheck {
        let mut findings = Vec::new();
        let vmrun = self.find(
            "vmrun",
            &[
                (
                    HostOs::Windows,
                    r"%ProgramFiles(x86)%\VMware\VMware Workstation\vmrun.exe",
                ),
                (
                    HostOs::Windows,
                    r"%ProgramFiles%\VMware\VMware Workstation\vmrun.exe",
                ),
                (HostOs::Linux, "/usr/bin/vmrun"),
            ],
            os,
        );
        match &vmrun {
            Some(p) => findings.push(Finding::ok(format!("vmrun found at {}.", p.display()))),
            None => findings.push(Finding::problem(
                Level::Error,
                "VMware Workstation is not installed (vmrun not found).",
                install_hint(
                    os,
                    "VMware Workstation",
                    "https://www.vmware.com/products/desktop-hypervisor",
                ),
            )),
        }

        let plugin = plugins.iter().find(|p| p.name == VMWARE_PLUGIN);
        match (plugin, vagrant_ok) {
            (Some(p), _) => findings.push(Finding::ok(format!(
                "Vagrant plugin {VMWARE_PLUGIN} {} is installed.",
                p.version.as_deref().unwrap_or("")
            ))),
            (None, true) => findings.push(Finding::problem(
                Level::Error,
                format!("The Vagrant plugin {VMWARE_PLUGIN} is not installed."),
                format!("Run: vagrant plugin install {VMWARE_PLUGIN}"),
            )),
            (None, false) => {}
        }

        let utility = self.probe.is_open(VMWARE_UTILITY_ADDR).await;
        if utility {
            findings.push(Finding::ok("The Vagrant VMware Utility is running."));
        } else {
            let fix = match os {
                HostOs::Windows => {
                    "Install the Vagrant VMware Utility if needed, then start its service: open Services and start “Vagrant VMware Utility”, or run `net start VagrantVMware` as Administrator."
                }
                _ => {
                    "Install the Vagrant VMware Utility if needed, then start it: `sudo systemctl start vagrant-vmware-utility`."
                }
            };
            findings.push(Finding::problem(
                Level::Error,
                format!("The Vagrant VMware Utility is not reachable on {VMWARE_UTILITY_ADDR}."),
                format!("{fix} Download: https://developer.hashicorp.com/vagrant/install/vmware"),
            ));
        }

        let state = if vmrun.is_none() {
            ProviderState::Unavailable
        } else if findings.iter().any(|f| f.level != Level::Ok) {
            ProviderState::Attention
        } else {
            ProviderState::Ready
        };
        ProviderCheck {
            id: "vmware_desktop",
            name: "VMware Desktop",
            state,
            version: None,
            findings,
        }
    }

    /// PATH first, then well-known install locations for this OS.
    fn find(&self, name: &str, known: &[(HostOs, &str)], os: HostOs) -> Option<PathBuf> {
        self.platform.find_executable(name).or_else(|| {
            known
                .iter()
                .filter(|(o, _)| *o == os)
                .map(|(_, p)| PathBuf::from(expand_env(p)))
                .find(|p| p.is_file())
        })
    }

    async fn first_line(&self, program: &Path, args: &[&str]) -> Option<String> {
        let mut first: Option<String> = None;
        let result = self
            .runner
            .run(
                Invocation {
                    program: program.to_path_buf(),
                    args: args.iter().map(|a| (*a).to_owned()).collect(),
                    working_dir: std::env::temp_dir(),
                    timeout: Some(Duration::from_secs(30)),
                    env: vec![],
                },
                Box::new(|line| {
                    if first.is_none() && !line.text.trim().is_empty() {
                        first = Some(line.text.trim().to_owned());
                    }
                }),
                CancellationToken::new(),
            )
            .await
            .ok()?;
        result.success().then_some(first).flatten()
    }
}

fn install_hint(os: HostOs, product: &str, url: &str) -> String {
    let os_name = match os {
        HostOs::Windows => "Windows",
        HostOs::Linux => "Linux",
        HostOs::Macos => "macOS",
        HostOs::Other => "your system",
    };
    format!("Install {product} for {os_name} from {url}, then refresh this page.")
}

/// Expands `%VAR%` (Windows-style) references; unknown variables are left as-is.
fn expand_env(path: &str) -> String {
    let mut out = String::new();
    let mut rest = path;
    while let Some(start) = rest.find('%') {
        let Some(len) = rest[start + 1..].find('%') else {
            break;
        };
        let name = &rest[start + 1..start + 1 + len];
        out.push_str(&rest[..start]);
        match std::env::var(name) {
            Ok(value) => out.push_str(&value),
            Err(_) => out.push_str(&rest[start..start + len + 2]),
        }
        rest = &rest[start + len + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{LineSink, RunFuture};
    use crate::{OutputLine, RunResult, Stream, Termination};
    use vagrant_studio_platform::{Arch, HostInfo};

    struct FakePlatform {
        os: HostOs,
        found: Vec<&'static str>,
    }
    impl PlatformAdapter for FakePlatform {
        fn host_info(&self) -> HostInfo {
            HostInfo {
                os: self.os,
                arch: Arch::X86_64,
                is_wsl: false,
            }
        }
        fn find_executable(&self, name: &str) -> Option<PathBuf> {
            self.found
                .contains(&name)
                .then(|| PathBuf::from(format!("/bin/{name}")))
        }
    }

    /// Answers by program/args with fixture output.
    struct ScriptedRunner;
    impl Runner for ScriptedRunner {
        fn run<'a>(
            &'a self,
            inv: Invocation,
            mut on_line: LineSink<'a>,
            _: CancellationToken,
        ) -> RunFuture<'a> {
            let program = inv.program.display().to_string();
            let output = if program.ends_with("vagrant") && inv.args == ["--version"] {
                "Vagrant 2.4.9".to_owned()
            } else if program.ends_with("vagrant")
                && inv.args.first().map(String::as_str) == Some("plugin")
            {
                std::fs::read_to_string(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../fixtures/command-output/windows-vagrant-2.4.9/plugin-list.txt"
                ))
                .unwrap()
            } else if program.ends_with("VBoxManage") {
                "7.1.18r173720".to_owned()
            } else {
                String::new()
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

    struct Probe(bool);
    impl PortProbe for Probe {
        fn is_open<'a>(
            &'a self,
            _: &'a str,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + 'a>> {
            let open = self.0;
            Box::pin(async move { open })
        }
    }

    async fn check(found: Vec<&'static str>, utility: bool) -> HostReadiness {
        Checker {
            runner: &ScriptedRunner,
            platform: &FakePlatform {
                os: HostOs::Windows,
                found,
            },
            probe: &Probe(utility),
            vagrant_override: None,
        }
        .check()
        .await
    }

    #[tokio::test]
    async fn everything_installed_and_running() {
        let r = check(vec!["vagrant", "VBoxManage", "vmrun"], true).await;
        assert_eq!(r.vagrant.version.as_deref(), Some("2.4.9"));
        assert_eq!(r.plugins[0].name, "vagrant-vmware-desktop");
        let vb = &r.providers[0];
        assert_eq!(
            (vb.id, vb.state, vb.version.as_deref()),
            ("virtualbox", ProviderState::Ready, Some("7.1.18"))
        );
        let vm = &r.providers[1];
        assert_eq!((vm.id, vm.state), ("vmware_desktop", ProviderState::Ready));
    }

    #[tokio::test]
    async fn stopped_vmware_utility_needs_attention_with_a_fix() {
        let r = check(vec!["vagrant", "vmrun"], false).await;
        let vm = &r.providers[1];
        assert_eq!(vm.state, ProviderState::Attention);
        let problem = vm
            .findings
            .iter()
            .find(|f| f.level == Level::Error)
            .unwrap();
        assert!(problem.message.contains("127.0.0.1:9922"));
        assert!(problem.fix.as_deref().unwrap().contains("VagrantVMware"));
    }

    #[tokio::test]
    async fn missing_vagrant_and_providers_are_explained() {
        let r = check(vec![], false).await;
        assert_eq!(r.vagrant.path, None);
        assert_eq!(r.vagrant.findings[0].level, Level::Error);
        assert!(
            r.vagrant.findings[0]
                .fix
                .as_deref()
                .unwrap()
                .contains("developer.hashicorp.com")
        );
        assert!(r.plugins.is_empty());
        // Unless the real machine has them installed in the default locations.
        if !Path::new(&expand_env(
            r"%ProgramFiles%\Oracle\VirtualBox\VBoxManage.exe",
        ))
        .is_file()
        {
            assert_eq!(r.providers[0].state, ProviderState::Unavailable);
        }
    }

    #[test]
    fn expands_windows_style_variables() {
        // SAFETY of test isolation: uses a variable name unique to this test.
        let path = expand_env(r"%VAGRANT_STUDIO_SURELY_UNSET%\x");
        assert_eq!(path, r"%VAGRANT_STUDIO_SURELY_UNSET%\x");
        assert_eq!(expand_env("/usr/bin/vmrun"), "/usr/bin/vmrun");
        assert_eq!(expand_env("50%"), "50%");
    }
}
