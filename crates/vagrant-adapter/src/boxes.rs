//! The host's box store: typed box arguments, `vagrant box outdated --global`
//! parsing, and which machines use a box according to Vagrant's machine index.
//!
//! Box arguments come from the UI, so each one is checked to be a single,
//! harmless argv item: never empty, never starting with `-`, no control
//! characters. Vagrant itself reports anything else that is wrong.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::machine_readable::Line;

/// `box list` reports this when a box has no architecture (old box format).
const NO_ARCHITECTURE: &str = "n/a";

fn argument(value: &str, what: &str, max: usize) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("The {what} is required."));
    }
    if value.starts_with('-') {
        return Err(format!("The {what} can't start with “-”."));
    }
    if value.len() > max || value.chars().any(char::is_control) {
        return Err(format!("“{value}” is not a valid {what}."));
    }
    Ok(value.to_owned())
}

/// Provider and architecture ids: `virtualbox`, `vmware_desktop`, `amd64`.
fn identifier(value: &str, what: &str) -> Result<String, String> {
    let value = argument(value, what, 64)?;
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
    {
        Ok(value)
    } else {
        Err(format!("“{value}” is not a valid {what}."))
    }
}

/// A box version such as `9.6.20250522`, or for `box add` a constraint
/// such as `>= 1.0, < 2`.
fn version(value: &str) -> Result<String, String> {
    let value = argument(value, "version", 100)?;
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || " .,<>=~!+_-".contains(c))
    {
        Ok(value)
    } else {
        Err(format!("“{value}” is not a valid version."))
    }
}

/// One installed box: a name, provider, version and architecture as listed
/// by `vagrant box list`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "BoxRefInput")]
pub struct BoxRef {
    pub name: String,
    pub provider: String,
    pub version: String,
    /// `None` for boxes without an architecture (`n/a` in `box list`).
    pub architecture: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BoxRefInput {
    name: String,
    provider: String,
    version: String,
    architecture: Option<String>,
}

impl TryFrom<BoxRefInput> for BoxRef {
    type Error = String;

    fn try_from(b: BoxRefInput) -> Result<Self, String> {
        Ok(Self {
            name: argument(&b.name, "box name", 255)?,
            provider: identifier(&b.provider, "provider")?,
            version: version(&b.version)?,
            architecture: match b.architecture.as_deref().map(str::trim) {
                None | Some("") | Some(NO_ARCHITECTURE) => None,
                Some(a) => Some(identifier(a, "architecture")?),
            },
        })
    }
}

impl BoxRef {
    pub fn label(&self) -> String {
        let mut label = format!("{} ({}, {}", self.name, self.provider, self.version);
        if let Some(a) = &self.architecture {
            label.push_str(", ");
            label.push_str(a);
        }
        label.push(')');
        label
    }
}

/// Where `vagrant box add` gets a box from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum BoxSource {
    /// A catalog name such as `bento/ubuntu-24.04`.
    Catalog(String),
    /// An `http(s)` URL of a `.box` file or catalog metadata.
    Url(String),
    /// A local `.box` file.
    File(PathBuf),
}

/// A validated `vagrant box add` request.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoxAdd {
    pub source: BoxSource,
    pub provider: Option<String>,
    pub version: Option<String>,
    pub architecture: Option<String>,
    /// The name to give a box added from a file or URL.
    pub name: Option<String>,
}

/// What the UI sends to add a box.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxAddRequest {
    pub source: BoxSource,
    pub provider: Option<String>,
    pub version: Option<String>,
    pub architecture: Option<String>,
    pub name: Option<String>,
}

fn optional(
    value: Option<&str>,
    check: impl Fn(&str) -> Result<String, String>,
) -> Result<Option<String>, String> {
    match value.map(str::trim) {
        None | Some("") => Ok(None),
        Some(v) => check(v).map(Some),
    }
}

impl TryFrom<BoxAddRequest> for BoxAdd {
    type Error = String;

    fn try_from(r: BoxAddRequest) -> Result<Self, String> {
        let source = match r.source {
            BoxSource::Catalog(name) => {
                let name = argument(&name, "box name", 255)?;
                let valid = name.split_once('/').is_some_and(|(owner, box_name)| {
                    [owner, box_name].iter().all(|part| {
                        !part.is_empty()
                            && part
                                .chars()
                                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
                    })
                });
                if !valid {
                    return Err(format!(
                        "“{name}” is not a catalog box name. Use the form owner/box, for example bento/ubuntu-24.04."
                    ));
                }
                BoxSource::Catalog(name)
            }
            BoxSource::Url(url) => {
                let url = argument(&url, "URL", 2000)?;
                let rest = url
                    .strip_prefix("https://")
                    .or_else(|| url.strip_prefix("http://"))
                    .ok_or("The URL must start with https:// or http://.")?;
                let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
                if authority.is_empty() {
                    return Err("The URL has no host.".into());
                }
                if authority.contains('@') {
                    return Err(
                        "URLs with a user name or password aren't accepted, because they would be saved in logs.".into(),
                    );
                }
                if url.chars().any(char::is_whitespace) {
                    return Err("The URL can't contain spaces.".into());
                }
                BoxSource::Url(url)
            }
            BoxSource::File(path) => {
                let text = path.to_str().ok_or("The file path is not valid UTF-8.")?;
                if text.starts_with('-') || !path.is_absolute() {
                    return Err("Choose the .box file with its full path.".into());
                }
                if !path.is_file() {
                    return Err(format!("{} doesn't exist or isn't a file.", path.display()));
                }
                BoxSource::File(path)
            }
        };
        let name = optional(r.name.as_deref(), |v| argument(v, "box name", 255))?;
        if matches!(source, BoxSource::File(_)) && name.is_none() {
            return Err("Give the box a name, for example my/box.".into());
        }
        Ok(Self {
            name: if matches!(source, BoxSource::Catalog(_)) {
                None
            } else {
                name
            },
            provider: optional(r.provider.as_deref(), |v| identifier(v, "provider"))?,
            version: optional(r.version.as_deref(), version)?,
            architecture: optional(r.architecture.as_deref(), |v| identifier(v, "architecture"))?,
            source,
        })
    }
}

/// Result of checking one installed box for a newer version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum OutdatedState {
    UpToDate {
        version: String,
    },
    Outdated {
        current: String,
        latest: String,
    },
    /// Added from a file or URL without catalog metadata.
    NoMetadata,
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutdatedBox {
    pub name: String,
    pub provider: String,
    #[serde(flatten)]
    pub state: OutdatedState,
}

/// Parses `vagrant box outdated --global --machine-readable`. Vagrant 2.4.9
/// prints only `ui` lines for it, one per box, from these templates:
///
/// - `* 'NAME' for 'PROVIDER' (vVERSION) is up to date`
/// - `* 'NAME' for 'PROVIDER' is outdated! Current: X. Latest: Y`
/// - `* 'NAME' for 'PROVIDER' wasn't added from a catalog, no version information`
/// - `* 'NAME' for 'PROVIDER': Error loading metadata: MESSAGE`
pub fn parse_outdated(lines: &[Line]) -> Vec<OutdatedBox> {
    lines
        .iter()
        .filter(|l| l.kind == "ui")
        .filter_map(|l| parse_outdated_message(l.data.get(1)?))
        .collect()
}

fn parse_outdated_message(text: &str) -> Option<OutdatedBox> {
    let rest = text.trim().strip_prefix("* '")?;
    let (name, rest) = rest.split_once("' for '")?;
    let (provider, rest) = rest.split_once('\'')?;
    let state = if let Some(message) = rest.strip_prefix(": Error loading metadata: ") {
        OutdatedState::Error {
            message: message.trim().to_owned(),
        }
    } else if rest.starts_with(" wasn't added from a catalog") {
        OutdatedState::NoMetadata
    } else if let Some(versions) = rest.strip_prefix(" is outdated! Current: ") {
        let (current, latest) = versions.split_once(". Latest: ")?;
        OutdatedState::Outdated {
            current: current.to_owned(),
            latest: latest.trim().to_owned(),
        }
    } else {
        let version = rest
            .strip_prefix(" (v")?
            .strip_suffix(") is up to date")?
            .to_owned();
        OutdatedState::UpToDate { version }
    };
    Some(OutdatedBox {
        name: name.to_owned(),
        provider: provider.to_owned(),
        state,
    })
}

/// A machine from Vagrant's global machine index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedMachine {
    pub name: String,
    pub provider: String,
    /// The state Vagrant last recorded; may be out of date.
    pub state: Option<String>,
    /// The folder of the machine's Vagrantfile, as Vagrant recorded it.
    pub vagrantfile_path: String,
    pub box_name: Option<String>,
    pub box_provider: Option<String>,
    pub box_version: Option<String>,
    pub box_architecture: Option<String>,
}

impl IndexedMachine {
    /// Whether the machine was created from `b`, using Vagrant's rule
    /// (name, provider, version and architecture match). An architecture
    /// recorded as `auto` or missing counts as a match, so a box is never
    /// reported unused when it might be in use.
    pub fn uses(&self, b: &BoxRef) -> bool {
        self.box_name.as_deref() == Some(b.name.as_str())
            && self.box_provider.as_deref() == Some(b.provider.as_str())
            && self.box_version.as_deref() == Some(b.version.as_str())
            && match (self.box_architecture.as_deref(), b.architecture.as_deref()) {
                (None | Some("auto"), _) => true,
                (Some(a), Some(b)) => a == b,
                (Some(_), None) => false,
            }
    }
}

/// Vagrant's data folder: `VAGRANT_HOME`, else `~/.vagrant.d`.
pub fn vagrant_home() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("VAGRANT_HOME").filter(|h| !h.is_empty()) {
        return Some(PathBuf::from(home));
    }
    let home = if cfg!(windows) {
        std::env::var_os("USERPROFILE")
    } else {
        std::env::var_os("HOME")
    }?;
    Some(Path::new(&home).join(".vagrant.d"))
}

/// Reads `data/machine-index/index` under Vagrant's home. Read-only; a
/// missing index means Vagrant hasn't created any machine yet.
pub fn read_machine_index(vagrant_home: &Path) -> std::io::Result<Vec<IndexedMachine>> {
    let path = vagrant_home
        .join("data")
        .join("machine-index")
        .join("index");
    match std::fs::read_to_string(&path) {
        Ok(text) => parse_machine_index(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

#[derive(Deserialize)]
struct Index {
    #[serde(default)]
    machines: std::collections::BTreeMap<String, IndexEntry>,
}

#[derive(Deserialize)]
struct IndexEntry {
    name: Option<String>,
    provider: Option<String>,
    state: Option<String>,
    vagrantfile_path: Option<String>,
    #[serde(default)]
    extra_data: Option<ExtraData>,
}

#[derive(Deserialize)]
struct ExtraData {
    #[serde(rename = "box")]
    box_data: Option<IndexBox>,
}

#[derive(Deserialize)]
struct IndexBox {
    name: Option<String>,
    provider: Option<String>,
    version: Option<String>,
    architecture: Option<String>,
}

pub fn parse_machine_index(text: &str) -> Result<Vec<IndexedMachine>, serde_json::Error> {
    let index: Index = serde_json::from_str(text)?;
    Ok(index
        .machines
        .into_values()
        .filter_map(|e| {
            let b = e.extra_data.and_then(|x| x.box_data);
            Some(IndexedMachine {
                name: e.name?,
                provider: e.provider.unwrap_or_default(),
                state: e.state,
                vagrantfile_path: e.vagrantfile_path?,
                box_name: b.as_ref().and_then(|b| b.name.clone()),
                box_provider: b.as_ref().and_then(|b| b.provider.clone()),
                box_version: b.as_ref().and_then(|b| b.version.clone()),
                box_architecture: b.and_then(|b| b.architecture),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine_readable::parse;

    fn fixture(name: &str) -> String {
        let path = format!(
            "{}/../../fixtures/command-output/windows-vagrant-2.4.9/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    fn box_ref(arch: Option<&str>) -> BoxRef {
        serde_json::from_value(serde_json::json!({
            "name": "almalinux/9", "provider": "vmware_desktop",
            "version": "9.6.20250522", "architecture": arch,
        }))
        .unwrap()
    }

    fn add(source: BoxSource) -> Result<BoxAdd, String> {
        BoxAdd::try_from(BoxAddRequest {
            source,
            provider: Some("virtualbox".into()),
            version: None,
            architecture: None,
            name: None,
        })
    }

    #[test]
    fn box_refs_reject_option_like_and_odd_values() {
        assert_eq!(box_ref(Some("n/a")).architecture, None);
        assert_eq!(
            box_ref(Some("amd64")).architecture.as_deref(),
            Some("amd64")
        );
        for (field, bad) in [
            ("name", "--all"),
            ("name", ""),
            ("provider", "virtual box"),
            ("version", "-1"),
            ("version", "1;rm"),
            ("architecture", "--x"),
        ] {
            let mut v = serde_json::json!({
                "name": "a/b", "provider": "virtualbox", "version": "1.0", "architecture": null,
            });
            v[field] = bad.into();
            assert!(
                serde_json::from_value::<BoxRef>(v).is_err(),
                "{field}={bad}"
            );
        }
    }

    #[test]
    fn add_requests_are_checked() {
        assert!(add(BoxSource::Catalog("bento/ubuntu-24.04".into())).is_ok());
        for bad in ["ubuntu", "-f/x", "a/b c", "a/../b/c"] {
            assert!(add(BoxSource::Catalog(bad.into())).is_err(), "{bad}");
        }
        assert!(add(BoxSource::Url("https://example.com/x.box".into())).is_ok());
        for bad in [
            "ftp://example.com/x.box",
            "https://user:pw@example.com/x.box",
            "https:///x.box",
            "https://example.com/a b.box",
        ] {
            assert!(add(BoxSource::Url(bad.into())).is_err(), "{bad}");
        }
        assert!(add(BoxSource::File("relative.box".into())).is_err());

        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("my.box");
        std::fs::write(&file, "").unwrap();
        assert!(
            add(BoxSource::File(file.clone()))
                .unwrap_err()
                .contains("name")
        );
        let ok = BoxAdd::try_from(BoxAddRequest {
            source: BoxSource::File(file),
            provider: None,
            version: Some(" ".into()),
            architecture: None,
            name: Some("my/box".into()),
        })
        .unwrap();
        assert_eq!((ok.name.as_deref(), ok.version), (Some("my/box"), None));
    }

    #[test]
    fn outdated_global_fixture() {
        let boxes = parse_outdated(&parse(&fixture("box-outdated-global.txt")));
        assert_eq!(boxes.len(), 4);
        assert_eq!(
            boxes[0],
            OutdatedBox {
                name: "ubuntu/jammy64".into(),
                provider: "virtualbox".into(),
                state: OutdatedState::UpToDate {
                    version: "20241002.0.0".into()
                },
            }
        );
        assert_eq!(
            boxes[3].state,
            OutdatedState::Outdated {
                current: "9.6.20250522".into(),
                latest: "9.8.20260907".into()
            }
        );
    }

    #[test]
    fn outdated_messages_without_versions() {
        assert_eq!(
            parse_outdated_message(
                "* 'my/box' for 'virtualbox' wasn't added from a catalog, no version information"
            )
            .unwrap()
            .state,
            OutdatedState::NoMetadata
        );
        assert_eq!(
            parse_outdated_message("* 'a/b' for 'virtualbox': Error loading metadata: timeout")
                .unwrap()
                .state,
            OutdatedState::Error {
                message: "timeout".into()
            }
        );
        assert_eq!(parse_outdated_message("Checking for updates..."), None);
    }

    #[test]
    fn machine_index_and_box_usage() {
        let machines = parse_machine_index(&fixture("machine-index.json")).unwrap();
        assert_eq!(machines.len(), 3);
        let alma = machines
            .iter()
            .find(|m| m.provider == "vmware_desktop")
            .unwrap();
        assert_eq!(alma.vagrantfile_path, "D:/Projects/alma-lab");
        assert_eq!(alma.state.as_deref(), Some("not running"));
        // Recorded as `auto`, so it matches the installed amd64 box.
        assert!(alma.uses(&box_ref(Some("amd64"))));
        let mut other_version = box_ref(Some("amd64"));
        other_version.version = "9.8.20260907".into();
        assert!(!alma.uses(&other_version));
        assert_eq!(
            machines
                .iter()
                .filter(|m| m.uses(&box_ref(Some("amd64"))))
                .count(),
            1
        );
        assert!(parse_machine_index("{\"version\":1}").unwrap().is_empty());
        assert!(parse_machine_index("not json").is_err());
    }

    #[test]
    fn missing_index_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_machine_index(dir.path()).unwrap().is_empty());
    }
}
