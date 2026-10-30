//! `vagrant snapshot` names and list output.

use serde::{Deserialize, Serialize};

use crate::machine_readable::Line;

/// A snapshot name: always a separate argument, so spaces are fine, but it
/// can't read as an option or hide line breaks in logs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SnapshotName(String);

impl SnapshotName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SnapshotName {
    type Error = String;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        let valid = !name.is_empty()
            && name.chars().count() <= 100
            && !name.starts_with('-')
            && name.trim() == name
            && !name.chars().any(char::is_control);
        if valid {
            Ok(Self(name))
        } else {
            Err(format!("\"{name}\" can't be used as a snapshot name"))
        }
    }
}

impl From<SnapshotName> for String {
    fn from(name: SnapshotName) -> Self {
        name.0
    }
}

/// Names from `vagrant snapshot list --machine-readable <machine>`. Vagrant
/// prints a bare `==> machine:` `ui,output` line and then each name as a
/// `ui,detail` line. Its "no snapshots" help also uses `ui,detail`, but after
/// an `output` line with text, so only lines after a bare header count.
pub fn parse_snapshots(lines: &[Line], machine: &str) -> Vec<String> {
    let header = format!("==> {machine}:");
    let mut names = Vec::new();
    let mut listing = false;
    for line in lines
        .iter()
        .filter(|l| l.kind == "ui" && l.target == machine)
    {
        let text = line.data.get(1).map_or("", |t| t.trim());
        match line.data.first().map(String::as_str) {
            Some("output") => listing = text == header,
            Some("detail") if listing && !text.is_empty() => names.push(text.to_owned()),
            _ => {}
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine_readable::parse;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/../../fixtures/command-output/windows-vagrant-2.4.9/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    #[test]
    fn lists_names_of_the_requested_machine_only() {
        let lines = parse(&fixture("snapshot-list-multi.txt"));
        assert_eq!(parse_snapshots(&lines, "web"), ["after setup", "base"]);
        assert!(parse_snapshots(&lines, "db").is_empty());
    }

    #[test]
    fn no_snapshots_or_no_machine_is_an_empty_list() {
        for (file, machine) in [
            ("snapshot-list-none.txt", "web"),
            ("snapshot-list-not-created.txt", "web"),
        ] {
            assert!(parse_snapshots(&parse(&fixture(file)), machine).is_empty());
        }
    }

    #[test]
    fn names_stay_separate_arguments_and_cant_be_options() {
        for bad in ["", "-f", " base", "a\nb", "--force"] {
            assert!(SnapshotName::try_from(bad.to_owned()).is_err(), "{bad:?}");
        }
        let name = SnapshotName::try_from("after setup".to_owned()).unwrap();
        let machine = crate::MachineName::try_from("web".to_owned()).unwrap();
        let command = crate::VagrantCommand::SnapshotRestore { machine, name };
        assert_eq!(
            command.args(),
            [
                "snapshot",
                "restore",
                "--no-provision",
                "--no-tty",
                "web",
                "after setup"
            ]
        );
        assert!(command.is_mutating() && command.is_destructive());
    }
}
