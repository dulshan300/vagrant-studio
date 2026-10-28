//! Parsers for `vagrant … --machine-readable` output.
//!
//! Each line is `timestamp,target,type,data…`. Within fields Vagrant escapes
//! commas as `%!(VAGRANT_COMMA)` and newlines as a literal `\n`; `\r` may
//! precede the line break on Windows. Unknown line types are ignored so newer
//! Vagrant versions don't break parsing.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub timestamp: Option<u64>,
    pub target: String,
    pub kind: String,
    pub data: Vec<String>,
}

/// Parses one machine-readable line; returns `None` for anything else
/// (blank lines, plain-text output such as warnings printed to stderr).
pub fn parse_line(line: &str) -> Option<Line> {
    let line = line.trim_end_matches(['\r', '\n']);
    let mut fields = line.split(',');
    let timestamp = fields.next()?;
    if timestamp.is_empty() || !timestamp.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let target = fields.next()?;
    let kind = fields.next()?;
    if kind.is_empty() {
        return None;
    }
    Some(Line {
        timestamp: timestamp.parse().ok(),
        target: unescape(target),
        kind: kind.to_owned(),
        data: fields.map(unescape).collect(),
    })
}

pub fn parse(output: &str) -> Vec<Line> {
    output.lines().filter_map(parse_line).collect()
}

fn unescape(field: &str) -> String {
    field.replace("%!(VAGRANT_COMMA)", ",").replace("\\n", "\n")
}

/// A Vagrant `error-exit` record: the command failed with this error class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VagrantError {
    /// Ruby error class, e.g. `Vagrant::Errors::MachineNotFound`.
    pub class: String,
    pub message: String,
}

pub fn error_exit(lines: &[Line]) -> Option<VagrantError> {
    lines
        .iter()
        .find(|l| l.kind == "error-exit")
        .map(|l| VagrantError {
            class: l.data.first().cloned().unwrap_or_default(),
            message: l.data.get(1).cloned().unwrap_or_default(),
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineStatus {
    pub name: String,
    pub provider: Option<String>,
    /// Vagrant's state id, e.g. `running`, `poweroff`, `not_created`, `saved`.
    pub state: Option<String>,
    pub state_short: Option<String>,
    pub state_long: Option<String>,
}

/// Machines in the order Vagrant first mentions them.
pub fn parse_status(lines: &[Line]) -> Vec<MachineStatus> {
    let mut machines: Vec<MachineStatus> = Vec::new();
    for line in lines.iter().filter(|l| !l.target.is_empty()) {
        let machine = match machines.iter_mut().position(|m| m.name == line.target) {
            Some(i) => &mut machines[i],
            None => {
                machines.push(MachineStatus {
                    name: line.target.clone(),
                    provider: None,
                    state: None,
                    state_short: None,
                    state_long: None,
                });
                machines.last_mut().expect("just pushed")
            }
        };
        let value = || line.data.last().cloned();
        match (line.kind.as_str(), line.data.first().map(String::as_str)) {
            ("provider-name", _) => machine.provider = value(),
            ("metadata", Some("provider")) if machine.provider.is_none() => {
                machine.provider = value()
            }
            ("state", _) => machine.state = value(),
            ("state-human-short", _) => machine.state_short = value(),
            ("state-human-long", _) => machine.state_long = value(),
            _ => {}
        }
    }
    machines
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxInfo {
    pub name: String,
    pub provider: Option<String>,
    pub version: Option<String>,
    pub architecture: Option<String>,
}

/// `box-name` starts a record; following provider/version/architecture lines belong to it.
pub fn parse_box_list(lines: &[Line]) -> Vec<BoxInfo> {
    let mut boxes: Vec<BoxInfo> = Vec::new();
    for line in lines {
        let value = line.data.first().cloned();
        match line.kind.as_str() {
            "box-name" => boxes.push(BoxInfo {
                name: value.unwrap_or_default(),
                provider: None,
                version: None,
                architecture: None,
            }),
            "box-provider" => set_last(&mut boxes, |b| b.provider = value),
            "box-version" => set_last(&mut boxes, |b| b.version = value),
            "box-architecture" => set_last(&mut boxes, |b| b.architecture = value),
            _ => {}
        }
    }
    boxes
}

fn set_last<T>(items: &mut [T], f: impl FnOnce(&mut T)) {
    if let Some(last) = items.last_mut() {
        f(last);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub name: String,
    pub version: Option<String>,
    /// `global` or `local`, when reported.
    pub scope: Option<String>,
}

pub fn parse_plugin_list(lines: &[Line]) -> Vec<PluginInfo> {
    let mut plugins: Vec<PluginInfo> = Vec::new();
    for line in lines {
        match line.kind.as_str() {
            "plugin-name" => plugins.push(PluginInfo {
                name: line.data.first().cloned().unwrap_or_default(),
                version: None,
                scope: None,
            }),
            // Data looks like "3.0.5, global" once unescaped.
            "plugin-version" => {
                if let Some(p) = plugins.iter_mut().rev().find(|p| p.name == line.target) {
                    let raw = line.data.first().cloned().unwrap_or_default();
                    let mut parts = raw.splitn(2, ',').map(str::trim);
                    p.version = parts.next().filter(|v| !v.is_empty()).map(str::to_owned);
                    p.scope = parts.next().map(str::to_owned);
                }
            }
            _ => {}
        }
    }
    plugins
}

/// Parses `vagrant --version` output such as `Vagrant 2.4.9`.
pub fn parse_version(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let version = line.trim().strip_prefix("Vagrant ")?.trim();
        let valid = version
            .split('.')
            .take(2)
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()));
        valid.then(|| version.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        let path = format!(
            "{}/../../fixtures/command-output/windows-vagrant-2.4.9/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    #[test]
    fn fixtures_keep_windows_line_endings() {
        assert!(fixture("box-list.txt").contains("\r\n"));
    }

    #[test]
    fn unescapes_commas_and_newlines() {
        let line = parse_line("1,web,ui,info,a%!(VAGRANT_COMMA) b\\nc\r\n").unwrap();
        assert_eq!(line.target, "web");
        assert_eq!(line.data, vec!["info", "a, b\nc"]);
        assert_eq!(parse_line("==> web: Booting VM..."), None);
        assert_eq!(parse_line(""), None);
    }

    #[test]
    fn status_multi_machine_virtualbox() {
        let machines = parse_status(&parse(&fixture("status-multi-not-created-virtualbox.txt")));
        assert_eq!(machines.len(), 2);
        assert_eq!(machines[0].name, "web");
        assert_eq!(machines[1].name, "db");
        for m in &machines {
            assert_eq!(m.provider.as_deref(), Some("virtualbox"));
            assert_eq!(m.state.as_deref(), Some("not_created"));
            assert_eq!(m.state_short.as_deref(), Some("not created"));
            assert!(m.state_long.as_deref().unwrap().contains("vagrant up"));
        }
    }

    #[test]
    fn status_multi_machine_vmware() {
        let lines = parse(&fixture("status-multi-not-created-vmware.txt"));
        assert_eq!(error_exit(&lines), None);
        let machines = parse_status(&lines);
        assert_eq!(
            machines
                .iter()
                .map(|m| (m.name.as_str(), m.provider.as_deref(), m.state.as_deref()))
                .collect::<Vec<_>>(),
            [
                ("web", Some("vmware_desktop"), Some("not_created")),
                ("db", Some("vmware_desktop"), Some("not_created"))
            ]
        );
    }

    #[test]
    fn status_errors_are_reported() {
        let unreachable = parse(&fixture("status-vmware-utility-unreachable.txt"));
        let err = error_exit(&unreachable).unwrap();
        assert_eq!(
            err.class,
            "HashiCorp::VagrantVMwareDesktop::Errors::DriverAPIRequestUnexpectedError"
        );
        assert!(err.message.contains("127.0.0.1:9922"));
        assert!(parse_status(&unreachable).is_empty());

        let missing = error_exit(&parse(&fixture("status-unknown-machine.txt"))).unwrap();
        assert_eq!(missing.class, "Vagrant::Errors::MachineNotFound");
        let no_env = error_exit(&parse(&fixture("status-no-vagrantfile.txt"))).unwrap();
        assert_eq!(no_env.class, "Vagrant::Errors::NoEnvironmentError");
    }

    #[test]
    fn box_list() {
        let boxes = parse_box_list(&parse(&fixture("box-list.txt")));
        assert_eq!(boxes.len(), 4);
        assert_eq!(
            boxes[0],
            BoxInfo {
                name: "almalinux/9".into(),
                provider: Some("vmware_desktop".into()),
                version: Some("9.6.20250522".into()),
                architecture: Some("amd64".into()),
            }
        );
        assert_eq!(boxes[1].name, "hashicorp/bionic64");
        assert_eq!(boxes[1].provider.as_deref(), Some("virtualbox"));
    }

    #[test]
    fn plugin_list() {
        let plugins = parse_plugin_list(&parse(&fixture("plugin-list.txt")));
        assert_eq!(
            plugins,
            vec![PluginInfo {
                name: "vagrant-vmware-desktop".into(),
                version: Some("3.0.5".into()),
                scope: Some("global".into()),
            }]
        );
    }

    #[test]
    fn version() {
        assert_eq!(
            parse_version(&fixture("version.txt")).as_deref(),
            Some("2.4.9")
        );
        assert_eq!(
            parse_version("Vagrant 2.4.9.dev\n").as_deref(),
            Some("2.4.9.dev")
        );
        assert_eq!(parse_version("bash: vagrant: not found"), None);
        assert_eq!(parse_version("Vagrant x.y"), None);
    }
}
