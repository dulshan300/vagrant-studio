//! Vagrant CLI integration: locating the executable, building typed
//! commands, running them and parsing their output.

pub mod boxes;
pub mod command;
pub mod machine_readable;
pub mod readiness;
pub mod runner;

use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use vagrant_studio_platform::PlatformAdapter;

pub use command::{LifecycleAction, MachineName, StateClass, VagrantCommand};
pub use runner::{Invocation, OutputLine, ProcessRunner, RunResult, Runner, Stream, Termination};

/// Finds the Vagrant executable: a configured override if it exists, else the
/// platform's search (PATH, well-known install locations).
pub fn locate(override_path: Option<&Path>, platform: &dyn PlatformAdapter) -> Option<PathBuf> {
    if let Some(path) = override_path {
        return path.is_file().then(|| path.to_path_buf());
    }
    platform.find_executable("vagrant")
}

#[derive(Debug, thiserror::Error)]
pub enum DetectError {
    #[error("could not run Vagrant: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("Vagrant did not respond within {0:?}")]
    TimedOut(Duration),
    #[error("unexpected output from `vagrant --version`: {0}")]
    UnexpectedOutput(String),
}

/// Runs `vagrant --version` (Vagrant on Windows can take several seconds to start).
pub async fn detect_version(runner: &dyn Runner, program: &Path) -> Result<String, DetectError> {
    const TIMEOUT: Duration = Duration::from_secs(60);
    let mut output = String::new();
    let result = runner
        .run(
            Invocation {
                program: program.to_path_buf(),
                args: VagrantCommand::Version.args(),
                working_dir: std::env::temp_dir(),
                timeout: Some(TIMEOUT),
                env: vec![],
            },
            Box::new(|line| {
                output.push_str(&line.text);
                output.push('\n');
            }),
            CancellationToken::new(),
        )
        .await?;
    if result.termination == Termination::TimedOut {
        return Err(DetectError::TimedOut(TIMEOUT));
    }
    machine_readable::parse_version(&output)
        .ok_or_else(|| DetectError::UnexpectedOutput(output.trim().chars().take(300).collect()))
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("could not run Vagrant: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("Vagrant did not respond within {} seconds", .0.as_secs())]
    TimedOut(Duration),
    #[error("{0}")]
    Failed(String),
}

/// Runs a read-only `--machine-readable` command that isn't recorded as an
/// operation (box list, update check) and returns its parsed lines.
pub async fn query(
    runner: &dyn Runner,
    program: &Path,
    command: &VagrantCommand,
    timeout: Duration,
) -> Result<Vec<machine_readable::Line>, QueryError> {
    debug_assert!(!command.is_mutating() && !command.changes_boxes());
    let mut stdout = String::new();
    let mut stderr = String::new();
    let result = runner
        .run(
            Invocation {
                program: program.to_path_buf(),
                args: command.args(),
                working_dir: std::env::temp_dir(),
                timeout: Some(timeout),
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
        .await?;
    if result.termination == Termination::TimedOut {
        return Err(QueryError::TimedOut(timeout));
    }
    let lines = machine_readable::parse(&stdout);
    if result.success() {
        return Ok(lines);
    }
    let message = match machine_readable::error_exit(&lines) {
        Some(e) => e.message,
        None if !stderr.trim().is_empty() => stderr.trim().chars().take(1000).collect(),
        None => format!(
            "Vagrant exited with code {}.",
            result.exit_code.unwrap_or(-1)
        ),
    };
    Err(QueryError::Failed(message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vagrant_studio_platform::{Arch, HostInfo, HostOs};

    struct NoVagrant;
    impl PlatformAdapter for NoVagrant {
        fn host_info(&self) -> HostInfo {
            HostInfo {
                os: HostOs::Linux,
                arch: Arch::X86_64,
                is_wsl: false,
            }
        }
        fn find_executable(&self, _: &str) -> Option<PathBuf> {
            None
        }
    }

    /// Replays canned output instead of running a process.
    struct Canned(&'static str);
    impl Runner for Canned {
        fn run<'a>(
            &'a self,
            _: Invocation,
            mut on_line: runner::LineSink<'a>,
            _: CancellationToken,
        ) -> runner::RunFuture<'a> {
            Box::pin(async move {
                for line in self.0.lines() {
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

    #[test]
    fn missing_vagrant_is_reported_as_none() {
        assert_eq!(locate(None, &NoVagrant), None);
        assert_eq!(
            locate(Some(Path::new("/missing/vagrant")), &NoVagrant),
            None
        );
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("vagrant");
        std::fs::write(&exe, "").unwrap();
        assert_eq!(locate(Some(&exe), &NoVagrant), Some(exe));
    }

    #[tokio::test]
    async fn queries_parse_output_or_report_vagrant_errors() {
        let lines = query(
            &Canned("1,,box-name,a/b\r\n1,,box-provider,virtualbox"),
            Path::new("vagrant"),
            &VagrantCommand::BoxList,
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        assert_eq!(machine_readable::parse_box_list(&lines)[0].name, "a/b");
    }

    #[tokio::test]
    async fn version_detection_parses_or_explains() {
        let v = detect_version(&Canned("Vagrant 2.4.9\r\n"), Path::new("vagrant"))
            .await
            .unwrap();
        assert_eq!(v, "2.4.9");
        let err = detect_version(&Canned("something else"), Path::new("vagrant"))
            .await
            .unwrap_err();
        assert!(matches!(err, DetectError::UnexpectedOutput(ref s) if s == "something else"));
    }
}
