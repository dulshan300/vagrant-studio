//! Git for a project folder, from the host's `git`: status, history, and the
//! few writes the version-control panel offers (init, stage, unstage, commit).
//!
//! Git runs through the adapter's [`Runner`] with an argv vector, never a
//! shell. Reads pass `core.fsmonitor=false` so the repository's own
//! configuration can't start programs, and `--no-optional-locks` so they
//! never write. Paths handed to Git come from its own status output and are
//! passed as literal, repository-relative pathspecs.

mod parse;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tokio_util::sync::CancellationToken;
use vagrant_studio_platform::{HostOs, PlatformAdapter};
use vagrant_studio_vagrant_adapter::{Invocation, RunResult, Runner, Stream};

pub use parse::{parse_log, parse_status, unquote_path};

const TIMEOUT: Duration = Duration::from_secs(20);
/// Commits run hooks, which can take a while.
const COMMIT_TIMEOUT: Duration = Duration::from_secs(120);
/// Commits listed in the panel.
pub const LOG_LIMIT: usize = 15;

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

/// Branch, change counts and changed files. Changes cover only the folder
/// and what is below it, even when the repository root is higher up.
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
    pub files: Vec<FileChange>,
    /// The repository's top-level folder as Git prints it (forward slashes).
    pub root: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeKind {
    Changed,
    Renamed,
    Conflicted,
    Untracked,
}

/// One changed path, relative to the repository root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    /// The old path of a rename or copy.
    pub orig_path: Option<String>,
    pub kind: ChangeKind,
    /// Git's status letter in the index (`M`, `A`, `D`, `R`, …), or `.` when
    /// unchanged there; `?` for untracked files.
    pub index: char,
    /// The same for the work tree.
    pub worktree: char,
}

impl FileChange {
    /// Whether some of the change is staged.
    pub fn is_staged(&self) -> bool {
        !matches!(self.index, '.' | '?')
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitSummary {
    /// Abbreviated commit id.
    pub id: String,
    pub author: String,
    /// Author date, ISO 8601 with offset.
    pub date: String,
    pub subject: String,
}

/// Arguments for `git status` in the working directory.
pub fn status_args() -> Vec<String> {
    strings(&[
        "-c",
        "core.fsmonitor=false",
        "-c",
        "core.quotePath=false",
        // Paths relative to the repository root, like the pathspecs we pass back.
        "-c",
        "status.relativePaths=false",
        "--no-optional-locks",
        "status",
        "--porcelain=v2",
        "--branch",
        "--untracked-files=normal",
        "--",
        ".",
    ])
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| (*a).to_owned()).collect()
}

/// A literal pathspec relative to the repository root, so the working
/// directory doesn't matter and nothing in the path acts as a wildcard.
fn pathspec(path: &str) -> String {
    format!(":(top,literal){path}")
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

struct GitOutput {
    result: std::io::Result<RunResult>,
    stdout: Vec<String>,
    stderr: Vec<String>,
}

impl GitOutput {
    /// The last non-empty error line, or `fallback`.
    fn error(&self, fallback: &str) -> String {
        match &self.result {
            Err(e) => format!("git couldn't be started: {e}"),
            Ok(_) => self
                .stderr
                .iter()
                .chain(&self.stdout)
                .rev()
                .find(|l| !l.trim().is_empty())
                .map(|l| l.trim().to_owned())
                .unwrap_or_else(|| fallback.to_owned()),
        }
    }

    fn success(&self) -> bool {
        self.result.as_ref().is_ok_and(RunResult::success)
    }

    fn into_result(self, fallback: &str) -> Result<Vec<String>, String> {
        if self.success() {
            Ok(self.stdout)
        } else {
            Err(self.error(fallback))
        }
    }
}

async fn run_git(
    runner: &dyn Runner,
    git: &Path,
    folder: &Path,
    args: Vec<String>,
    timeout: Duration,
) -> GitOutput {
    let stdout = Arc::new(Mutex::new(Vec::new()));
    let stderr = Arc::new(Mutex::new(Vec::new()));
    let (out, err) = (stdout.clone(), stderr.clone());
    let result = runner
        .run(
            Invocation {
                program: git.to_owned(),
                args,
                working_dir: folder.to_owned(),
                timeout: Some(timeout),
                env: vec![("GIT_TERMINAL_PROMPT".into(), "0".into())],
            },
            Box::new(move |line| match line.stream {
                Stream::Stdout => out.lock().unwrap().push(line.text),
                Stream::Stderr => err.lock().unwrap().push(line.text),
            }),
            CancellationToken::new(),
        )
        .await;
    let stdout = std::mem::take(&mut *stdout.lock().unwrap());
    let stderr = std::mem::take(&mut *stderr.lock().unwrap());
    GitOutput {
        result,
        stdout,
        stderr,
    }
}

/// Runs `git status` in `folder`. `git` is `None` when Git wasn't found.
pub async fn status(runner: &dyn Runner, git: Option<&Path>, folder: &Path) -> GitStatus {
    let Some(git) = git else {
        return GitStatus::Unavailable;
    };
    let out = run_git(runner, git, folder, status_args(), TIMEOUT).await;
    if out.success() {
        let mut status = parse_status(&out.stdout);
        let root = run_git(
            runner,
            git,
            folder,
            strings(&["-c", "core.fsmonitor=false", "rev-parse", "--show-toplevel"]),
            TIMEOUT,
        )
        .await;
        status.root = root
            .into_result("")
            .ok()
            .and_then(|lines| lines.into_iter().next());
        GitStatus::Repository(status)
    } else if out.result.is_ok()
        && out
            .stderr
            .iter()
            .any(|l| l.contains("not a git repository"))
    {
        GitStatus::NotRepository
    } else {
        GitStatus::Error {
            message: out.error("git status failed."),
        }
    }
}

/// The latest commits that touched `folder`, newest first. Empty before the
/// first commit.
pub async fn log(
    runner: &dyn Runner,
    git: &Path,
    folder: &Path,
) -> Result<Vec<CommitSummary>, String> {
    let limit = LOG_LIMIT.to_string();
    let args = strings(&[
        "-c",
        "core.fsmonitor=false",
        "--no-optional-locks",
        "log",
        "-n",
        &limit,
        "--format=%h%x1f%an%x1f%aI%x1f%s",
        "--",
        ".",
    ]);
    let out = run_git(runner, git, folder, args, TIMEOUT).await;
    if !out.success()
        && out
            .stderr
            .iter()
            .any(|l| l.contains("does not have any commits"))
    {
        return Ok(Vec::new());
    }
    out.into_result("git log failed.")
        .map(|lines| parse_log(&lines))
}

/// Creates an empty repository in `folder` with `git init`. The initial
/// branch follows the user's Git configuration; nothing is committed.
pub async fn init(runner: &dyn Runner, git: &Path, folder: &Path) -> Result<(), String> {
    run_git(runner, git, folder, strings(&["init", "--quiet"]), TIMEOUT)
        .await
        .into_result("git init failed.")
        .map(drop)
}

/// Stages the changes to `files`, including deletions.
pub async fn stage(
    runner: &dyn Runner,
    git: &Path,
    folder: &Path,
    files: &[&FileChange],
) -> Result<(), String> {
    let mut args = strings(&["add", "-A", "--"]);
    args.extend(files.iter().map(|f| pathspec(&f.path)));
    run_git(runner, git, folder, args, TIMEOUT)
        .await
        .into_result("git add failed.")
        .map(drop)
}

/// Unstages `files`, keeping the work tree as it is. Before the first commit
/// there is nothing to restore from, so the paths are removed from the index.
pub async fn unstage(
    runner: &dyn Runner,
    git: &Path,
    folder: &Path,
    files: &[&FileChange],
    has_commits: bool,
) -> Result<(), String> {
    let mut args = if has_commits {
        strings(&["restore", "--staged", "--"])
    } else {
        strings(&["rm", "--cached", "-r", "-q", "--"])
    };
    for f in files {
        args.push(pathspec(&f.path));
        // A staged rename has to be undone on both sides.
        if let Some(orig) = &f.orig_path {
            args.push(pathspec(orig));
        }
    }
    run_git(runner, git, folder, args, TIMEOUT)
        .await
        .into_result("Unstaging failed.")
        .map(drop)
}

/// Commits what is staged, with the user's own Git identity and hooks.
pub async fn commit(
    runner: &dyn Runner,
    git: &Path,
    folder: &Path,
    message: &str,
) -> Result<(), String> {
    let out = run_git(
        runner,
        git,
        folder,
        vec![
            "commit".into(),
            "--quiet".into(),
            "-m".into(),
            message.to_owned(),
        ],
        COMMIT_TIMEOUT,
    )
    .await;
    if !out.success()
        && out
            .stderr
            .iter()
            .any(|l| l.contains("Please tell me who you are"))
    {
        return Err("Git doesn't know who you are yet. In a terminal, run \
             git config --global user.name \"Your Name\" and \
             git config --global user.email you@example.com, then commit again."
            .into());
    }
    out.into_result("git commit failed.").map(drop)
}

#[cfg(test)]
mod tests;
