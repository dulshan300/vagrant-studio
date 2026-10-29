//! Vagrant's built-in `docker` provider: containers, not VMs. It is the
//! automated lifecycle test harness on Linux (ADR-0003). It refuses
//! `suspend` (verification log, 2026-10-28) and has no CPU/memory settings,
//! and without an SSH-enabled image there is no `vagrant ssh` or shell
//! provisioning, so only the capabilities that hold for every container
//! are reported.

use vagrant_studio_platform::HostOs;
use vagrant_studio_vagrant_adapter::readiness::{Finding, Level, install_hint};

use crate::{Capability, Context, DetectFuture, Detection, ProviderAdapter, ProviderKind, tools};

pub struct Docker;

impl ProviderAdapter for Docker {
    fn id(&self) -> &'static str {
        "docker"
    }
    fn name(&self) -> &'static str {
        "Docker"
    }
    fn kind(&self) -> ProviderKind {
        ProviderKind::Container
    }
    fn capabilities(&self) -> &'static [Capability] {
        &[
            Capability::LifecycleBasic,
            Capability::NetworkForwardedPort,
            Capability::SyncedFolderDefault,
        ]
    }
    fn detect<'a>(&'a self, ctx: &'a Context<'a>) -> DetectFuture<'a> {
        Box::pin(detect(ctx))
    }
}

async fn detect(ctx: &Context<'_>) -> Detection {
    let exe = tools::find(
        ctx,
        "docker",
        &[(
            HostOs::Windows,
            r"%ProgramFiles%\Docker\Docker\resources\bin\docker.exe",
        )],
    );
    let Some(exe) = exe else {
        return Detection::new(
            false,
            None,
            vec![Finding::problem(
                Level::Error,
                "Docker is not installed (docker not found).",
                install_hint(
                    ctx.os,
                    "Docker",
                    "https://docs.docker.com/get-started/get-docker/",
                ),
            )],
        );
    };
    let mut findings = vec![Finding::ok(format!("docker found at {}.", exe.display()))];
    // Asks the daemon, so it fails when the daemon is stopped or not accessible.
    let output = tools::run(ctx, &exe, &["version", "--format", "{{.Server.Version}}"]).await;
    let version = match output {
        Some(o) if o.success && o.first_line().is_some() => {
            let v = o.first_line().unwrap_or_default().to_owned();
            findings.push(Finding::ok(format!("The Docker daemon {v} is running.")));
            Some(v)
        }
        other => {
            let detail = other
                .as_ref()
                .and_then(|o| o.stderr.lines().map(str::trim).find(|l| !l.is_empty()))
                .map(|l| format!(" Docker said: {l}"))
                .unwrap_or_default();
            let fix = match ctx.os {
                HostOs::Windows => {
                    "Start Docker Desktop and wait until it reports that it is running."
                }
                _ => {
                    "Start the Docker service (`sudo systemctl start docker`) and make sure your user may use it (member of the `docker` group; log in again after adding it)."
                }
            };
            findings.push(Finding::problem(
                Level::Error,
                format!("The Docker daemon isn't reachable.{detail}"),
                fix,
            ));
            None
        }
    };
    Detection::new(true, version, findings)
}
