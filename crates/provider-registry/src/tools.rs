//! Finding provider executables and reading their output.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use vagrant_studio_platform::HostOs;
use vagrant_studio_vagrant_adapter::{Invocation, Stream, Termination};

use crate::Context;

/// PATH first, then well-known install locations for this OS.
pub(crate) fn find(ctx: &Context<'_>, name: &str, known: &[(HostOs, &str)]) -> Option<PathBuf> {
    ctx.platform.find_executable(name).or_else(|| {
        known
            .iter()
            .filter(|(os, _)| *os == ctx.os)
            .map(|(_, p)| PathBuf::from(expand_env(p)))
            .find(|p| ctx.probe.path_exists(p))
    })
}

pub(crate) struct ToolOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl ToolOutput {
    pub fn first_line(&self) -> Option<&str> {
        self.stdout.lines().map(str::trim).find(|l| !l.is_empty())
    }
}

/// Runs a provider tool briefly; `None` when it couldn't be started or timed out.
pub(crate) async fn run(ctx: &Context<'_>, program: &Path, args: &[&str]) -> Option<ToolOutput> {
    let mut stdout = String::new();
    let mut stderr = String::new();
    let result = ctx
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
                let out = match line.stream {
                    Stream::Stdout => &mut stdout,
                    Stream::Stderr => &mut stderr,
                };
                out.push_str(&line.text);
                out.push('\n');
            }),
            CancellationToken::new(),
        )
        .await
        .ok()?;
    if result.termination != Termination::Exited {
        return None;
    }
    Some(ToolOutput {
        success: result.success(),
        stdout,
        stderr,
    })
}

/// Expands `%VAR%` (Windows-style) references; unknown variables are left as-is.
pub(crate) fn expand_env(path: &str) -> String {
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
