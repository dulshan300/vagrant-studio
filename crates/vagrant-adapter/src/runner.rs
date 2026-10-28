//! Process execution: argv only, explicit working directory, streamed output,
//! timeouts and cancellation that end the whole process tree.

use std::future::Future;
use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::time::Duration;

use process_wrap::tokio::*;
use serde::Serialize;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

/// How long to wait for a graceful stop before killing the process tree.
const GRACE_PERIOD: Duration = Duration::from_secs(10);
/// How long to keep reading output after the process has exited.
const DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_LINE: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
    pub timeout: Option<Duration>,
    /// Extra environment variables (e.g. `VAGRANT_DEFAULT_PROVIDER`).
    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutputLine {
    pub stream: Stream,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Termination {
    /// The process exited on its own; `exit_code` is set unless it died from a signal.
    Exited,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    pub termination: Termination,
    pub exit_code: Option<i32>,
}

impl RunResult {
    pub fn success(&self) -> bool {
        self.termination == Termination::Exited && self.exit_code == Some(0)
    }
}

pub type LineSink<'a> = Box<dyn FnMut(OutputLine) + Send + 'a>;
pub type RunFuture<'a> = Pin<Box<dyn Future<Output = io::Result<RunResult>> + Send + 'a>>;

/// Runs processes. The real implementation is [`ProcessRunner`]; tests use fakes.
pub trait Runner: Send + Sync {
    fn run<'a>(
        &'a self,
        invocation: Invocation,
        on_line: LineSink<'a>,
        cancel: CancellationToken,
    ) -> RunFuture<'a>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessRunner;

impl Runner for ProcessRunner {
    fn run<'a>(
        &'a self,
        invocation: Invocation,
        on_line: LineSink<'a>,
        cancel: CancellationToken,
    ) -> RunFuture<'a> {
        Box::pin(run_process(invocation, on_line, cancel))
    }
}

async fn run_process(
    inv: Invocation,
    mut on_line: LineSink<'_>,
    cancel: CancellationToken,
) -> io::Result<RunResult> {
    if !inv.working_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "working directory {} does not exist",
                inv.working_dir.display()
            ),
        ));
    }

    let mut command = CommandWrap::with_new(&inv.program, |c| {
        c.args(&inv.args)
            .current_dir(&inv.working_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Offline-first: no version-check network calls.
            .env("VAGRANT_CHECKPOINT_DISABLE", "1")
            .envs(inv.env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    });
    #[cfg(windows)]
    {
        use windows::Win32::System::Threading::CREATE_NO_WINDOW;
        command.wrap(CreationFlags(CREATE_NO_WINDOW));
        command.wrap(JobObject);
    }
    #[cfg(unix)]
    command.wrap(ProcessGroup::leader());
    command.wrap(KillOnDrop);

    let mut child = command.spawn()?;
    tracing::debug!(program = %inv.program.display(), args = ?inv.args, cwd = %inv.working_dir.display(), pid = ?child.id(), "spawned");

    let (tx, mut lines) = mpsc::unbounded_channel();
    let readers = [
        child
            .stdout()
            .take()
            .map(|r| tokio::spawn(read_lines(r, Stream::Stdout, tx.clone()))),
        child
            .stderr()
            .take()
            .map(|r| tokio::spawn(read_lines(r, Stream::Stderr, tx.clone()))),
    ];
    drop(tx);

    // The child is owned by its own task so it can be waited on and killed
    // without fighting the borrow checker in the select loop below.
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let mut waiter = tokio::spawn(async move {
        tokio::select! {
            status = child.wait() => status,
            _ = stop_rx => {
                stop(&mut *child).await;
                child.wait().await
            }
        }
    });

    let deadline = async {
        match inv.timeout {
            Some(t) => tokio::time::sleep(t).await,
            None => std::future::pending().await,
        }
    };
    tokio::pin!(deadline);

    let mut stop_tx = Some(stop_tx);
    let mut termination = Termination::Exited;
    let status = loop {
        tokio::select! {
            Some(line) = lines.recv() => on_line(line),
            _ = cancel.cancelled(), if stop_tx.is_some() => {
                termination = Termination::Cancelled;
                let _ = stop_tx.take().map(|tx| tx.send(()));
            }
            _ = &mut deadline, if stop_tx.is_some() => {
                termination = Termination::TimedOut;
                let _ = stop_tx.take().map(|tx| tx.send(()));
            }
            status = &mut waiter => break status.map_err(io::Error::other)??,
        }
    };

    // Deliver output still buffered after exit, but don't hang on handles
    // inherited by stray grandchildren.
    let _ = tokio::time::timeout(DRAIN_TIMEOUT, async {
        while let Some(line) = lines.recv().await {
            on_line(line);
        }
    })
    .await;
    for reader in readers.into_iter().flatten() {
        reader.abort();
    }

    Ok(RunResult {
        termination,
        exit_code: status.code(),
    })
}

/// Unix: SIGINT to the process group (Vagrant then cleans up), kill after a
/// grace period. Windows: console-less processes can't receive Ctrl+C, so
/// the job object (whole tree) is terminated immediately.
async fn stop(child: &mut dyn ChildWrapper) {
    #[cfg(unix)]
    {
        const SIGINT: i32 = 2;
        if child.signal(SIGINT).is_ok()
            && tokio::time::timeout(GRACE_PERIOD, child.wait())
                .await
                .is_ok()
        {
            return;
        }
    }
    #[cfg(not(unix))]
    let _ = GRACE_PERIOD;
    if let Err(e) = child.start_kill() {
        tracing::warn!(error = %e, "failed to kill process tree");
    }
}

/// Splits on `\n`, `\r\n` and bare `\r` (progress updates), decoding lossily.
async fn read_lines(
    mut reader: impl AsyncRead + Unpin,
    stream: Stream,
    tx: mpsc::UnboundedSender<OutputLine>,
) {
    let mut buf = [0u8; 8192];
    let mut line: Vec<u8> = Vec::new();
    let mut last_was_cr = false;
    let send = |line: &mut Vec<u8>| {
        let text = String::from_utf8_lossy(line).into_owned();
        line.clear();
        tx.send(OutputLine { stream, text }).is_ok()
    };
    loop {
        let n = match reader.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        for &b in &buf[..n] {
            match b {
                b'\n' if last_was_cr => {}
                b'\n' | b'\r' => {
                    if !send(&mut line) {
                        return;
                    }
                }
                _ => {
                    line.push(b);
                    if line.len() >= MAX_LINE && !send(&mut line) {
                        return;
                    }
                }
            }
            last_was_cr = b == b'\r';
        }
    }
    if !line.is_empty() {
        send(&mut line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    async fn run(
        program: &str,
        args: &[&str],
        timeout: Option<Duration>,
        cancel: CancellationToken,
    ) -> (io::Result<RunResult>, Vec<OutputLine>) {
        let collected = Arc::new(Mutex::new(Vec::new()));
        let sink = collected.clone();
        let result = ProcessRunner
            .run(
                Invocation {
                    program: program.into(),
                    args: args.iter().map(|s| (*s).to_owned()).collect(),
                    working_dir: std::env::temp_dir(),
                    timeout,
                    env: vec![],
                },
                Box::new(move |l| sink.lock().unwrap().push(l)),
                cancel,
            )
            .await;
        let lines = collected.lock().unwrap().clone();
        (result, lines)
    }

    #[cfg(unix)]
    const SHELL: (&str, &str) = ("sh", "-c");
    #[cfg(windows)]
    const SHELL: (&str, &str) = ("cmd", "/C");

    #[tokio::test]
    async fn streams_both_outputs_and_reports_exit_code() {
        let (result, lines) = run(
            SHELL.0,
            &[SHELL.1, "echo out&& echo err 1>&2&& exit 3"],
            None,
            CancellationToken::new(),
        )
        .await;
        let result = result.unwrap();
        assert_eq!(result.termination, Termination::Exited);
        assert_eq!(result.exit_code, Some(3));
        assert!(!result.success());
        let text = |s: Stream| -> Vec<String> {
            lines
                .iter()
                .filter(|l| l.stream == s)
                .map(|l| l.text.trim().to_owned())
                .collect()
        };
        assert_eq!(text(Stream::Stdout), ["out"]);
        assert_eq!(text(Stream::Stderr), ["err"]);
    }

    #[tokio::test]
    async fn cancellation_stops_the_process_quickly() {
        let cancel = CancellationToken::new();
        let trigger = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            trigger.cancel();
        });
        let started = Instant::now();
        #[cfg(unix)]
        let args = [SHELL.1, "sleep 30"];
        #[cfg(windows)]
        let args = [SHELL.1, "ping -n 30 127.0.0.1 >NUL"];
        let (result, _) = run(SHELL.0, &args, None, cancel).await;
        assert_eq!(result.unwrap().termination, Termination::Cancelled);
        assert!(started.elapsed() < Duration::from_secs(15));
    }

    #[tokio::test]
    async fn timeout_stops_the_process() {
        #[cfg(unix)]
        let args = [SHELL.1, "sleep 30"];
        #[cfg(windows)]
        let args = [SHELL.1, "ping -n 30 127.0.0.1 >NUL"];
        let started = Instant::now();
        let (result, _) = run(
            SHELL.0,
            &args,
            Some(Duration::from_millis(300)),
            CancellationToken::new(),
        )
        .await;
        let result = result.unwrap();
        assert_eq!(result.termination, Termination::TimedOut);
        assert!(!result.success());
        assert!(started.elapsed() < Duration::from_secs(15));
    }

    #[tokio::test]
    async fn arguments_are_not_interpreted_by_a_shell() {
        #[cfg(unix)]
        let (program, args) = ("echo", ["a; echo injected", "$(whoami)"]);
        #[cfg(windows)]
        let (program, args) = ("cmd", ["/C", "echo"]);
        let (result, lines) = run(program, &args, None, CancellationToken::new()).await;
        assert!(result.unwrap().success());
        #[cfg(unix)]
        assert_eq!(lines[0].text, "a; echo injected $(whoami)");
        #[cfg(windows)]
        let _ = lines;
    }

    #[tokio::test]
    async fn missing_program_or_directory_is_an_error() {
        let (result, _) = run(
            "definitely-not-a-real-program-xyz",
            &[],
            None,
            CancellationToken::new(),
        )
        .await;
        assert!(result.is_err());

        let missing_dir = ProcessRunner
            .run(
                Invocation {
                    program: SHELL.0.into(),
                    args: vec![],
                    working_dir: "/definitely/missing/dir".into(),
                    timeout: None,
                    env: vec![],
                },
                Box::new(|_| {}),
                CancellationToken::new(),
            )
            .await;
        assert_eq!(missing_dir.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn splits_crlf_and_bare_carriage_returns() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        read_lines(&b"a\r\nb\rc\nd\xff"[..], Stream::Stdout, tx).await;
        let mut out = Vec::new();
        while let Some(l) = rx.recv().await {
            out.push(l.text);
        }
        assert_eq!(out, ["a", "b", "c", "d\u{fffd}"]);
    }
}
