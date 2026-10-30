use super::*;

const MULTI: &str = r#"# -*- mode: ruby -*-
# vi: set ft=ruby :

Vagrant.configure("2") do |config|
  config.vm.box = "bento/ubuntu-24.04"
  config.vm.synced_folder ".", "/vagrant", disabled: true

  config.vm.define "web", primary: true do |web|
    web.vm.hostname = "web"
    web.vm.network "private_network", ip: "192.168.56.10"
    web.vm.network "public_network", ip: "192.168.1.60", bridge: "Intel(R) Wi-Fi 6 AX200 160MHz"
    web.vm.network "forwarded_port", guest: 80, host: 8080, host_ip: "127.0.0.1"
    web.vm.provider "virtualbox" do |vb|
      vb.cpus = 1
      vb.memory = "1024"
    end
    web.vm.provider :vmware_desktop do |v|
      v.vmx["numvcpus"] = "2"
      v.vmx["memsize"] = "2048"
    end
    web.vm.provision "shell", inline: <<-SHELL
      apt-get update -y
    SHELL
  end

  config.vm.define "db", autostart: false do |db|
    db.vm.box = "bento/debian-12"
    db.vm.network "private_network", type: "dhcp"
    db.vm.network "public_network"
    db.vm.network "forwarded_port",
      guest: 53, host: 5353, protocol: "udp"
    db.vm.provision "shell", path: "scripts/db.sh", run: "always", privileged: false
    db.vm.provision "seed data", type: :shell, inline: "echo seed", run: "never"
  end
end
"#;

fn multi_model() -> VagrantConfig {
    VagrantConfig {
        defaults: MachineSettings {
            box_name: Some("bento/ubuntu-24.04".into()),
            synced_folders: vec![SyncedFolder {
                host_path: ".".into(),
                guest_path: "/vagrant".into(),
                disabled: true,
            }],
            ..Default::default()
        },
        machines: vec![
            MachineDefinition {
                name: "web".into(),
                primary: true,
                autostart: true,
                settings: MachineSettings {
                    hostname: Some("web".into()),
                    private_networks: vec![PrivateNetwork {
                        ip: Some("192.168.56.10".into()),
                    }],
                    public_networks: vec![PublicNetwork {
                        ip: Some("192.168.1.60".into()),
                        bridge: Some("Intel(R) Wi-Fi 6 AX200 160MHz".into()),
                    }],
                    forwarded_ports: vec![ForwardedPort {
                        guest: 80,
                        host: 8080,
                        host_ip: Some("127.0.0.1".into()),
                        protocol: None,
                    }],
                    providers: vec![
                        ProviderSettings {
                            provider: ProviderKind::Virtualbox,
                            cpus: Some(1),
                            memory_mb: Some(1024),
                        },
                        ProviderSettings {
                            provider: ProviderKind::VmwareDesktop,
                            cpus: Some(2),
                            memory_mb: Some(2048),
                        },
                    ],
                    provisioners: vec![ShellProvisioner {
                        name: None,
                        source: ShellSource::Inline {
                            script: "      apt-get update -y\n".into(),
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
                    forwarded_ports: vec![ForwardedPort {
                        guest: 53,
                        host: 5353,
                        host_ip: None,
                        protocol: Some(Protocol::Udp),
                    }],
                    provisioners: vec![
                        ShellProvisioner {
                            name: None,
                            source: ShellSource::Path {
                                path: "scripts/db.sh".into(),
                            },
                            run: Some(ProvisionRun::Always),
                            privileged: Some(false),
                        },
                        ShellProvisioner {
                            name: Some("seed data".into()),
                            source: ShellSource::Inline {
                                script: "echo seed".into(),
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
fn recognizes_a_multi_machine_file() {
    let a = analyze(MULTI);
    assert_eq!(a.unsupported, None);
    assert_eq!(a.model, Some(multi_model()));
    assert!(!a.managed);
    assert!(a.warnings.is_empty(), "{:?}", a.warnings);
    assert!(a.errors.is_empty(), "{:?}", a.errors);
    assert_eq!(a.line_ending, LineEnding::Lf);
}

#[test]
fn generated_files_read_back_identically() {
    let model = multi_model();
    let text = generate(&model, LineEnding::Lf).unwrap();
    let a = analyze(&text);
    assert_eq!(a.model, Some(model));
    assert!(a.managed && !a.modified_outside);
    assert!(a.warnings.is_empty(), "{:?}", a.warnings);
    assert!(text.starts_with("# -*- mode: ruby -*-\n"));
    assert!(text.contains("  config.vm.define \"web\", primary: true do |web|\n"));
    assert!(text.contains("  config.vm.define \"db\", autostart: false do |db|\n"));
    assert!(text.contains("      v.vmx[\"memsize\"] = \"2048\"\n"));
    assert!(text.ends_with("  end\nend\n"));
}

#[test]
fn single_machine_layout() {
    let model = VagrantConfig {
        defaults: MachineSettings {
            box_name: Some("bento/ubuntu-24.04".into()),
            hostname: Some("dev".into()),
            providers: vec![ProviderSettings {
                provider: ProviderKind::Virtualbox,
                cpus: Some(2),
                memory_mb: Some(2048),
            }],
            provisioners: vec![ShellProvisioner {
                name: None,
                source: ShellSource::Inline {
                    script: "apt-get update -y\napt-get install -y git\n".into(),
                },
                run: None,
                privileged: None,
            }],
            ..Default::default()
        },
        machines: vec![],
    };
    let text = generate(&model, LineEnding::Lf).unwrap();
    let body = text.split_once("\n\n").unwrap().1;
    assert_eq!(
        body,
        r#"Vagrant.configure("2") do |config|
  config.vm.box = "bento/ubuntu-24.04"
  config.vm.hostname = "dev"

  config.vm.provider "virtualbox" do |vb|
    vb.cpus = 2
    vb.memory = 2048
  end

  config.vm.provision "shell", inline: <<~SHELL
    apt-get update -y
    apt-get install -y git
  SHELL
end
"#
    );
}

#[test]
fn tricky_scripts_round_trip_exactly() {
    let scripts = [
        "echo plain\n",
        "  indented\n    more\n",
        "line one \\\n  continued\n",
        "echo \"#{not ruby}\" '#@x' #$y\n",
        "SHELL\nSCRIPT\necho both terminators\n",
        "\tstarts with a tab\n",
        "a\n   \nb\n",
        "a\n\n\nb\n",
        "no trailing newline",
        "",
        "ünïcödé ✓ \"quotes\" and 'single'\n",
        "trailing spaces   \n",
    ];
    for script in scripts {
        let model = VagrantConfig {
            defaults: MachineSettings {
                box_name: Some("b".into()),
                provisioners: vec![ShellProvisioner {
                    name: None,
                    source: ShellSource::Inline {
                        script: script.into(),
                    },
                    run: Some(ProvisionRun::Once),
                    privileged: Some(true),
                }],
                ..Default::default()
            },
            machines: vec![],
        };
        if script.trim().is_empty() {
            assert!(generate(&model, LineEnding::Lf).is_err());
            continue;
        }
        let text = generate(&model, LineEnding::Lf).unwrap_or_else(|e| panic!("{script:?}: {e:?}"));
        assert_eq!(analyze(&text).model, Some(model), "{script:?}\n{text}");
    }
}

#[test]
fn strings_with_special_characters_round_trip() {
    let model = VagrantConfig {
        defaults: MachineSettings {
            box_name: Some("https://example.com/boxes/x#{y}.box".into()),
            synced_folders: vec![SyncedFolder {
                host_path: r#"C:\Users\Dev Person\code "x""#.into(),
                guest_path: "/home/vagrant/código".into(),
                disabled: false,
            }],
            ..Default::default()
        },
        machines: vec![],
    };
    let text = generate(&model, LineEnding::Lf).unwrap();
    assert_eq!(analyze(&text).model, Some(model));
}

#[test]
fn crlf_files_keep_their_line_endings() {
    let crlf = MULTI.replace('\n', "\r\n");
    let a = analyze(&crlf);
    assert_eq!(a.model, Some(multi_model()));
    assert_eq!(a.line_ending, LineEnding::Crlf);

    let text = generate(&multi_model(), LineEnding::Crlf).unwrap();
    assert!(!text.replace("\r\n", "").contains('\n'));
    let back = analyze(&text);
    assert_eq!(back.model, Some(multi_model()));
    assert!(back.managed && !back.modified_outside);
    assert_eq!(back.line_ending, LineEnding::Crlf);
}

#[test]
fn edits_outside_the_editor_are_noticed() {
    let text = generate(&multi_model(), LineEnding::Lf).unwrap();
    let edited = text.replace("bento/debian-12", "bento/debian-13");
    let a = analyze(&edited);
    assert!(a.managed && a.modified_outside);
    assert_eq!(
        a.model.unwrap().machines[1].settings.box_name.as_deref(),
        Some("bento/debian-13")
    );

    let broken = text.replace(
        "  config.vm.define \"db\"",
        "  config.vm.box_check_update = false\n  config.vm.define \"db\"",
    );
    let a = analyze(&broken);
    assert!(a.managed && a.modified_outside && a.model.is_none());
    assert!(a.unsupported.unwrap().message.contains("box_check_update"));
}

#[test]
fn comments_are_reported_as_lost() {
    let text = MULTI
        .replace("  config.vm.box =", "  # pinned for CI\n  config.vm.box =")
        .replace("vb.cpus = 1", "vb.cpus = 1 # keep small");
    let a = analyze(&text);
    assert!(a.model.is_some());
    assert_eq!(
        a.warnings,
        vec!["Saving from the visual editor removes 2 comments (line 5, 15).".to_owned()]
    );
}

#[test]
fn unsupported_files_name_the_first_line_they_cannot_show() {
    let cases: &[(&str, usize, &str)] = &[
        (
            "require 'yaml'\nVagrant.configure(\"2\") do |config|\nend\n",
            1,
            "require",
        ),
        ("BOX = \"x\"\n", 1, "BOX"),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.vm.box_check_update = false\nend\n",
            2,
            "box_check_update",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.vm.box = \"a\"\n  if true\n  end\nend\n",
            3,
            "`if`",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.vm.provider \"virtualbox\" do |vb|\n    vb.gui = true\n  end\nend\n",
            3,
            "vb.gui",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.vm.network \"public_network\", bridge: [\"a\", \"b\"]\nend\n",
            2,
            "bridge:",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.vm.network \"public_network\", ip: \"10.0.0.5\", type: \"dhcp\"\nend\n",
            2,
            "both",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.vm.network \"public_network\", use_dhcp_assigned_default_route: true\nend\n",
            2,
            "use_dhcp_assigned_default_route",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.vm.provision \"ansible\", playbook: \"p.yml\"\nend\n",
            2,
            "ansible",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.vm.provider \"hyperv\" do |h|\n  end\nend\n",
            2,
            "hyperv",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  (1..3).each do |i|\n  end\nend\n",
            2,
            "",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.vm.define \"a\" do |a|\n    config.vm.box = \"x\"\n  end\nend\n",
            3,
            "inside a machine block",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.vm.hostname = \"h-#{1}\"\nend\n",
            2,
            "interpolation",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.vm.box = \"a\"\n  config.vm.box = \"b\"\nend\n",
            3,
            "more than once",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.vm.network \"forwarded_port\", guest: 80, host: 70000\nend\n",
            2,
            "valid port",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.vm.network \"forwarded_port\", guest: 80, host: 8080, auto_correct: true\nend\n",
            2,
            "auto_correct",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\n  config.ssh.forward_agent = true\nend\n",
            2,
            "config.ssh",
        ),
        ("Vagrant.configure(\"2\") do |config|\n", 2, "missing `end`"),
        (
            "Vagrant.configure(\"2\") do |config|\nend\nputs 1\n",
            3,
            "only comments",
        ),
        (
            "Vagrant.configure(\"2\") do |config|\r\nend\nx\r",
            3,
            "control characters",
        ),
    ];
    for (src, line, needle) in cases {
        let a = analyze(src);
        let u = a.unsupported.unwrap_or_else(|| panic!("accepted: {src}"));
        assert_eq!(u.line, *line, "{src}: {u:?}");
        assert!(u.message.contains(needle), "{src}: {u:?}");
        assert!(a.model.is_none());
    }
}

#[test]
fn accepts_common_spellings() {
    let src = "Vagrant.configure(2) do |c|\n  c.vm.box = 'x'\n  c.vm.provider 'vmware_desktop' do |v|\n    v.cpus = 2\n    v.memory = 512\n  end\nend\n";
    let m = analyze(src).model.unwrap();
    assert_eq!(
        m.defaults.providers,
        vec![ProviderSettings {
            provider: ProviderKind::VmwareDesktop,
            cpus: Some(2),
            memory_mb: Some(512)
        }]
    );
}

#[test]
fn public_network_dhcp_spellings_mean_the_same() {
    let explicit = "Vagrant.configure(\"2\") do |config|\n  config.vm.box = \"x\"\n  config.vm.network \"public_network\", type: \"dhcp\", bridge: \"eth0\"\nend\n";
    let implicit = "Vagrant.configure(\"2\") do |config|\n  config.vm.box = \"x\"\n  config.vm.network \"public_network\", bridge: \"eth0\"\nend\n";
    let a = analyze(explicit).model;
    assert!(a.is_some(), "{:?}", analyze(explicit).unsupported);
    assert_eq!(a, analyze(implicit).model);
    let generated = generate(&a.unwrap(), LineEnding::Lf).unwrap();
    assert!(generated.contains("  config.vm.network \"public_network\", bridge: \"eth0\"\n"));
}

#[test]
fn validation_finds_problems_and_blocks_generation() {
    let mut m = multi_model();
    m.defaults.box_name = None;
    m.machines[0].primary = true;
    m.machines[1].primary = true;
    m.machines[1].name = "WEB".into();
    m.machines[0].settings.private_networks[0].ip = Some("192.168.56".into());
    m.machines[0].settings.providers[0].cpus = Some(0);
    m.machines[0].settings.hostname = Some("bad_host".into());
    m.machines[1].settings.forwarded_ports[0].host = 8080;
    m.machines[1].settings.forwarded_ports[0].protocol = None;
    m.machines[0].settings.forwarded_ports[0].host_ip = None;
    let paths: Vec<String> = validate(&m).into_iter().map(|e| e.path).collect();
    assert_eq!(
        paths,
        vec![
            "machines.0.settings.hostname",
            "machines.0.settings.privateNetworks.0.ip",
            "machines.0.settings.providers.0.cpus",
            "machines.0.settings.box",
            "machines.1.name",
            "machines.1.primary",
            "machines.1.settings.forwardedPorts.0.host",
        ]
    );
    assert!(generate(&m, LineEnding::Lf).is_err());

    let mut shared = multi_model();
    shared.defaults.forwarded_ports.push(ForwardedPort {
        guest: 22,
        host: 2200,
        host_ip: None,
        protocol: None,
    });
    assert_eq!(validate(&shared)[0].path, "defaults.forwardedPorts.0.host");

    let empty = VagrantConfig::default();
    assert_eq!(validate(&empty)[0].path, "defaults.box");
}

#[test]
fn model_json_uses_camel_case() {
    let json = serde_json::to_value(multi_model()).unwrap();
    assert_eq!(json["defaults"]["box"], "bento/ubuntu-24.04");
    let web = &json["machines"][0]["settings"];
    assert_eq!(web["forwardedPorts"][0]["hostIp"], "127.0.0.1");
    assert_eq!(web["providers"][1]["provider"], "vmware_desktop");
    assert_eq!(web["providers"][1]["memoryMb"], 2048);
    assert_eq!(web["provisioners"][0]["source"]["kind"], "inline");
    let back: VagrantConfig = serde_json::from_value(json).unwrap();
    assert_eq!(back, multi_model());
}

#[test]
fn named_provisioners_generate_with_type() {
    let model = multi_model();
    let text = generate(&model, LineEnding::Lf).unwrap();
    assert!(
        text.contains("    db.vm.provision \"seed data\", type: \"shell\", inline: \"echo seed\", run: \"never\"\n"),
        "{text}"
    );
    assert_eq!(analyze(&text).model, Some(model));

    let typed_string = "Vagrant.configure(\"2\") do |config|\n  config.vm.box = \"x\"\n  config.vm.provision \"a\", type: \"shell\", inline: \"echo\"\nend\n";
    let typed_symbol = typed_string.replace("type: \"shell\"", "type: :shell");
    let a = analyze(typed_string).model.unwrap();
    assert_eq!(a.defaults.provisioners[0].name.as_deref(), Some("a"));
    assert_eq!(analyze(&typed_symbol).model, Some(a));

    let other = typed_string.replace("type: \"shell\"", "type: \"file\"");
    let u = analyze(&other).unsupported.unwrap();
    assert!(u.message.contains("`file` provisioners"), "{u:?}");
}

#[test]
fn provisioner_names_must_be_selectable_and_distinct() {
    let named = |name: &str| ShellProvisioner {
        name: Some(name.into()),
        source: ShellSource::Inline {
            script: "echo\n".into(),
        },
        run: None,
        privileged: None,
    };
    let mut m = multi_model();
    m.defaults.provisioners = vec![named("base"), named("a,b"), named("-x"), named(" pad")];
    m.machines[0].settings.provisioners = vec![named("app"), named("app"), named("base")];
    let paths: Vec<String> = validate(&m).into_iter().map(|e| e.path).collect();
    assert_eq!(
        paths,
        vec![
            "defaults.provisioners.1.name",
            "defaults.provisioners.2.name",
            "defaults.provisioners.3.name",
            "machines.0.settings.provisioners.1.name",
            "machines.0.settings.provisioners.2.name",
        ]
    );
    for ok in ["setup", "seed data", "ünï.cödé_2"] {
        assert!(valid_provisioner_name(ok), "{ok}");
    }
}

#[test]
fn machines_run_shared_provisioners_first() {
    let mut m = multi_model();
    m.defaults.provisioners.push(ShellProvisioner {
        name: Some("base".into()),
        source: ShellSource::Inline {
            script: "echo\n".into(),
        },
        run: None,
        privileged: None,
    });
    let by_machine = m.provisioners_by_machine();
    let db: Vec<_> = by_machine[1]
        .provisioners
        .iter()
        .map(|p| (p.provisioner.name.as_deref(), p.shared))
        .collect();
    assert_eq!(by_machine[1].machine, "db");
    assert_eq!(
        db,
        [
            (Some("base"), true),
            (None, false),
            (Some("seed data"), false)
        ]
    );
    let json = serde_json::to_value(&by_machine[1].provisioners[2]).unwrap();
    assert_eq!(json["name"], "seed data");
    assert_eq!(json["run"], "never");
    assert_eq!(json["shared"], false);

    m.machines.clear();
    let single = m.provisioners_by_machine();
    assert_eq!(single[0].machine, "default");
    assert_eq!(single[0].provisioners.len(), 1);
}

#[test]
fn machines_get_shared_forwarded_ports_too() {
    let mut m = multi_model();
    m.defaults.forwarded_ports.push(ForwardedPort {
        guest: 9000,
        host: 9000,
        host_ip: None,
        protocol: None,
    });
    let by_machine = m.forwarded_ports_by_machine();
    assert_eq!(by_machine.len(), m.machines.len());
    for (name, ports) in &by_machine {
        assert_eq!(ports[0].host, 9000, "{name}");
    }
    let single = VagrantConfig {
        defaults: m.defaults.clone(),
        machines: vec![],
    };
    let only = single.forwarded_ports_by_machine();
    assert_eq!((only[0].0.as_str(), only[0].1.len()), ("default", 1));
}
