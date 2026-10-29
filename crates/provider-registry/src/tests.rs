use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use tokio_util::sync::CancellationToken;
use vagrant_studio_platform::{Arch, HostInfo, HostOs, PlatformAdapter};
use vagrant_studio_vagrant_adapter::runner::{LineSink, RunFuture};
use vagrant_studio_vagrant_adapter::{
    Invocation, OutputLine, RunResult, Runner, Stream, Termination,
};

use super::*;

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

/// Answers by program name and first argument: (exit code, stdout, stderr).
struct ScriptedRunner {
    virtualbox: &'static str,
    docker: (i32, &'static str, &'static str),
}

impl Runner for ScriptedRunner {
    fn run<'a>(
        &'a self,
        inv: Invocation,
        mut on_line: LineSink<'a>,
        _: CancellationToken,
    ) -> RunFuture<'a> {
        let program = inv.program.display().to_string();
        let (code, stdout, stderr) = if program.ends_with("vagrant") && inv.args == ["--version"] {
            (0, "Vagrant 2.4.9".to_owned(), "")
        } else if program.ends_with("vagrant") {
            let fixture = std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/command-output/windows-vagrant-2.4.9/plugin-list.txt"
            ))
            .unwrap();
            (0, fixture, "")
        } else if program.ends_with("VBoxManage") {
            (0, self.virtualbox.to_owned(), "")
        } else if program.ends_with("docker") {
            let (code, out, err) = self.docker;
            (code, out.to_owned(), err)
        } else {
            (1, String::new(), "")
        };
        Box::pin(async move {
            for (stream, text) in [(Stream::Stdout, stdout.as_str()), (Stream::Stderr, stderr)] {
                for line in text.lines() {
                    on_line(OutputLine {
                        stream,
                        text: line.to_owned(),
                    });
                }
            }
            Ok(RunResult {
                termination: Termination::Exited,
                exit_code: Some(code),
            })
        })
    }
}

struct FakeProbe {
    utility: bool,
    paths: Vec<&'static str>,
    hypervisor: Option<bool>,
}

impl HostProbe for FakeProbe {
    fn is_open<'a>(&'a self, _: &'a str) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        let open = self.utility;
        Box::pin(async move { open })
    }
    fn path_exists(&self, path: &Path) -> bool {
        self.paths.iter().any(|p| Path::new(p) == path)
    }
    fn hypervisor_present(&self) -> Option<bool> {
        self.hypervisor
    }
}

struct Host {
    os: HostOs,
    found: Vec<&'static str>,
    virtualbox: &'static str,
    docker: (i32, &'static str, &'static str),
    probe: FakeProbe,
}

impl Default for Host {
    /// Windows with everything installed and running.
    fn default() -> Self {
        Self {
            os: HostOs::Windows,
            found: vec!["vagrant", "VBoxManage", "vmrun", "docker"],
            virtualbox: "7.1.18r173720",
            docker: (0, "29.7.2", ""),
            probe: FakeProbe {
                utility: true,
                paths: vec![],
                hypervisor: Some(false),
            },
        }
    }
}

impl Host {
    async fn check(self) -> HostReadiness {
        Registry::default()
            .check_host(
                &ScriptedRunner {
                    virtualbox: self.virtualbox,
                    docker: self.docker,
                },
                &FakePlatform {
                    os: self.os,
                    found: self.found,
                },
                &self.probe,
                None,
            )
            .await
    }
}

fn provider<'a>(r: &'a HostReadiness, id: &str) -> &'a ProviderReport {
    r.providers.iter().find(|p| p.id == id).unwrap()
}

fn problem(p: &ProviderReport) -> &Finding {
    p.findings.iter().find(|f| f.level != Level::Ok).unwrap()
}

#[tokio::test]
async fn everything_installed_and_running() {
    let r = Host::default().check().await;
    assert_eq!(r.vagrant.version.as_deref(), Some("2.4.9"));
    assert_eq!(r.plugins[0].name, "vagrant-vmware-desktop");
    let ids: Vec<_> = r.providers.iter().map(|p| p.id).collect();
    assert_eq!(ids, ["virtualbox", "vmware_desktop", "docker"]);
    for p in &r.providers {
        assert_eq!(p.state, ProviderState::Ready, "{}: {:?}", p.id, p.findings);
        // Only VirtualBox has passed the full test plan, on Windows.
        let verified = p.id == "virtualbox";
        assert_eq!(p.support == SupportLevel::Verified, verified, "{}", p.id);
    }
    let vb = provider(&r, "virtualbox");
    assert_eq!(vb.version.as_deref(), Some("7.1.18"));
    assert!(
        vb.findings
            .iter()
            .any(|f| f.message == "Vagrant 2.4.9 supports VirtualBox 7.1.18.")
    );
    assert!(vb.capabilities.contains(&Capability::LifecycleSuspend));

    let docker = provider(&r, "docker");
    assert_eq!(
        (docker.kind, docker.version.as_deref()),
        (ProviderKind::Container, Some("29.7.2"))
    );
    assert!(docker.capabilities.contains(&Capability::LifecycleBasic));
    assert!(!docker.capabilities.contains(&Capability::LifecycleSuspend));
}

#[tokio::test]
async fn missing_providers_report_no_capabilities() {
    let r = Host {
        found: vec!["vagrant"],
        ..Host::default()
    }
    .check()
    .await;
    for p in &r.providers {
        assert_eq!(p.state, ProviderState::Unavailable, "{}", p.id);
        assert_eq!(p.support, SupportLevel::Unavailable);
        assert!(p.capabilities.is_empty());
        assert!(problem(p).fix.as_deref().unwrap().contains("Install"));
    }
}

#[tokio::test]
async fn missing_vagrant_skips_plugin_findings() {
    let r = Host {
        found: vec!["vmrun"],
        ..Host::default()
    }
    .check()
    .await;
    assert_eq!(r.vagrant.findings[0].level, Level::Error);
    assert!(r.plugins.is_empty());
    let vm = provider(&r, "vmware_desktop");
    assert_eq!(vm.state, ProviderState::Ready);
    assert!(!vm.findings.iter().any(|f| f.message.contains("plugin")));
}

#[tokio::test]
async fn stopped_vmware_utility_needs_attention_with_a_fix() {
    let r = Host {
        probe: FakeProbe {
            utility: false,
            paths: vec![],
            hypervisor: Some(false),
        },
        ..Host::default()
    }
    .check()
    .await;
    let vm = provider(&r, "vmware_desktop");
    assert_eq!(vm.state, ProviderState::Attention);
    // Still found, so its capabilities are known.
    assert_eq!(vm.support, SupportLevel::Detected);
    assert!(problem(vm).message.contains("127.0.0.1:9922"));
    assert!(
        problem(vm)
            .fix
            .as_deref()
            .unwrap()
            .contains("VagrantVMware")
    );
}

#[tokio::test]
async fn windows_hypervisor_is_a_virtualbox_warning() {
    let r = Host {
        probe: FakeProbe {
            utility: true,
            paths: vec![],
            hypervisor: Some(true),
        },
        ..Host::default()
    }
    .check()
    .await;
    let vb = provider(&r, "virtualbox");
    assert_eq!(vb.state, ProviderState::Attention);
    assert_eq!(problem(vb).level, Level::Warning);
    assert!(
        problem(vb)
            .message
            .contains("Windows hypervisor is running")
    );
    assert!(
        problem(vb)
            .fix
            .as_deref()
            .unwrap()
            .contains("doesn't change")
    );
    // VMware isn't affected by this check.
    assert_eq!(provider(&r, "vmware_desktop").state, ProviderState::Ready);
}

#[tokio::test]
async fn unsupported_virtualbox_version_is_an_error() {
    let r = Host {
        virtualbox: "7.3.0r180000",
        ..Host::default()
    }
    .check()
    .await;
    let vb = provider(&r, "virtualbox");
    assert_eq!(vb.state, ProviderState::Attention);
    assert_eq!(problem(vb).level, Level::Error);
    assert_eq!(
        problem(vb).message,
        "Vagrant 2.4.9 doesn't support VirtualBox 7.3.0; machines won't start."
    );
    assert!(problem(vb).fix.as_deref().unwrap().contains("7.2 or older"));
}

#[tokio::test]
async fn linux_virtualbox_needs_its_kernel_module() {
    let linux = |paths| Host {
        os: HostOs::Linux,
        probe: FakeProbe {
            utility: true,
            paths,
            hypervisor: Some(true),
        },
        ..Host::default()
    };
    let r = linux(vec![]).check().await;
    let vb = provider(&r, "virtualbox");
    assert_eq!(vb.state, ProviderState::Attention);
    assert!(problem(vb).message.contains("vboxdrv"));
    assert!(problem(vb).fix.as_deref().unwrap().contains("vboxconfig"));

    let r = linux(vec!["/dev/vboxdrv"]).check().await;
    let vb = provider(&r, "virtualbox");
    // The hypervisor check applies to Windows only.
    assert_eq!(vb.state, ProviderState::Ready, "{:?}", vb.findings);
    // Verified on Windows doesn't carry over to Linux.
    assert_eq!(vb.support, SupportLevel::Detected);
}

#[tokio::test]
async fn unreachable_docker_daemon_quotes_docker() {
    let r = Host {
        os: HostOs::Linux,
        docker: (
            1,
            "",
            "permission denied while trying to connect to the Docker daemon socket",
        ),
        probe: FakeProbe {
            utility: true,
            paths: vec!["/dev/vboxdrv"],
            hypervisor: None,
        },
        ..Host::default()
    }
    .check()
    .await;
    let docker = provider(&r, "docker");
    assert_eq!(docker.state, ProviderState::Attention);
    assert_eq!(docker.version, None);
    assert!(problem(docker).message.contains("permission denied"));
    assert!(
        problem(docker)
            .fix
            .as_deref()
            .unwrap()
            .contains("docker` group")
    );
}

#[tokio::test]
async fn reports_serialize_for_the_frontend() {
    let r = Host::default().check().await;
    let json = serde_json::to_value(provider(&r, "docker")).unwrap();
    assert_eq!(json["kind"], "container");
    assert_eq!(json["support"], "detected");
    assert_eq!(
        json["capabilities"],
        serde_json::json!([
            "lifecycle.basic",
            "network.forwarded_port",
            "synced_folder.default"
        ])
    );
}

#[test]
fn known_install_locations_are_used_when_not_on_path() {
    let probe = FakeProbe {
        utility: false,
        paths: vec!["/usr/lib/virtualbox/VBoxManage"],
        hypervisor: None,
    };
    let platform = FakePlatform {
        os: HostOs::Linux,
        found: vec![],
    };
    let runner = ScriptedRunner {
        virtualbox: "",
        docker: (0, "", ""),
    };
    let ctx = Context {
        runner: &runner,
        platform: &platform,
        probe: &probe,
        os: HostOs::Linux,
        vagrant_version: None,
        plugins: &[],
    };
    let found = tools::find(
        &ctx,
        "VBoxManage",
        &[
            (HostOs::Windows, r"C:\VBoxManage.exe"),
            (HostOs::Linux, "/usr/bin/VBoxManage"),
            (HostOs::Linux, "/usr/lib/virtualbox/VBoxManage"),
        ],
    );
    assert_eq!(found, Some(PathBuf::from("/usr/lib/virtualbox/VBoxManage")));
}

#[test]
fn expands_windows_style_variables() {
    let path = tools::expand_env(r"%VAGRANT_STUDIO_SURELY_UNSET%\x");
    assert_eq!(path, r"%VAGRANT_STUDIO_SURELY_UNSET%\x");
    assert_eq!(tools::expand_env("/usr/bin/vmrun"), "/usr/bin/vmrun");
    assert_eq!(tools::expand_env("50%"), "50%");
}

#[cfg(target_arch = "x86_64")]
#[test]
fn system_probe_reads_the_hypervisor_bit() {
    let present = SystemProbe.hypervisor_present();
    println!("hypervisor present: {present:?}");
    assert!(present.is_some());
}
