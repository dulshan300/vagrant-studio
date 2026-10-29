//! Built-in Vagrantfile templates, rendered for a chosen provider.
//!
//! Template files (`templates/*/Vagrantfile.tmpl`) are Ruby with two kinds of
//! placeholder: `{{provider_name}}`, and a line holding only
//! `{{provider target=<var> cpus=<n> memory=<mb>}}`, which is replaced by that
//! provider's configuration block at the same indentation.

use serde::Serialize;

/// Providers the templates can be rendered for (`id`, display name).
pub const TEMPLATE_PROVIDERS: &[(&str, &str)] = &[
    ("virtualbox", "VirtualBox"),
    ("vmware_desktop", "VMware Desktop"),
];

/// A built-in starting point for a new project's Vagrantfile.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub machines: &'static [&'static str],
    /// Provider ids this template can be rendered for.
    pub providers: Vec<&'static str>,
    #[serde(skip)]
    pub source: &'static str,
}

/// Rendered templates are checked with `vagrant validate` for every provider
/// (see docs/implementation-status.md).
pub fn all() -> Vec<Template> {
    let providers = || TEMPLATE_PROVIDERS.iter().map(|(id, _)| *id).collect();
    vec![
        Template {
            id: "single-linux-vm",
            name: "Single Linux VM",
            description: "One Ubuntu 24.04 machine with common build tools.",
            machines: &["default"],
            providers: providers(),
            source: include_str!("../../../templates/single-linux-vm/Vagrantfile.tmpl"),
        },
        Template {
            id: "web-database",
            name: "Web + database",
            description: "An nginx web server and a PostgreSQL server on a private network.",
            machines: &["web", "db"],
            providers: providers(),
            source: include_str!("../../../templates/web-database/Vagrantfile.tmpl"),
        },
        Template {
            id: "multi-tier-app",
            name: "Multi-tier application",
            description: "Reverse proxy, application server and database on a private network.",
            machines: &["proxy", "app", "db"],
            providers: providers(),
            source: include_str!("../../../templates/multi-tier-app/Vagrantfile.tmpl"),
        },
    ]
}

pub fn find(id: &str) -> Option<Template> {
    all().into_iter().find(|t| t.id == id)
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    #[error("the {0} provider isn't supported by this template")]
    UnsupportedProvider(String),
    #[error("template line {line} has an invalid placeholder: {text}")]
    BadPlaceholder { line: usize, text: String },
}

impl Template {
    pub fn render(&self, provider: &str) -> Result<String, RenderError> {
        let provider_name = TEMPLATE_PROVIDERS
            .iter()
            .find(|(id, _)| *id == provider && self.providers.contains(id))
            .map(|(_, name)| *name)
            .ok_or_else(|| RenderError::UnsupportedProvider(provider.to_owned()))?;

        let mut out = String::with_capacity(self.source.len() + 256);
        for (i, line) in self.source.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(inner) = trimmed
                .strip_prefix("{{provider ")
                .and_then(|s| s.strip_suffix("}}"))
            {
                let indent = &line[..line.len() - line.trim_start().len()];
                let bad = || RenderError::BadPlaceholder {
                    line: i + 1,
                    text: trimmed.to_owned(),
                };
                let block = ProviderBlock::parse(inner).ok_or_else(bad)?;
                block.write(&mut out, indent, provider);
            } else {
                let line = line.replace("{{provider_name}}", provider_name);
                if line.contains("{{") {
                    return Err(RenderError::BadPlaceholder {
                        line: i + 1,
                        text: trimmed.to_owned(),
                    });
                }
                out.push_str(&line);
                out.push('\n');
            }
        }
        Ok(out)
    }
}

struct ProviderBlock<'a> {
    target: &'a str,
    cpus: u32,
    memory_mb: u32,
}

impl<'a> ProviderBlock<'a> {
    fn parse(spec: &'a str) -> Option<Self> {
        let (mut target, mut cpus, mut memory_mb) = (None, None, None);
        for pair in spec.split_whitespace() {
            match pair.split_once('=')? {
                ("target", v) if v.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') => {
                    target = Some(v)
                }
                ("cpus", v) => cpus = v.parse().ok(),
                ("memory", v) => memory_mb = v.parse().ok(),
                _ => return None,
            }
        }
        Some(Self {
            target: target?,
            cpus: cpus?,
            memory_mb: memory_mb?,
        })
    }

    fn write(&self, out: &mut String, indent: &str, provider: &str) {
        let Self {
            target,
            cpus,
            memory_mb,
        } = self;
        let lines: Vec<String> = match provider {
            "vmware_desktop" => vec![
                format!("{target}.vm.provider \"vmware_desktop\" do |v|"),
                format!("  v.vmx[\"numvcpus\"] = \"{cpus}\""),
                format!("  v.vmx[\"memsize\"] = \"{memory_mb}\""),
                "end".to_owned(),
            ],
            _ => vec![
                format!("{target}.vm.provider \"virtualbox\" do |vb|"),
                format!("  vb.cpus = {cpus}"),
                format!("  vb.memory = {memory_mb}"),
                "end".to_owned(),
            ],
        };
        for line in lines {
            out.push_str(indent);
            out.push_str(&line);
            out.push('\n');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_renders_for_every_provider() {
        for t in all() {
            for (provider, name) in TEMPLATE_PROVIDERS {
                let out = t.render(provider).unwrap();
                assert!(!out.contains("{{"), "{} {provider}: placeholder left", t.id);
                assert!(out.contains("Generated by Vagrant Studio"));
                assert!(out.contains(&format!("for the {name} provider")));
                assert!(out.contains(&format!(".vm.provider \"{provider}\"")));
                let other = if *provider == "virtualbox" {
                    "vmware_desktop"
                } else {
                    "virtualbox"
                };
                assert!(
                    !out.contains(&format!("\"{other}\"")),
                    "{} mixes providers",
                    t.id
                );
                if t.machines.len() > 1 {
                    for m in t.machines {
                        assert!(out.contains(&format!("config.vm.define \"{m}\"")));
                    }
                }
            }
        }
    }

    /// Template projects open in the visual editor, and saving them without
    /// changes regenerates the same model.
    #[test]
    fn every_template_is_editable_visually() {
        use vagrant_studio_config_model::{LineEnding, ProviderKind, analyze, generate};
        for t in all() {
            for (provider, _) in TEMPLATE_PROVIDERS {
                let a = analyze(&t.render(provider).unwrap());
                let model = a
                    .model
                    .unwrap_or_else(|| panic!("{} {provider}: {:?}", t.id, a.unsupported));
                assert!(a.errors.is_empty(), "{} {provider}: {:?}", t.id, a.errors);
                assert!(
                    a.warnings.is_empty(),
                    "{} {provider}: {:?}",
                    t.id,
                    a.warnings
                );
                assert!(!a.managed);
                let names: Vec<&str> = model.machines.iter().map(|m| m.name.as_str()).collect();
                if t.machines == ["default"] {
                    assert!(names.is_empty());
                    assert_eq!(model.defaults.providers.len(), 1);
                } else {
                    assert_eq!(names, t.machines);
                    for m in &model.machines {
                        assert_eq!(
                            m.settings.providers[0].provider,
                            ProviderKind::parse(provider).unwrap()
                        );
                    }
                }
                let regenerated = generate(&model, LineEnding::Lf).unwrap();
                assert_eq!(analyze(&regenerated).model, Some(model));
            }
        }
    }

    #[test]
    fn provider_blocks_keep_indentation_and_values() {
        let t = find("web-database").unwrap();
        let vmware = t.render("vmware_desktop").unwrap();
        assert!(vmware.contains(
            "    db.vm.provider \"vmware_desktop\" do |v|\n      v.vmx[\"numvcpus\"] = \"1\"\n      v.vmx[\"memsize\"] = \"1024\"\n    end\n"
        ));
        let vbox = find("single-linux-vm")
            .unwrap()
            .render("virtualbox")
            .unwrap();
        assert!(vbox.contains(
            "  config.vm.provider \"virtualbox\" do |vb|\n    vb.cpus = 2\n    vb.memory = 2048\n  end\n"
        ));
    }

    /// Writes every template × provider render, the visual editor's version
    /// of each, and a sample using every visual-editor feature, for checking
    /// with `vagrant validate` (see docs/implementation-status.md):
    /// `VAGRANT_STUDIO_RENDER_DIR=<dir> cargo test -p vagrant-studio-project-service -- --ignored render_all`
    #[test]
    #[ignore = "writes files for manual validation with Vagrant"]
    fn render_all_for_validation() {
        use vagrant_studio_config_model::{LineEnding, analyze, generate};
        let dir = std::path::PathBuf::from(
            std::env::var("VAGRANT_STUDIO_RENDER_DIR").expect("VAGRANT_STUDIO_RENDER_DIR"),
        );
        let write = |name: String, content: String| {
            let out = dir.join(name);
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(out.join("Vagrantfile"), content).unwrap();
            out
        };
        for t in all() {
            for (provider, _) in TEMPLATE_PROVIDERS {
                let rendered = t.render(provider).unwrap();
                let model = analyze(&rendered).model.unwrap();
                write(format!("{}--{provider}", t.id), rendered);
                write(
                    format!("{}--{provider}--visual", t.id),
                    generate(&model, LineEnding::Lf).unwrap(),
                );
            }
        }
        for (provider, _) in TEMPLATE_PROVIDERS {
            let out = write(
                format!("visual-features--{provider}"),
                generate(&feature_sample(provider), LineEnding::Crlf).unwrap(),
            );
            std::fs::create_dir_all(out.join("scripts")).unwrap();
            std::fs::write(out.join("scripts/setup.sh"), "#!/bin/sh\necho setup\n").unwrap();
        }
    }

    /// Every setting the visual editor supports, in one multi-machine file.
    fn feature_sample(provider: &str) -> vagrant_studio_config_model::VagrantConfig {
        use vagrant_studio_config_model::*;
        let kind = ProviderKind::parse(provider).unwrap();
        VagrantConfig {
            defaults: MachineSettings {
                box_name: Some("bento/ubuntu-24.04".into()),
                synced_folders: vec![SyncedFolder {
                    host_path: ".".into(),
                    guest_path: "/vagrant".into(),
                    disabled: true,
                }],
                provisioners: vec![ShellProvisioner {
                    name: None,
                    source: ShellSource::Inline {
                        script: "echo \"shared #{1}\" \\\n  && true\n".into(),
                    },
                    run: Some(ProvisionRun::Always),
                    privileged: Some(false),
                }],
                ..Default::default()
            },
            machines: vec![
                MachineDefinition {
                    name: "web-1".into(),
                    primary: true,
                    autostart: true,
                    settings: MachineSettings {
                        hostname: Some("web-1.local".into()),
                        private_networks: vec![PrivateNetwork {
                            ip: Some("192.168.56.30".into()),
                        }],
                        public_networks: vec![PublicNetwork {
                            ip: Some("192.168.1.230".into()),
                            bridge: Some("Intel(R) Wi-Fi 6 AX200 160MHz".into()),
                        }],
                        forwarded_ports: vec![
                            ForwardedPort {
                                guest: 80,
                                host: 18080,
                                host_ip: Some("127.0.0.1".into()),
                                protocol: None,
                            },
                            ForwardedPort {
                                guest: 53,
                                host: 15353,
                                host_ip: None,
                                protocol: Some(Protocol::Udp),
                            },
                        ],
                        synced_folders: vec![SyncedFolder {
                            host_path: "scripts".into(),
                            guest_path: "/opt/scripts".into(),
                            disabled: false,
                        }],
                        providers: vec![ProviderSettings {
                            provider: kind,
                            cpus: Some(1),
                            memory_mb: Some(1024),
                        }],
                        provisioners: vec![ShellProvisioner {
                            name: None,
                            source: ShellSource::Inline {
                                script: "apt-get update -y\n  echo indented\n".into(),
                            },
                            run: None,
                            privileged: None,
                        }],
                        ..Default::default()
                    },
                },
                MachineDefinition {
                    name: "db".into(),
                    primary: false,
                    autostart: false,
                    settings: MachineSettings {
                        box_name: Some("bento/debian-12".into()),
                        private_networks: vec![PrivateNetwork { ip: None }],
                        public_networks: vec![PublicNetwork {
                            ip: None,
                            bridge: None,
                        }],
                        providers: vec![ProviderSettings {
                            provider: kind,
                            cpus: None,
                            memory_mb: Some(512),
                        }],
                        provisioners: vec![
                            ShellProvisioner {
                                name: None,
                                source: ShellSource::Path {
                                    path: "scripts/setup.sh".into(),
                                },
                                run: Some(ProvisionRun::Once),
                                privileged: Some(true),
                            },
                            ShellProvisioner {
                                name: Some("seed data".into()),
                                source: ShellSource::Inline {
                                    script: "echo seed\n".into(),
                                },
                                run: Some(ProvisionRun::Never),
                                privileged: None,
                            },
                        ],
                        ..Default::default()
                    },
                },
            ],
        }
    }

    #[test]
    fn rejects_unknown_providers_and_bad_placeholders() {
        let t = find("single-linux-vm").unwrap();
        assert_eq!(
            t.render("hyperv"),
            Err(RenderError::UnsupportedProvider("hyperv".into()))
        );
        let broken = Template {
            source: "  {{provider target=x cpus=two memory=1}}\n",
            ..t.clone()
        };
        assert!(matches!(
            broken.render("virtualbox"),
            Err(RenderError::BadPlaceholder { line: 1, .. })
        ));
        let stray = Template {
            source: "x = {{unknown}}\n",
            ..t
        };
        assert!(stray.render("virtualbox").is_err());
    }
}
