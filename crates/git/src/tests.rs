use std::process::Command;

use vagrant_studio_platform::SystemPlatform;
use vagrant_studio_vagrant_adapter::ProcessRunner;

use super::*;

fn lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_owned).collect()
}

#[test]
fn parses_branch_upstream_and_counts() {
    let status = parse_status(&lines(
        "# branch.oid 8b9e04b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7
# branch.head phase-7
# branch.upstream origin/phase-7
# branch.ab +3 -1
1 M. N... 100644 100644 100644 aaa bbb Vagrantfile
1 .M N... 100644 100644 100644 aaa bbb notes.md
1 MM N... 100644 100644 100644 aaa bbb both.txt
2 R. N... 100644 100644 100644 aaa bbb R100 new name.txt\told.txt
u UU N... 100644 100644 100644 100644 aaa bbb ccc conflict.txt
? new file.txt
? scripts/",
    ));
    assert_eq!(
        status,
        RepoStatus {
            branch: Some("phase-7".into()),
            commit: Some("8b9e04b".into()),
            upstream: Some("origin/phase-7".into()),
            ahead: Some(3),
            behind: Some(1),
            staged: 3,
            modified: 2,
            untracked: 2,
            conflicted: 1,
        }
    );
}

#[test]
fn parses_a_new_repository_and_a_detached_head() {
    let fresh = parse_status(&lines("# branch.oid (initial)\n# branch.head main"));
    assert_eq!(fresh.commit, None);
    assert_eq!(fresh.branch.as_deref(), Some("main"));
    assert_eq!((fresh.ahead, fresh.upstream), (None, None));

    let detached = parse_status(&lines("# branch.oid abcdef0123\n# branch.head (detached)"));
    assert_eq!(detached.branch, None);
    assert_eq!(detached.commit.as_deref(), Some("abcdef0"));
}

#[test]
fn status_is_read_only_and_scoped_to_the_folder() {
    let args = status_args();
    assert_eq!(
        &args[..3],
        ["-c", "core.fsmonitor=false", "--no-optional-locks"]
    );
    assert_eq!(&args[args.len() - 2..], ["--", "."]);
    let json = serde_json::to_value(GitStatus::Repository(RepoStatus::default())).unwrap();
    assert_eq!(json["state"], "repository");
    assert_eq!(
        serde_json::to_value(GitStatus::NotRepository).unwrap()["state"],
        "notRepository"
    );
}

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git {args:?}");
}

/// Uses the host's real `git`; skipped when Git isn't installed.
#[tokio::test]
async fn real_git_reports_changes_below_the_folder_only() {
    let Some(program) = locate(&SystemPlatform) else {
        eprintln!("git not installed; skipping");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let root = dunce_root(dir.path());
    let env = root.join("env");
    std::fs::create_dir(&env).unwrap();

    assert_eq!(
        status(&ProcessRunner, Some(&program), &env).await,
        GitStatus::NotRepository
    );
    assert_eq!(
        status(&ProcessRunner, None, &env).await,
        GitStatus::Unavailable
    );

    git(&root, &["init", "-q", "-b", "main"]);
    std::fs::write(env.join("Vagrantfile"), "x").unwrap();
    std::fs::write(root.join("README"), "x").unwrap();
    let GitStatus::Repository(s) = status(&ProcessRunner, Some(&program), &env).await else {
        panic!("expected a repository");
    };
    assert_eq!(
        (s.branch.as_deref(), s.commit.as_deref()),
        (Some("main"), None)
    );
    // README is above the environment folder, so it isn't counted there.
    assert_eq!((s.untracked, s.staged), (1, 0));

    git(&root, &["add", "."]);
    git(&root, &["commit", "-q", "-m", "init"]);
    std::fs::write(env.join("Vagrantfile"), "y").unwrap();
    let GitStatus::Repository(s) = status(&ProcessRunner, Some(&program), &root).await else {
        panic!("expected a repository");
    };
    assert!(s.commit.is_some());
    assert_eq!((s.modified, s.untracked, s.staged), (1, 0, 0));
}

/// Temp folders can sit behind a symlink or 8.3 name; Git reports real paths.
fn dunce_root(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .map(|p| PathBuf::from(p.to_string_lossy().trim_start_matches(r"\\?\")))
        .unwrap()
}
