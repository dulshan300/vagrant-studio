//! Machine lifecycle through the real engine, Vagrant and its docker
//! provider. Ignored by default; run on Linux with Docker available:
//! `cargo test -p vagrant-studio-operation-engine --test docker_lifecycle -- --ignored`.
//! Containers are created from a temporary folder and destroyed at the end.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use vagrant_studio_domain::{
    EnvironmentId, EnvironmentType, NewEnvironment, NewProject, Operation, OperationStatus,
    OperationType,
};
use vagrant_studio_operation_engine::lifecycle::{
    confirmation_phrase, operation_type, order_targets,
};
use vagrant_studio_operation_engine::{
    ConfirmationProof, EventSink, OperationEngine, OperationEvent, OperationRequest, logs_dir,
};
use vagrant_studio_platform::SystemPlatform;
use vagrant_studio_project_store::Store;
use vagrant_studio_vagrant_adapter::{
    Invocation, LifecycleAction, ProcessRunner, Runner, VagrantCommand, locate,
};

/// Three containers; `gamma` is not started by a bare `vagrant up`.
const VAGRANTFILE: &str = r#"
Vagrant.configure("2") do |config|
  config.vm.synced_folder ".", "/vagrant", disabled: true
  [["alpha", true], ["beta", true], ["gamma", false]].each do |name, autostart|
    config.vm.define name, autostart: autostart do |m|
      m.vm.provider "docker" do |d|
        d.image = "busybox:1.36"
        d.cmd = ["sleep", "infinity"]
        d.remains_running = true
      end
    end
  end
end
"#;

struct NoEvents;
impl EventSink for NoEvents {
    fn emit(&self, _: OperationEvent) {}
}

#[derive(Clone)]
struct Env {
    engine: OperationEngine,
    id: EnvironmentId,
    vagrant: PathBuf,
    dir: PathBuf,
}

impl Env {
    fn request(&self, operation_type: OperationType, command: VagrantCommand) -> OperationRequest {
        OperationRequest {
            operation_type,
            command,
            program: self.vagrant.clone(),
            working_dir: self.dir.clone(),
            project_id: None,
            environment_id: Some(self.id),
            machine_name: None,
            target_label: "docker lifecycle test".into(),
            env: docker_env(),
            timeout: Some(Duration::from_secs(600)),
            confirmation: None,
        }
    }

    async fn states(&self) -> Vec<(String, String)> {
        let status = self
            .engine
            .environment_status(self.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status.error, None);
        status
            .machines
            .into_iter()
            .map(|m| (m.name, m.state.unwrap_or_default()))
            .collect()
    }

    /// Runs a lifecycle command the way the app's `run_lifecycle` does.
    async fn lifecycle(&self, action: LifecycleAction, machines: &[&str]) -> Operation {
        let status = self.engine.environment_status(self.id).await.unwrap();
        let requested: Vec<String> = machines.iter().map(|m| (*m).to_owned()).collect();
        let targets = order_targets(status.as_ref(), &requested).unwrap();
        let command = VagrantCommand::Lifecycle {
            action,
            machines: targets.clone(),
        };
        let mut req = self.request(operation_type(action), command.clone());
        if action.is_destructive() {
            let phrase = confirmation_phrase("test", &targets);
            let c = self.engine.confirm(Some(self.id), command, phrase.clone());
            req.confirmation = Some(ConfirmationProof {
                token: c.token,
                typed: phrase,
            });
        }
        self.engine.run(req).await.unwrap()
    }
}

fn docker_env() -> Vec<(String, String)> {
    vec![("VAGRANT_DEFAULT_PROVIDER".into(), "docker".into())]
}

fn state(states: &[(String, String)], name: &str) -> String {
    states.iter().find(|(n, _)| n == name).unwrap().1.clone()
}

async fn steps(env: Env) {
    let op = env
        .engine
        .run(env.request(
            OperationType::Status,
            VagrantCommand::Status { machine: None },
        ))
        .await
        .unwrap();
    assert_eq!(op.status, OperationStatus::Succeeded, "{op:?}");
    let states = env.states().await;
    assert_eq!(states.len(), 3);
    assert!(states.iter().all(|(_, s)| s == "not_created"), "{states:?}");

    // Naming gamma starts it although it has autostart: false.
    let op = env
        .lifecycle(LifecycleAction::Up, &["gamma", "alpha"])
        .await;
    assert_eq!(op.status, OperationStatus::Succeeded, "{op:?}");
    assert_eq!(op.command_line, "vagrant up --no-tty alpha gamma");
    assert_eq!(op.error_summary, None, "no state warning");
    let states = env.states().await;
    assert_eq!(state(&states, "alpha"), "running");
    assert_eq!(state(&states, "gamma"), "running");
    assert_eq!(state(&states, "beta"), "not_created");

    let op = env.lifecycle(LifecycleAction::Halt, &["alpha"]).await;
    assert_eq!(op.status, OperationStatus::Succeeded, "{op:?}");
    let states = env.states().await;
    assert_eq!(state(&states, "alpha"), "stopped");
    assert_eq!(state(&states, "gamma"), "running");

    let op = env.lifecycle(LifecycleAction::Reload, &["gamma"]).await;
    assert_eq!(op.status, OperationStatus::Succeeded, "{op:?}");
    assert_eq!(state(&env.states().await, "gamma"), "running");

    // The docker provider can't suspend: Vagrant fails and the state is re-read.
    let op = env.lifecycle(LifecycleAction::Suspend, &["gamma"]).await;
    assert_eq!(op.status, OperationStatus::Failed, "{op:?}");
    assert!(
        op.error_summary
            .as_deref()
            .unwrap()
            .contains("not supported with the Docker provider")
    );
    assert_eq!(state(&env.states().await, "gamma"), "running");

    let op = env
        .lifecycle(LifecycleAction::Destroy, &["alpha", "beta", "gamma"])
        .await;
    assert_eq!(op.status, OperationStatus::Succeeded, "{op:?}");
    let states = env.states().await;
    assert!(states.iter().all(|(_, s)| s == "not_created"), "{states:?}");
}

#[tokio::test]
#[ignore = "needs Vagrant and Docker on a Linux host"]
async fn docker_multi_machine_lifecycle() {
    let vagrant = locate(None, &SystemPlatform).expect("Vagrant is installed and on PATH");
    let store = Store::open_in_memory().await.unwrap();
    let project = store
        .insert_project(&NewProject {
            name: "test".into(),
            description: String::new(),
            root_path: "/test".into(),
            repository_url: None,
            tags: vec![],
        })
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Vagrantfile"), VAGRANTFILE).unwrap();
    let id = store
        .insert_environment(
            project.id,
            &NewEnvironment {
                name: "test".into(),
                root_path: dir.path().display().to_string(),
                vagrantfile_path: Some("Vagrantfile".into()),
                default_provider: Some("docker".into()),
                environment_type: EnvironmentType::Development,
            },
        )
        .await
        .unwrap()
        .id;
    let env = Env {
        engine: OperationEngine::new(
            store,
            Arc::new(ProcessRunner),
            Arc::new(NoEvents),
            logs_dir(dir.path()),
        ),
        id,
        vagrant,
        dir: dir.path().to_path_buf(),
    };

    let outcome = tokio::spawn(steps(env.clone())).await;
    // Always remove the containers, even after a failed assertion.
    force_destroy(&env.vagrant, &env.dir).await;
    if let Err(e) = outcome {
        std::panic::resume_unwind(e.into_panic());
    }
}

/// `vagrant destroy --force` outside the engine (cleanup needs no confirmation).
async fn force_destroy(vagrant: &Path, dir: &Path) {
    let _ = ProcessRunner
        .run(
            Invocation {
                program: vagrant.to_path_buf(),
                args: VagrantCommand::Lifecycle {
                    action: LifecycleAction::Destroy,
                    machines: vec![],
                }
                .args(),
                working_dir: dir.to_path_buf(),
                timeout: Some(Duration::from_secs(300)),
                env: docker_env(),
            },
            Box::new(|_| {}),
            CancellationToken::new(),
        )
        .await;
}
