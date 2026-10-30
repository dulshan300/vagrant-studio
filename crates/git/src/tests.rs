use std::process::Command;

use vagrant_studio_platform::SystemPlatform;
use vagrant_studio_vagrant_adapter::ProcessRunner;

use super::*;

fn lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_owned).collect()
}

const STATUS: &str = "# branch.oid 8b9e04b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7
# branch.head phase-7
# branch.upstream origin/phase-7
# branch.ab +3 -1
1 M. N... 100644 100644 100644 aaa bbb Vagrantfile
1 .M N... 100644 100644 100644 aaa bbb notes with spaces.md
1 MM N... 100644 100644 100644 aaa bbb both.txt
2 R. N... 100644 100644 100644 aaa bbb R100 new name.txt\told.txt
u UU N... 100644 100644 100644 100644 aaa bbb ccc conflict.txt
? new file.txt
? \"quo\\\"ted\\303\\251.txt\"";

#[test]
fn parses_branch_upstream_counts_and_files() {
    let status = parse_status(&lines(STATUS));
    assert_eq!(status.branch.as_deref(), Some("phase-7"));
    assert_eq!(status.commit.as_deref(), Some("8b9e04b"));
    assert_eq!(status.upstream.as_deref(), Some("origin/phase-7"));
    assert_eq!((status.ahead, status.behind), (Some(3), Some(1)));
    assert_eq!(
        (
            status.staged,
            status.modified,
            status.untracked,
            status.conflicted
        ),
        (3, 2, 2, 1)
    );
    let paths: Vec<_> = status.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "Vagrantfile",
            "notes with spaces.md",
            "both.txt",
            "new name.txt",
            "conflict.txt",
            "new file.txt",
            "quo\"tedé.txt",
        ]
    );
    let renamed = &status.files[3];
    assert_eq!(renamed.kind, ChangeKind::Renamed);
    assert_eq!(renamed.orig_path.as_deref(), Some("old.txt"));
    assert!(renamed.is_staged());
    assert!(!status.files[1].is_staged());
    assert_eq!(
        (status.files[5].index, status.files[5].worktree),
        ('?', '?')
    );
    assert!(!status.files[5].is_staged());
}

#[test]
fn parses_a_new_repository_and_a_detached_head() {
    let fresh = parse_status(&lines("# branch.oid (initial)\n# branch.head main"));
    assert_eq!(fresh.commit, None);
    assert_eq!(fresh.branch.as_deref(), Some("main"));
    assert_eq!((fresh.ahead, fresh.upstream), (None, None));
    assert!(fresh.files.is_empty());

    let detached = parse_status(&lines("# branch.oid abcdef0123\n# branch.head (detached)"));
    assert_eq!(detached.branch, None);
    assert_eq!(detached.commit.as_deref(), Some("abcdef0"));
}

#[test]
fn unquotes_paths() {
    assert_eq!(unquote_path("plain name.txt"), "plain name.txt");
    assert_eq!(unquote_path(r#""tab\there""#), "tab\there");
    assert_eq!(unquote_path(r#""back\\slash""#), "back\\slash");
    assert_eq!(unquote_path(r#""caf\303\251""#), "café");
}

#[test]
fn parses_log_lines() {
    let log = parse_log(&lines(
        "1a2b3c4\u{1f}Dev\u{1f}2026-10-30T10:00:00+05:30\u{1f}Add web: nginx\u{1f}x\nbroken",
    ));
    assert_eq!(
        log,
        [CommitSummary {
            id: "1a2b3c4".into(),
            author: "Dev".into(),
            date: "2026-10-30T10:00:00+05:30".into(),
            subject: "Add web: nginx\u{1f}x".into(),
        }]
    );
}

#[test]
fn reads_are_side_effect_free_and_paths_are_literal() {
    let args = status_args();
    assert_eq!(&args[..2], ["-c", "core.fsmonitor=false"]);
    assert!(args.contains(&"--no-optional-locks".to_owned()));
    assert_eq!(&args[args.len() - 2..], ["--", "."]);
    assert_eq!(pathspec("*.log"), ":(top,literal)*.log");
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
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git {args:?}");
}

async fn repo(program: &Path, dir: &Path) -> RepoStatus {
    match status(&ProcessRunner, Some(program), dir).await {
        GitStatus::Repository(s) => s,
        other => panic!("expected a repository, got {other:?}"),
    }
}

fn find(s: &RepoStatus, path: &str) -> FileChange {
    s.files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("{path} not in {:?}", s.files))
        .clone()
}

/// Uses the host's real `git`; skipped when Git isn't installed.
#[tokio::test]
async fn real_git_init_stage_unstage_commit_and_log() {
    let Some(program) = locate(&SystemPlatform) else {
        eprintln!("git not installed; skipping");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let root = real_path(dir.path());
    let env = root.join("env");
    std::fs::create_dir(&env).unwrap();
    let runner = ProcessRunner;

    assert_eq!(
        status(&runner, Some(&program), &env).await,
        GitStatus::NotRepository
    );
    assert_eq!(status(&runner, None, &env).await, GitStatus::Unavailable);
    assert!(
        init(&runner, &program, &root.join("missing"))
            .await
            .is_err()
    );

    init(&runner, &program, &root).await.unwrap();
    // The initial branch and identity follow the user's configuration; pin them.
    git(&root, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    git(&root, &["config", "user.name", "Test"]);
    git(&root, &["config", "user.email", "test@example.com"]);
    std::fs::write(env.join("Vagrantfile"), "x").unwrap();
    // As a pattern `[a].txt` would also match `a.txt`.
    std::fs::write(env.join("[a].txt"), "x").unwrap();
    std::fs::write(env.join("a.txt"), "x").unwrap();
    std::fs::write(root.join("README"), "x").unwrap();

    let s = repo(&program, &env).await;
    assert_eq!(
        (s.branch.as_deref(), s.commit.as_deref()),
        (Some("main"), None)
    );
    assert!(s.root.as_deref().is_some_and(|r| !r.is_empty()));
    // README is above the folder; the untracked folder is one entry.
    let paths: Vec<_> = s.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["env/"]);
    assert!(log(&runner, &program, &env).await.unwrap().is_empty());

    stage(&runner, &program, &env, &[&find(&s, "env/")])
        .await
        .unwrap();
    let s = repo(&program, &env).await;
    assert_eq!(s.staged, 3);

    // Before the first commit, unstaging removes paths from the index.
    unstage(&runner, &program, &env, &[&find(&s, "env/[a].txt")], false)
        .await
        .unwrap();
    let s = repo(&program, &env).await;
    assert_eq!(find(&s, "env/[a].txt").kind, ChangeKind::Untracked);
    assert!(
        find(&s, "env/a.txt").is_staged(),
        "the pathspec must be literal"
    );
    unstage(&runner, &program, &env, &[&find(&s, "env/a.txt")], false)
        .await
        .unwrap();
    let s = repo(&program, &env).await;
    stage(&runner, &program, &env, &[&find(&s, "env/[a].txt")])
        .await
        .unwrap();
    let s = repo(&program, &env).await;
    assert!(find(&s, "env/[a].txt").is_staged());
    assert_eq!(find(&s, "env/a.txt").kind, ChangeKind::Untracked);

    commit(&runner, &program, &env, "Add Vagrantfile\n\nFirst commit.")
        .await
        .unwrap();
    let history = log(&runner, &program, &env).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(
        (history[0].subject.as_str(), history[0].author.as_str()),
        ("Add Vagrantfile", "Test")
    );
    let s = repo(&program, &env).await;
    assert!(s.commit.is_some());
    assert_eq!(s.files.len(), 1, "only a.txt is left: {:?}", s.files);

    std::fs::write(env.join("Vagrantfile"), "y").unwrap();
    let s = repo(&program, &env).await;
    let changed = find(&s, "env/Vagrantfile");
    assert_eq!((changed.index, changed.worktree), ('.', 'M'));
    stage(&runner, &program, &env, &[&changed]).await.unwrap();
    let staged = find(&repo(&program, &env).await, "env/Vagrantfile");
    assert_eq!((staged.index, staged.worktree), ('M', '.'));
    unstage(&runner, &program, &env, &[&staged], true)
        .await
        .unwrap();
    let back = find(&repo(&program, &env).await, "env/Vagrantfile");
    assert_eq!((back.index, back.worktree), ('.', 'M'));
    assert_eq!(
        std::fs::read_to_string(env.join("Vagrantfile")).unwrap(),
        "y"
    );
}

/// Temp folders can sit behind a symlink or 8.3 name; Git reports real paths.
fn real_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .map(|p| PathBuf::from(p.to_string_lossy().trim_start_matches(r"\\?\")))
        .unwrap()
}
