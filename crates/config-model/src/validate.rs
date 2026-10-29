//! Checks a model before it is turned into a Vagrantfile. Errors name the
//! field by its camelCase path (`machines.0.settings.forwardedPorts.1.host`)
//! so the forms can show them next to the input.

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use serde::Serialize;

use crate::model::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldError {
    pub path: String,
    pub message: String,
}

impl FieldError {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

pub const MAX_CPUS: u32 = 256;
pub const MIN_MEMORY_MB: u32 = 64;
pub const MAX_MEMORY_MB: u32 = 1024 * 1024;

pub fn validate(config: &VagrantConfig) -> Vec<FieldError> {
    let mut errors = Vec::new();
    settings(&mut errors, "defaults", &config.defaults);

    let mut names: HashMap<String, usize> = HashMap::new();
    for (i, m) in config.machines.iter().enumerate() {
        let path = format!("machines.{i}");
        if !valid_machine_name(&m.name) {
            errors.push(FieldError::new(
                format!("{path}.name"),
                "Use letters, digits, '-', '_' or '.', not starting with '-'.",
            ));
        } else if let Some(first) = names.insert(m.name.to_lowercase(), i) {
            errors.push(FieldError::new(
                format!("{path}.name"),
                format!(
                    "Machine {} already uses this name (names are compared without case).",
                    first + 1
                ),
            ));
        }
        settings(&mut errors, &format!("{path}.settings"), &m.settings);
        if m.settings.box_name.is_none() && config.defaults.box_name.is_none() {
            errors.push(FieldError::new(
                format!("{path}.settings.box"),
                "Choose a box here or in the shared settings.",
            ));
        }
    }
    if config.machines.is_empty() && config.defaults.box_name.is_none() {
        errors.push(FieldError::new("defaults.box", "Choose a box."));
    }
    if let Some(second) = config
        .machines
        .iter()
        .enumerate()
        .filter(|(_, m)| m.primary)
        .nth(1)
    {
        errors.push(FieldError::new(
            format!("machines.{}.primary", second.0),
            "Only one machine can be primary.",
        ));
    }
    host_port_collisions(&mut errors, config);
    errors
}

fn settings(errors: &mut Vec<FieldError>, path: &str, s: &MachineSettings) {
    let mut err = |field: String, msg: &str| errors.push(FieldError::new(field, msg));
    if let Some(b) = &s.box_name
        && (b.is_empty() || b.len() > 300 || b.chars().any(|c| c.is_whitespace() || c.is_control()))
    {
        err(
            format!("{path}.box"),
            "Enter a box name such as bento/ubuntu-24.04 (no spaces).",
        );
    }
    if let Some(h) = &s.hostname
        && !valid_hostname(h)
    {
        err(
            format!("{path}.hostname"),
            "Use letters, digits and '-', with dots between labels.",
        );
    }
    for (i, n) in s.private_networks.iter().enumerate() {
        if let Some(ip) = &n.ip
            && ip.parse::<IpAddr>().is_err()
        {
            err(
                format!("{path}.privateNetworks.{i}.ip"),
                "Enter an IP address such as 192.168.56.10.",
            );
        }
    }
    for (i, n) in s.public_networks.iter().enumerate() {
        let at = format!("{path}.publicNetworks.{i}");
        if let Some(ip) = &n.ip
            && ip.parse::<IpAddr>().is_err()
        {
            err(
                format!("{at}.ip"),
                "Enter an IP address on your network, such as 192.168.1.50.",
            );
        }
        if let Some(bridge) = &n.bridge
            && !plain_text(bridge)
        {
            err(
                format!("{at}.bridge"),
                "Enter the host network adapter's name.",
            );
        }
    }
    for (i, p) in s.forwarded_ports.iter().enumerate() {
        let at = format!("{path}.forwardedPorts.{i}");
        if p.guest == 0 {
            err(format!("{at}.guest"), "Ports are 1–65535.");
        }
        if p.host == 0 {
            err(format!("{at}.host"), "Ports are 1–65535.");
        }
        if let Some(ip) = &p.host_ip
            && ip.parse::<IpAddr>().is_err()
        {
            err(
                format!("{at}.hostIp"),
                "Enter an IP address such as 127.0.0.1.",
            );
        }
    }
    let mut guests = HashMap::new();
    for (i, f) in s.synced_folders.iter().enumerate() {
        let at = format!("{path}.syncedFolders.{i}");
        if !plain_text(&f.host_path) {
            err(format!("{at}.hostPath"), "Enter the host folder.");
        }
        if !plain_text(&f.guest_path) {
            err(
                format!("{at}.guestPath"),
                "Enter the folder inside the machine.",
            );
        } else if guests.insert(f.guest_path.clone(), i).is_some() {
            err(
                format!("{at}.guestPath"),
                "Another synced folder already uses this guest folder.",
            );
        }
    }
    for (i, p) in s.providers.iter().enumerate() {
        let at = format!("{path}.providers.{i}");
        if s.providers[..i].iter().any(|q| q.provider == p.provider) {
            err(
                format!("{at}.provider"),
                "This provider already has a settings block.",
            );
        }
        if p.cpus.is_some_and(|c| c == 0 || c > MAX_CPUS) {
            err(format!("{at}.cpus"), "Use 1–256 CPUs.");
        }
        if p.memory_mb
            .is_some_and(|m| !(MIN_MEMORY_MB..=MAX_MEMORY_MB).contains(&m))
        {
            err(format!("{at}.memoryMb"), "Use 64–1048576 MB.");
        }
    }
    for (i, p) in s.provisioners.iter().enumerate() {
        let at = format!("{path}.provisioners.{i}.source");
        match &p.source {
            ShellSource::Inline { script } => {
                if script.trim().is_empty() {
                    err(format!("{at}.script"), "Enter the script to run.");
                } else if script
                    .chars()
                    .any(|c| c.is_control() && c != '\n' && c != '\t')
                {
                    err(
                        format!("{at}.script"),
                        "The script contains control characters (such as carriage returns) that can't be saved.",
                    );
                }
            }
            ShellSource::Path { path: p } => {
                if !plain_text(p) {
                    err(format!("{at}.path"), "Enter the script file path.");
                }
            }
        }
    }
}

/// Vagrant refuses to start a machine whose host port is already forwarded
/// by another machine of the same environment; settings on `config` apply
/// to every machine.
fn host_port_collisions(errors: &mut Vec<FieldError>, config: &VagrantConfig) {
    let key = |p: &ForwardedPort| {
        (
            p.host_ip.clone().unwrap_or_default(),
            p.host,
            p.protocol.unwrap_or(Protocol::Tcp),
        )
    };
    let mut owners = HashSet::new();
    for (i, p) in config.defaults.forwarded_ports.iter().enumerate() {
        let path = format!("defaults.forwardedPorts.{i}.host");
        if !owners.insert(key(p)) {
            errors.push(FieldError::new(
                path,
                "This host port is already forwarded.",
            ));
        } else if config.machines.len() > 1 {
            errors.push(FieldError::new(
                path,
                "Every machine would forward this host port, so only one could start. Forward it from one machine instead.",
            ));
        }
    }
    for (m, machine) in config.machines.iter().enumerate() {
        for (i, p) in machine.settings.forwarded_ports.iter().enumerate() {
            let path = format!("machines.{m}.settings.forwardedPorts.{i}.host");
            if !owners.insert(key(p)) {
                errors.push(FieldError::new(
                    path,
                    "This host port is already forwarded by another entry.",
                ));
            }
        }
    }
}

pub(crate) fn valid_machine_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn valid_hostname(h: &str) -> bool {
    !h.is_empty()
        && h.len() <= 253
        && h.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

fn plain_text(s: &str) -> bool {
    !s.trim().is_empty() && !s.chars().any(char::is_control)
}
