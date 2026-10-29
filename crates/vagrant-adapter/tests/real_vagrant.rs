//! Integration tests against the Vagrant installed on this host. Ignored by
//! default; run with `cargo test -p vagrant-studio-vagrant-adapter -- --ignored`.
//! Set `VAGRANT_STUDIO_TEST_PROVIDER` (e.g. `vmware_desktop`) to choose the provider.
//! Only read-only commands are run, in a temporary copy of a template.

use tokio_util::sync::CancellationToken;
use vagrant_studio_platform::SystemPlatform;
use vagrant_studio_vagrant_adapter::machine_readable::{self, parse_status};
use vagrant_studio_vagrant_adapter::{
    Invocation, ProcessRunner, Runner, VagrantCommand, detect_version, locate,
};

/// Two machines and no provider block, so any installed provider can report them.
const MULTI_MACHINE: &str = r#"
Vagrant.configure("2") do |config|
  config.vm.box = "bento/ubuntu-24.04"
  config.vm.define "web"
  config.vm.define "db"
end
"#;

#[tokio::test]
#[ignore = "needs Vagrant installed on the host"]
async fn real_vagrant_version_and_multi_machine_status() {
    let vagrant = locate(None, &SystemPlatform).expect("Vagrant is installed and on PATH");
    let version = detect_version(&ProcessRunner, &vagrant).await.unwrap();
    eprintln!("Vagrant {version} at {}", vagrant.display());

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Vagrantfile"), MULTI_MACHINE).unwrap();

    let mut output = String::new();
    // `status` has no --provider flag; Vagrant's default-provider variable selects it.
    let provider = std::env::var("VAGRANT_STUDIO_TEST_PROVIDER").ok();
    let result = ProcessRunner
        .run(
            Invocation {
                program: vagrant,
                args: VagrantCommand::Status { machine: None }.args(),
                working_dir: dir.path().to_path_buf(),
                timeout: Some(std::time::Duration::from_secs(120)),
                env: provider
                    .iter()
                    .map(|p| ("VAGRANT_DEFAULT_PROVIDER".to_owned(), p.clone()))
                    .collect(),
            },
            Box::new(|line| {
                output.push_str(&line.text);
                output.push('\n');
            }),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    let lines = machine_readable::parse(&output);
    assert_eq!(machine_readable::error_exit(&lines), None, "{output}");
    assert!(result.success(), "{result:?}\n{output}");
    let machines = parse_status(&lines);
    let names: Vec<_> = machines.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["web", "db"]);
    for m in &machines {
        assert_eq!(m.state.as_deref(), Some("not_created"));
        if let Some(p) = &provider {
            assert_eq!(m.provider.as_deref(), Some(p.as_str()));
        }
        eprintln!("{} -> {:?} ({:?})", m.name, m.state, m.provider);
    }
}

/// Read-only: lists boxes, reads the machine index, checks catalogs for updates.
#[tokio::test]
#[ignore = "needs Vagrant installed on the host (and network for the update check)"]
async fn real_vagrant_box_store_queries() {
    use std::time::Duration;
    use vagrant_studio_vagrant_adapter::boxes;

    let vagrant = locate(None, &SystemPlatform).expect("Vagrant is installed and on PATH");
    let lines = vagrant_studio_vagrant_adapter::query(
        &ProcessRunner,
        &vagrant,
        &VagrantCommand::BoxList,
        Duration::from_secs(120),
    )
    .await
    .unwrap();
    let installed = machine_readable::parse_box_list(&lines);
    for b in &installed {
        assert!(b.provider.is_some() && b.version.is_some(), "{b:?}");
        eprintln!("box {b:?}");
    }

    let home = boxes::vagrant_home().expect("a home folder");
    let machines = boxes::read_machine_index(&home).unwrap();
    eprintln!("{} machines in {}", machines.len(), home.display());

    let outdated = boxes::parse_outdated(
        &vagrant_studio_vagrant_adapter::query(
            &ProcessRunner,
            &vagrant,
            &VagrantCommand::BoxOutdated,
            Duration::from_secs(600),
        )
        .await
        .unwrap(),
    );
    eprintln!("{outdated:?}");
    // One result per installed box version, whatever the catalogs say.
    assert_eq!(outdated.len(), installed.len());
}
