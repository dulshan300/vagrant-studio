//! The supported subset of a Vagrantfile, as plain data.
//!
//! Values are what Ruby would evaluate them to (an inline script is the
//! string Vagrant receives), so two files that mean the same thing produce
//! equal models even when they are formatted differently.

use serde::{Deserialize, Serialize};

/// A whole Vagrantfile: settings on `config` apply to every machine; with no
/// `config.vm.define` blocks, Vagrant runs one machine named `default`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VagrantConfig {
    pub defaults: MachineSettings,
    #[serde(default)]
    pub machines: Vec<MachineDefinition>,
}

/// A `config.vm.define "name" do |m| … end` block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineDefinition {
    pub name: String,
    #[serde(default)]
    pub primary: bool,
    /// `false` keeps a bare `vagrant up` from starting the machine.
    #[serde(default = "yes")]
    pub autostart: bool,
    #[serde(default)]
    pub settings: MachineSettings,
}

fn yes() -> bool {
    true
}

/// The provisioners Vagrant runs for one machine, in its order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineProvisioners {
    pub machine: String,
    pub provisioners: Vec<MachineProvisioner>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineProvisioner {
    #[serde(flatten)]
    pub provisioner: ShellProvisioner,
    /// Set on `config`, so every machine runs it.
    pub shared: bool,
}

impl VagrantConfig {
    /// Each machine's provisioners: shared ones first, then its own (Vagrant
    /// 2.4.9 runs them in this order). Without `vm.define` the one machine is
    /// `default`.
    /// Each machine's forwarded ports: shared ones first, then its own.
    /// Without `vm.define` the one machine is `default`.
    pub fn forwarded_ports_by_machine(&self) -> Vec<(String, Vec<ForwardedPort>)> {
        let ports = |own: &[ForwardedPort]| {
            self.defaults
                .forwarded_ports
                .iter()
                .chain(own)
                .cloned()
                .collect::<Vec<_>>()
        };
        if self.machines.is_empty() {
            return vec![("default".into(), ports(&[]))];
        }
        self.machines
            .iter()
            .map(|m| (m.name.clone(), ports(&m.settings.forwarded_ports)))
            .collect()
    }

    pub fn provisioners_by_machine(&self) -> Vec<MachineProvisioners> {
        let entries = |own: &[ShellProvisioner]| {
            let shared = self.defaults.provisioners.iter().map(|p| (p, true));
            shared
                .chain(own.iter().map(|p| (p, false)))
                .map(|(p, shared)| MachineProvisioner {
                    provisioner: p.clone(),
                    shared,
                })
                .collect()
        };
        if self.machines.is_empty() {
            return vec![MachineProvisioners {
                machine: "default".into(),
                provisioners: entries(&[]),
            }];
        }
        self.machines
            .iter()
            .map(|m| MachineProvisioners {
                machine: m.name.clone(),
                provisioners: entries(&m.settings.provisioners),
            })
            .collect()
    }
}

/// The `<var>.vm.*` settings of `config` or of one machine.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineSettings {
    #[serde(rename = "box", default)]
    pub box_name: Option<String>,
    #[serde(default)]
    pub hostname: Option<String>,
    #[serde(default)]
    pub private_networks: Vec<PrivateNetwork>,
    #[serde(default)]
    pub public_networks: Vec<PublicNetwork>,
    #[serde(default)]
    pub forwarded_ports: Vec<ForwardedPort>,
    #[serde(default)]
    pub synced_folders: Vec<SyncedFolder>,
    #[serde(default)]
    pub providers: Vec<ProviderSettings>,
    #[serde(default)]
    pub provisioners: Vec<ShellProvisioner>,
}

/// `vm.network "private_network"`: a static `ip`, or DHCP when `ip` is unset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivateNetwork {
    pub ip: Option<String>,
}

/// `vm.network "public_network"` (bridged): a static `ip`, or DHCP from the
/// host's network when unset. `bridge` names the host adapter; VirtualBox
/// asks interactively when it is unset and several adapters are up, which
/// fails without a terminal. VMware Desktop ignores it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicNetwork {
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub bridge: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardedPort {
    pub guest: u16,
    pub host: u16,
    #[serde(default)]
    pub host_ip: Option<String>,
    #[serde(default)]
    pub protocol: Option<Protocol>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
}

impl Protocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncedFolder {
    pub host_path: String,
    pub guest_path: String,
    #[serde(default)]
    pub disabled: bool,
}

/// A `vm.provider` block limited to CPU and memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettings {
    pub provider: ProviderKind,
    #[serde(default)]
    pub cpus: Option<u32>,
    #[serde(default)]
    pub memory_mb: Option<u32>,
}

/// Providers whose CPU/memory settings the visual editor understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Virtualbox,
    VmwareDesktop,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Virtualbox => "virtualbox",
            Self::VmwareDesktop => "vmware_desktop",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "virtualbox" => Some(Self::Virtualbox),
            "vmware_desktop" => Some(Self::VmwareDesktop),
            _ => None,
        }
    }
}

/// `vm.provision "shell"` with an inline script or a script file, or
/// `vm.provision "name", type: "shell"` when it has a name. Only named
/// provisioners can be run on their own (`vagrant provision --provision-with`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellProvisioner {
    #[serde(default)]
    pub name: Option<String>,
    pub source: ShellSource,
    #[serde(default)]
    pub run: Option<ProvisionRun>,
    #[serde(default)]
    pub privileged: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ShellSource {
    Inline { script: String },
    Path { path: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProvisionRun {
    Once,
    Always,
    Never,
}

impl ProvisionRun {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Always => "always",
            Self::Never => "never",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "once" => Some(Self::Once),
            "always" => Some(Self::Always),
            "never" => Some(Self::Never),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineEnding {
    #[default]
    Lf,
    Crlf,
}
