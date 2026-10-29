//! Provisioner runs read from the human output of `up`, `reload` and
//! `provision` (`--no-tty`). Vagrant has no machine-readable form of this, so
//! the parser relies only on the step headers Vagrant prints:
//!
//! ```text
//! ==> web: Running provisioner: shell...          (unnamed)
//! ==> web: Running provisioner: setup (shell)...  (named)
//!     web: Running: inline script                 (guest output follows)
//! ```
//!
//! Vagrant stops at the first provisioner that fails and only prints a
//! generic error to stderr, so a provisioner counts as finished once any later
//! step header appears, and the one still running when the command ends is
//! the one that failed or was stopped. Providers that bring machines up in
//! parallel (docker) can interleave headers; the result is then approximate.

use serde::{Deserialize, Serialize};

/// Guest output lines kept per provisioner; the operation log has all of them.
pub const OUTPUT_LINES: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvisionerStatus {
    Succeeded,
    Failed,
    /// Still running when the command was cancelled or timed out.
    Unfinished,
}

/// How the Vagrant command ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandEnd {
    Succeeded,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionerRun {
    pub machine: String,
    /// `None` for unnamed provisioners, which Vagrant labels by type.
    pub name: Option<String>,
    pub kind: String,
    pub status: ProvisionerStatus,
    /// The last [`OUTPUT_LINES`] lines Vagrant printed for this provisioner,
    /// without the machine prefix.
    pub output: Vec<String>,
    /// Whether earlier output lines were dropped.
    pub truncated: bool,
}

/// Every provisioner Vagrant started, in the order it started them.
pub fn parse(stdout: &[String], end: CommandEnd) -> Vec<ProvisionerRun> {
    let mut runs: Vec<ProvisionerRun> = Vec::new();
    let mut open: Option<usize> = None;
    for raw in stdout {
        let line = raw.trim_end_matches(['\r', '\n']);
        if let Some(step) = line.strip_prefix("==> ") {
            open = None;
            let Some((machine, text)) = step.split_once(": ") else {
                continue;
            };
            if let Some((name, kind)) = text
                .strip_prefix("Running provisioner: ")
                .and_then(|t| t.strip_suffix("..."))
                .map(name_and_kind)
            {
                runs.push(ProvisionerRun {
                    machine: machine.to_owned(),
                    name,
                    kind,
                    status: ProvisionerStatus::Succeeded,
                    output: Vec::new(),
                    truncated: false,
                });
                open = Some(runs.len() - 1);
            }
        } else if let Some(i) = open
            && let Some(text) = line
                .strip_prefix("    ")
                .and_then(|l| l.strip_prefix(runs[i].machine.as_str()))
                .and_then(|l| l.strip_prefix(':'))
        {
            let run = &mut runs[i];
            run.output
                .push(text.strip_prefix(' ').unwrap_or(text).to_owned());
            if run.output.len() > OUTPUT_LINES {
                run.output.remove(0);
                run.truncated = true;
            }
        }
    }
    if let Some(i) = open {
        runs[i].status = match end {
            CommandEnd::Succeeded => ProvisionerStatus::Succeeded,
            CommandEnd::Failed => ProvisionerStatus::Failed,
            CommandEnd::Stopped => ProvisionerStatus::Unfinished,
        };
    }
    runs
}

/// `setup (shell)` → named; `shell` → unnamed, labelled by its type.
fn name_and_kind(text: &str) -> (Option<String>, String) {
    if let Some(rest) = text.strip_suffix(')')
        && let Some((name, kind)) = rest.rsplit_once(" (")
        && !kind.is_empty()
        && kind.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
    {
        return (Some(name.to_owned()), kind.to_owned());
    }
    (None, text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/command-output/windows-vagrant-2.4.9/"
    );

    fn fixture(name: &str) -> Vec<String> {
        let text = std::fs::read_to_string(format!("{DIR}{name}")).unwrap();
        assert!(text.contains("\r\n"), "{name} lost its CRLF line endings");
        text.split('\n').map(str::to_owned).collect()
    }

    fn summary(runs: &[ProvisionerRun]) -> Vec<(String, Option<String>, ProvisionerStatus)> {
        runs.iter()
            .map(|r| (r.machine.clone(), r.name.clone(), r.status))
            .collect()
    }

    fn row(
        m: &str,
        name: Option<&str>,
        s: ProvisionerStatus,
    ) -> (String, Option<String>, ProvisionerStatus) {
        (m.to_owned(), name.map(str::to_owned), s)
    }

    #[test]
    fn first_up_provisions_each_machine_in_turn() {
        use ProvisionerStatus::Succeeded as S;
        let runs = parse(&fixture("up-provision-multi.txt"), CommandEnd::Succeeded);
        assert_eq!(
            summary(&runs),
            [
                row("web", None, S),
                row("web", Some("hello"), S),
                row("web", Some("setup"), S),
                row("db", None, S),
                row("db", Some("hello"), S),
                row("db", Some("always"), S),
            ]
        );
        assert_eq!(runs[0].kind, "shell");
        assert_eq!(
            runs[0].output,
            ["Running: inline script", "shared unnamed on web"]
        );
        // Boot messages before the first provisioner are not attributed to it.
        assert!(
            runs.iter()
                .all(|r| r.output.iter().all(|l| !l.contains("SSH")))
        );
        assert_eq!(runs[2].output.last().unwrap(), "args:");
    }

    #[test]
    fn provision_with_runs_only_the_chosen_provisioners() {
        let runs = parse(&fixture("provision-with.txt"), CommandEnd::Succeeded);
        let names: Vec<_> = runs.iter().map(|r| r.name.as_deref()).collect();
        assert_eq!(names, [Some("hello"), Some("setup")]);
    }

    #[test]
    fn the_running_provisioner_is_the_one_that_failed() {
        let runs = parse(&fixture("provision-fail.txt"), CommandEnd::Failed);
        assert_eq!(
            summary(&runs),
            [row("web", Some("fail"), ProvisionerStatus::Failed)]
        );
        assert_eq!(
            runs[0].output,
            ["Running: inline script", "about to fail", "something broke"]
        );
    }

    #[test]
    fn earlier_provisioners_finished_when_a_later_step_fails() {
        let lines: Vec<String> = [
            "==> web: Running provisioner: shell...",
            "    web: ok",
            "==> db: Booting VM...",
            "    db: Timed out while waiting for the machine to boot.",
        ]
        .map(str::to_owned)
        .to_vec();
        let runs = parse(&lines, CommandEnd::Failed);
        assert_eq!(
            summary(&runs),
            [row("web", None, ProvisionerStatus::Succeeded)]
        );
        assert_eq!(runs[0].output, ["ok"]);
    }

    #[test]
    fn stopped_commands_leave_the_last_provisioner_unfinished() {
        let lines = ["==> default: Running provisioner: a b (c) (shell)...".to_owned()];
        let runs = parse(&lines, CommandEnd::Stopped);
        assert_eq!(runs[0].name.as_deref(), Some("a b (c)"));
        assert_eq!(runs[0].status, ProvisionerStatus::Unfinished);
    }

    #[test]
    fn keeps_the_last_output_lines() {
        let mut lines = vec!["==> web: Running provisioner: shell...".to_owned()];
        lines.extend((0..100).map(|i| format!("    web: line {i}")));
        lines.push("    web:".to_owned());
        lines.push("    other: not ours".to_owned());
        let run = &parse(&lines, CommandEnd::Succeeded)[0];
        assert_eq!(run.output.len(), OUTPUT_LINES);
        assert!(run.truncated);
        assert_eq!(run.output.last().unwrap(), "");
        assert_eq!(run.output[0], "line 61");
    }
}
