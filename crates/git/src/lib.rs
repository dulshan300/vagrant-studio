//! Read-only Git status of a project folder, from the host's `git`.
//!
//! Git runs through the adapter's [`Runner`] with an argv vector. The
//! repository's own configuration can't start programs: `core.fsmonitor` is
//! turned off for the call, and `--no-optional-locks` keeps status from
//! writing to the repository.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tokio_util::sync::CancellationToken;
use vagrant_studio_platform::{HostOs, PlatformAdapter};
use vagrant_studio_vagrant_adapter::{Invocation, Runner, Stream};

const TIMEOUT: Duration = Duration::from_secs(20);

/// What Git says about a folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum GitStatus {
    /// Git isn't installed (or wasn't found).
    Unavailable,
    /// The folder isn't inside a Git repository.
    NotRepository,
    Repository(RepoStatus),
    /// Git ran but failed, e.g. a repository owned by another user.
    Error {
        message: String,
    },
}

/// Branch and change counts. Counts cover only the folder and what is below
/// it, even when the repository root is higher up.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoStatus {
    /// `None` when HEAD is detached.
    pub branch: Option<String>,
    /// Abbreviated commit id; `None` before the first commit.
    pub commit: Option<String>,
    pub upstream: Option<String>,
    /// Commits ahead of and behind the upstream, when there is one.
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    pub staged: u32,
    pub modified: u32,
    pub untracked: u32,
    pub conflicted: u32,
}

/// Arguments for `git status` in the working directory.
pub fn status_args() -> Vec<String> {
    [
        "-c",
        "core.fsmonitor=false",
        "--no-optional-locks",
        "status",
        "--porcelain=v2",
        "--branch",
        "--untracked-files=normal",
        "--",
        ".",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// Parses `git status --porcelain=v2 --branch` output.
pub fn parse_status(stdout: &[String]) -> RepoStatus {
    let mut s = RepoStatus::default();
    for line in stdout {
        if let Some(header) = line.strip_prefix("# ") {
            let (key, value) = header.split_once(' ').unwrap_or((header, ""));
            match key {
                "branch.oid" if value != "(initial)" => {
                    s.commit = Some(value.chars().take(7).collect());
                }
                "branch.head" if value != "(detached)" => s.branch = Some(value.to_owned()),
                "branch.upstream" => s.upstream = Some(value.to_owned()),
                "branch.ab" => {
                    let mut parts = value.split(' ');
                    s.ahead = parts.next().and_then(|a| a.strip_prefix('+')?.parse().ok());
                    s.behind = parts.next().and_then(|b| b.strip_prefix('-')?.parse().ok());
                }
                _ => {}
            }
            continue;
        }
        match line.split_once(' ') {
            // `1 XY ...` (changed) or `2 XY ...` (renamed or copied): X is the
            // index, Y the work tree; `.` means unchanged.
            Some(("1" | "2", rest)) => {
                let mut xy = rest.chars();
                if xy.next().is_some_and(|x| x != '.') {
                    s.staged += 1;
                }
                if xy.next().is_some_and(|y| y != '.') {
                    s.modified += 1;
                }
            }
            Some(("u", _)) => s.conflicted += 1,
            Some(("?", _)) => s.untracked += 1,
            _ => {}
        }
    }
    s
}

/// Finds `git`: on `PATH`, or Git for Windows' standard install folder.
pub fn locate(platform: &dyn PlatformAdapter) -> Option<PathBuf> {
    platform.find_executable("git").or_else(|| {
        if platform.host_info().os != HostOs::Windows {
            return None;
        }
        let base = std::env::var_os("ProgramFiles")?;
        let git = Path::new(&base).join(r"Git\cmd\git.exe");
        git.is_file().then_some(git)
    })
}

/// Runs `git status` in `folder`. `git` is `None` when Git wasn't found.
pub async fn status(runner: &dyn Runner, git: Option<&Path>, folder: &Path) -> GitStatus {
    let Some(git) = git else {
        return GitStatus::Unavailable;
    };
    let stdout = Arc::new(Mutex::new(Vec::new()));
    let stderr = Arc::new(Mutex::new(Vec::new()));
    let (out, err) = (stdout.clone(), stderr.clone());
    let result = runner
        .run(
            Invocation {
                program: git.to_owned(),
                args: status_args(),
                working_dir: folder.to_owned(),
                timeout: Some(TIMEOUT),
                env: vec![("GIT_TERMINAL_PROMPT".into(), "0".into())],
            },
            Box::new(move |line| match line.stream {
                Stream::Stdout => out.lock().unwrap().push(line.text),
                Stream::Stderr => err.lock().unwrap().push(line.text),
            }),
            CancellationToken::new(),
        )
        .await;
    let stderr = stderr.lock().unwrap().clone();
    match result {
        Ok(r) if r.success() => GitStatus::Repository(parse_status(&stdout.lock().unwrap())),
        Ok(_) if stderr.iter().any(|l| l.contains("not a git repository")) => {
            GitStatus::NotRepository
        }
        Ok(_) => GitStatus::Error {
            message: stderr
                .iter()
                .rev()
                .find(|l| !l.trim().is_empty())
                .map(|l| l.trim().to_owned())
                .unwrap_or_else(|| "git status failed.".into()),
        },
        Err(e) => GitStatus::Error {
            message: format!("git couldn't be started: {e}"),
        },
    }
}

#[cfg(test)]
mod tests;
