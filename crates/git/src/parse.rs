//! Parsers for `git status --porcelain=v2 --branch` and the panel's `git log`.

use crate::{ChangeKind, CommitSummary, FileChange, RepoStatus};

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
        let Some(change) = parse_entry(line) else {
            continue;
        };
        match change.kind {
            ChangeKind::Conflicted => s.conflicted += 1,
            ChangeKind::Untracked => s.untracked += 1,
            ChangeKind::Changed | ChangeKind::Renamed => {
                // X is the index, Y the work tree; `.` means unchanged.
                if change.index != '.' {
                    s.staged += 1;
                }
                if change.worktree != '.' {
                    s.modified += 1;
                }
            }
        }
        s.files.push(change);
    }
    s
}

/// One entry line: `1 XY sub mH mI mW hH hI path`, `2 XY sub mH mI mW hH hI
/// Xscore path<TAB>orig`, `u XY sub m1 m2 m3 mW h1 h2 h3 path` or `? path`.
fn parse_entry(line: &str) -> Option<FileChange> {
    let (tag, rest) = line.split_once(' ')?;
    let xy = |fields: &str| {
        let mut c = fields.chars();
        Some((c.next()?, c.next()?))
    };
    let (kind, (index, worktree), path, orig_path) = match tag {
        "1" => {
            let fields: Vec<&str> = rest.splitn(8, ' ').collect();
            (ChangeKind::Changed, xy(fields[0])?, *fields.get(7)?, None)
        }
        "2" => {
            let fields: Vec<&str> = rest.splitn(9, ' ').collect();
            let (path, orig) = fields.get(8)?.split_once('\t')?;
            (ChangeKind::Renamed, xy(fields[0])?, path, Some(orig))
        }
        "u" => {
            let fields: Vec<&str> = rest.splitn(10, ' ').collect();
            (
                ChangeKind::Conflicted,
                xy(fields[0])?,
                *fields.get(9)?,
                None,
            )
        }
        "?" => (ChangeKind::Untracked, ('?', '?'), rest, None),
        _ => return None,
    };
    Some(FileChange {
        path: unquote_path(path),
        orig_path: orig_path.map(unquote_path),
        kind,
        index,
        worktree,
    })
}

/// Undoes Git's C-style quoting of unusual paths (`"a\"b\303\251"`). Paths
/// without quotes are returned as they are.
pub fn unquote_path(raw: &str) -> String {
    let Some(inner) = raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')) else {
        return raw.to_owned();
    };
    let mut bytes = Vec::with_capacity(inner.len());
    let mut chars = inner.bytes().peekable();
    while let Some(b) = chars.next() {
        if b != b'\\' {
            bytes.push(b);
            continue;
        }
        match chars.next() {
            Some(b'n') => bytes.push(b'\n'),
            Some(b't') => bytes.push(b'\t'),
            Some(b'r') => bytes.push(b'\r'),
            Some(b'a') => bytes.push(0x07),
            Some(b'b') => bytes.push(0x08),
            Some(b'f') => bytes.push(0x0c),
            Some(b'v') => bytes.push(0x0b),
            Some(d @ b'0'..=b'7') => {
                let mut value = u32::from(d - b'0');
                for _ in 0..2 {
                    match chars.peek() {
                        Some(&n @ b'0'..=b'7') => {
                            value = value * 8 + u32::from(n - b'0');
                            chars.next();
                        }
                        _ => break,
                    }
                }
                bytes.push(u8::try_from(value).unwrap_or(b'?'));
            }
            Some(other) => bytes.push(other),
            None => bytes.push(b'\\'),
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Parses `git log --format=%h%x1f%an%x1f%aI%x1f%s`.
pub fn parse_log(stdout: &[String]) -> Vec<CommitSummary> {
    stdout
        .iter()
        .filter_map(|line| {
            let mut f = line.splitn(4, '\u{1f}');
            Some(CommitSummary {
                id: f.next()?.to_owned(),
                author: f.next()?.to_owned(),
                date: f.next()?.to_owned(),
                subject: f.next()?.to_owned(),
            })
        })
        .collect()
}
