use std::sync::Mutex as StdMutex;

use vagrant_studio_domain::{EnvironmentType, NewEnvironment, NewProject};
use vagrant_studio_vagrant_adapter::runner::{LineSink, RunFuture};
use vagrant_studio_vagrant_adapter::{LifecycleAction, MachineName, OutputLine, Provisioning};

use super::*;

/// Replays scripted output; optionally waits for cancellation instead of exiting.
struct FakeRunner {
    lines: Vec<(Stream, String)>,
    exit_code: i32,
    hang_until_cancelled: bool,
    /// When set, `vagrant status` replays this (exit 0) instead of `lines`.
    status_output: Option<String>,
    /// When set, other commands wait for a permit before exiting.
    gate: Option<Arc<tokio::sync::Semaphore>>,
    seen: StdMutex<Vec<Invocation>>,
}

impl FakeRunner {
    fn new(output: &str, exit_code: i32) -> Self {
        Self {
            lines: output
                .lines()
                .map(|l| match l.strip_prefix("!") {
                    Some(err) => (Stream::Stderr, err.to_owned()),
                    None => (Stream::Stdout, l.to_owned()),
                })
                .collect(),
            exit_code,
            hang_until_cancelled: false,
            status_output: None,
            gate: None,
            seen: StdMutex::new(Vec::new()),
        }
    }

    fn with_status(mut self, output: &str) -> Self {
        self.status_output = Some(output.to_owned());
        self
    }

    fn gated(mut self) -> (Self, Arc<tokio::sync::Semaphore>) {
        let gate = Arc::new(tokio::sync::Semaphore::new(0));
        self.gate = Some(gate.clone());
        (self, gate)
    }

    fn calls(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|i| i.args.join(" "))
            .collect()
    }
}

impl Runner for FakeRunner {
    fn run<'a>(
        &'a self,
        invocation: Invocation,
        mut on_line: LineSink<'a>,
        cancel: CancellationToken,
    ) -> RunFuture<'a> {
        let is_status = invocation.args.first().is_some_and(|a| a == "status");
        self.seen.lock().unwrap().push(invocation);
        Box::pin(async move {
            if let (true, Some(output)) = (is_status, &self.status_output) {
                for line in output.lines() {
                    on_line(OutputLine {
                        stream: Stream::Stdout,
                        text: line.to_owned(),
                    });
                }
                return Ok(RunResult {
                    termination: Termination::Exited,
                    exit_code: Some(0),
                });
            }
            for (stream, text) in &self.lines {
                on_line(OutputLine {
                    stream: *stream,
                    text: text.clone(),
                });
            }
            if self.hang_until_cancelled {
                cancel.cancelled().await;
                return Ok(RunResult {
                    termination: Termination::Cancelled,
                    exit_code: None,
                });
            }
            if let Some(gate) = &self.gate {
                tokio::select! {
                    permit = gate.acquire() => permit.unwrap().forget(),
                    () = cancel.cancelled() => {
                        return Ok(RunResult {
                            termination: Termination::Cancelled,
                            exit_code: None,
                        });
                    }
                }
            }
            Ok(RunResult {
                termination: Termination::Exited,
                exit_code: Some(self.exit_code),
            })
        })
    }
}

#[derive(Default)]
struct Recorder(StdMutex<Vec<OperationEvent>>);

impl EventSink for Recorder {
    fn emit(&self, event: OperationEvent) {
        self.0.lock().unwrap().push(event);
    }
}

impl Recorder {
    fn kinds(&self) -> Vec<&'static str> {
        self.0.lock().unwrap().iter().map(|e| e.kind).collect()
    }
}

struct Harness {
    engine: OperationEngine,
    store: Store,
    events: Arc<Recorder>,
    runner: Arc<FakeRunner>,
    project: ProjectId,
    env: EnvironmentId,
    _dir: tempfile::TempDir,
}

async fn harness(runner: FakeRunner) -> Harness {
    let store = Store::open_in_memory().await.unwrap();
    let project = store
        .insert_project(&NewProject {
            name: "Web".into(),
            description: String::new(),
            root_path: "/p".into(),
            repository_url: None,
            tags: vec![],
        })
        .await
        .unwrap();
    let env = store
        .insert_environment(
            project.id,
            &NewEnvironment {
                name: "default".into(),
                root_path: "/p".into(),
                vagrantfile_path: Some("Vagrantfile".into()),
                default_provider: Some("vmware_desktop".into()),
                environment_type: EnvironmentType::Development,
            },
        )
        .await
        .unwrap()
        .id;
    let dir = tempfile::tempdir().unwrap();
    let events = Arc::new(Recorder::default());
    let runner = Arc::new(runner);
    let engine = OperationEngine::new(
        store.clone(),
        runner.clone(),
        events.clone(),
        logs_dir(dir.path()),
    );
    Harness {
        engine,
        store,
        events,
        runner,
        project: project.id,
        env,
        _dir: dir,
    }
}

fn status_request(env: EnvironmentId) -> OperationRequest {
    OperationRequest {
        operation_type: OperationType::Status,
        command: VagrantCommand::Status { machine: None },
        program: "vagrant".into(),
        working_dir: "/p".into(),
        project_id: None,
        environment_id: Some(env),
        machine_name: None,
        target_label: "Web · default".into(),
        env: vec![("VAGRANT_DEFAULT_PROVIDER".into(), "vmware_desktop".into())],
        timeout: Some(Duration::from_secs(60)),
        confirmation: None,
    }
}

fn names(names: &[&str]) -> Vec<MachineName> {
    names
        .iter()
        .map(|n| MachineName::try_from((*n).to_owned()).unwrap())
        .collect()
}

fn lifecycle_request(
    env: EnvironmentId,
    action: LifecycleAction,
    machines: &[&str],
) -> OperationRequest {
    OperationRequest {
        operation_type: lifecycle::operation_type(action),
        command: VagrantCommand::Lifecycle {
            action,
            machines: names(machines),
            provisioning: Provisioning::Default,
        },
        ..status_request(env)
    }
}

/// Waits until `check` holds (or fails the test after ~2 s).
async fn eventually(mut check: impl FnMut() -> bool) {
    for _ in 0..200 {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("condition not reached");
}

const STATUS_RUNNING: &str = "1,web,provider-name,virtualbox\n1,web,state,running\n1,db,provider-name,virtualbox\n1,db,state,running";
const STATUS_WEB_OFF: &str = "1,web,provider-name,virtualbox\n1,web,state,poweroff\n1,db,provider-name,virtualbox\n1,db,state,running";

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/../../fixtures/command-output/windows-vagrant-2.4.9/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[tokio::test]
async fn successful_status_records_operation_log_events_and_snapshot() {
    let h = harness(FakeRunner::new(
        &fixture("status-multi-not-created-vmware.txt"),
        0,
    ))
    .await;
    let op = h.engine.run(status_request(h.env)).await.unwrap();

    assert_eq!(op.status, OperationStatus::Succeeded);
    assert_eq!(op.exit_code, Some(0));
    assert_eq!(op.command_line, "vagrant status --machine-readable");
    assert!(op.started_at.is_some() && op.finished_at.is_some());

    {
        let seen = h.runner.seen.lock().unwrap();
        assert_eq!(seen[0].working_dir, PathBuf::from("/p"));
        assert_eq!(seen[0].args, ["status", "--machine-readable"]);
        assert_eq!(
            seen[0].env,
            [(
                "VAGRANT_DEFAULT_PROVIDER".to_owned(),
                "vmware_desktop".to_owned()
            )]
        );
    }

    let status = h.engine.environment_status(h.env).await.unwrap().unwrap();
    assert_eq!(status.operation_id, Some(op.id));
    assert_eq!(status.error, None);
    assert_eq!(
        status
            .machines
            .iter()
            .map(|m| (m.name.as_str(), m.state.as_deref()))
            .collect::<Vec<_>>(),
        [("web", Some("not_created")), ("db", Some("not_created"))]
    );

    let log = read_log(Path::new(op.log_path.as_deref().unwrap()), 1000).unwrap();
    assert!(
        log.iter()
            .any(|l| l.text.contains("command: vagrant status"))
    );
    assert!(log.iter().any(|l| l.text.contains("web,state,not_created")));
    assert!(
        log.iter()
            .any(|l| l.text.starts_with("finished: succeeded"))
    );

    let kinds = h.events.kinds();
    assert_eq!(kinds.first(), Some(&"operation.queued"));
    assert_eq!(kinds[1], "operation.started");
    assert!(kinds.contains(&"operation.stdout"));
    assert!(kinds.contains(&"environment.status_updated"));
    assert_eq!(kinds.last(), Some(&"operation.completed"));
    assert!(!h.engine.is_running(op.id));
}

#[tokio::test]
async fn vagrant_errors_fail_the_operation_with_a_summary_and_status_error() {
    let h = harness(FakeRunner::new(
        &fixture("status-vmware-utility-unreachable.txt"),
        1,
    ))
    .await;
    let op = h.engine.run(status_request(h.env)).await.unwrap();
    assert_eq!(op.status, OperationStatus::Failed);
    assert_eq!(op.exit_code, Some(1));
    assert!(
        op.error_summary
            .as_deref()
            .unwrap()
            .contains("Vagrant VMware Utility")
    );

    let status = h.engine.environment_status(h.env).await.unwrap().unwrap();
    assert!(status.machines.is_empty());
    assert_eq!(
        status.error.unwrap().class.as_deref(),
        Some("HashiCorp::VagrantVMwareDesktop::Errors::DriverAPIRequestUnexpectedError")
    );
    assert_eq!(h.events.kinds().last(), Some(&"operation.failed"));
}

#[tokio::test]
async fn failures_without_machine_readable_errors_use_stderr() {
    let h = harness(FakeRunner::new(
        "==> web: doing\n!first problem\n!The VM failed",
        1,
    ))
    .await;
    let mut req = status_request(h.env);
    req.operation_type = OperationType::Validate;
    req.command = VagrantCommand::Validate;
    let op = h.engine.run(req).await.unwrap();
    assert_eq!(
        op.error_summary.as_deref(),
        Some("first problem\nThe VM failed\n\nLast output before the error:\n==> web: doing")
    );
    assert_eq!(h.engine.environment_status(h.env).await.unwrap(), None);
}

#[tokio::test]
async fn cancellation_is_recorded_and_never_reported_as_success() {
    let mut runner = FakeRunner::new("==> web: Booting VM...", 0).with_status(STATUS_RUNNING);
    runner.hang_until_cancelled = true;
    let h = harness(runner).await;
    let queued = h
        .engine
        .start(lifecycle_request(h.env, LifecycleAction::Up, &["web"]))
        .await
        .unwrap();
    assert_eq!(queued.status, OperationStatus::Queued);

    // Wait until it is running, then cancel.
    for _ in 0..100 {
        if h.events.kinds().contains(&"operation.stdout") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(h.engine.cancel(queued.id).await.unwrap());
    for _ in 0..100 {
        if !h.engine.is_running(queued.id) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let op = h.store.get_operation(queued.id).await.unwrap().unwrap();
    assert_eq!(op.status, OperationStatus::Cancelled);
    assert!(op.error_summary.unwrap().contains("part-way"));
    let kinds = h.events.kinds();
    assert!(kinds.contains(&"operation.cancel_requested"));
    assert_eq!(kinds.last(), Some(&"operation.cancelled"));
    assert!(
        !h.engine.cancel(queued.id).await.unwrap(),
        "already finished"
    );
}

#[tokio::test]
async fn recovery_marks_leftover_operations_interrupted() {
    let h = harness(FakeRunner::new("", 0)).await;
    let op = h
        .store
        .insert_operation(
            &NewOperation {
                project_id: None,
                environment_id: Some(h.env),
                machine_name: None,
                machines: vec![],
                target_label: "x".into(),
                operation_type: OperationType::Up,
                command_line: "vagrant up".into(),
                working_dir: None,
            },
            "",
        )
        .await
        .unwrap();
    h.store.mark_operation_running(op.id).await.unwrap();
    assert_eq!(h.engine.recover().await.unwrap(), 1);
    assert_eq!(
        h.store.get_operation(op.id).await.unwrap().unwrap().status,
        OperationStatus::Interrupted
    );
}

#[tokio::test]
async fn secrets_never_reach_logs_or_events() {
    let h = harness(FakeRunner::new(
        "export DB_PASSWORD=hunter2\n!-----BEGIN RSA PRIVATE KEY-----\n!MIIEsecret\n!-----END RSA PRIVATE KEY-----",
        0,
    ))
    .await;
    let op = h.engine.run(status_request(h.env)).await.unwrap();
    let log = std::fs::read_to_string(op.log_path.unwrap()).unwrap();
    assert!(
        !log.contains("hunter2") && !log.contains("MIIEsecret"),
        "{log}"
    );
    let events = format!("{:?}", h.events.0.lock().unwrap());
    assert!(!events.contains("hunter2") && !events.contains("MIIEsecret"));
}

#[tokio::test]
async fn lifecycle_commands_name_machines_and_refresh_status_afterwards() {
    let h = harness(FakeRunner::new("==> web: Booting VM...", 0).with_status(STATUS_RUNNING)).await;
    let op = h
        .engine
        .run(lifecycle_request(
            h.env,
            LifecycleAction::Up,
            &["web", "db"],
        ))
        .await
        .unwrap();
    assert_eq!(op.status, OperationStatus::Succeeded);
    assert_eq!(op.operation_type, OperationType::Up);
    assert_eq!(op.error_summary, None);
    assert_eq!(
        h.runner.calls(),
        ["up --no-tty web db", "status --machine-readable"]
    );
    for machine in ["web", "db"] {
        let filter = OperationFilter {
            environment_id: Some(h.env),
            machine: Some(machine.into()),
            ..Default::default()
        };
        let found = h.engine.list(&filter, 10).await.unwrap();
        assert_eq!(found.iter().map(|o| o.id).collect::<Vec<_>>(), [op.id]);
    }

    let status = h.engine.environment_status(h.env).await.unwrap().unwrap();
    assert_eq!(status.operation_id, Some(op.id));
    assert!(
        status
            .machines
            .iter()
            .all(|m| m.state.as_deref() == Some("running"))
    );

    let kinds = h.events.kinds();
    let updated = kinds
        .iter()
        .position(|k| *k == "environment.status_updated")
        .unwrap();
    let completed = kinds
        .iter()
        .position(|k| *k == "operation.completed")
        .unwrap();
    assert!(updated < completed, "{kinds:?}");
    let log = read_log(Path::new(op.log_path.as_deref().unwrap()), 1000).unwrap();
    assert!(
        log.iter()
            .any(|l| l.text.starts_with("refreshing machine state"))
    );
}

#[tokio::test]
async fn contradicting_state_after_success_is_a_warning() {
    let h = harness(FakeRunner::new("", 0).with_status(STATUS_WEB_OFF)).await;
    let op = h
        .engine
        .run(lifecycle_request(
            h.env,
            LifecycleAction::Up,
            &["web", "db"],
        ))
        .await
        .unwrap();
    assert_eq!(op.status, OperationStatus::Succeeded);
    assert_eq!(
        op.error_summary.as_deref(),
        Some("Vagrant reports web is poweroff afterwards (expected running).")
    );
    assert!(h.events.kinds().contains(&"operation.warning"));
}

#[tokio::test]
async fn failed_lifecycle_keeps_its_error_but_still_refreshes_status() {
    let h = harness(FakeRunner::new("!The provider failed", 1).with_status(STATUS_WEB_OFF)).await;
    let op = h
        .engine
        .run(lifecycle_request(h.env, LifecycleAction::Up, &["web"]))
        .await
        .unwrap();
    assert_eq!(op.status, OperationStatus::Failed);
    assert_eq!(op.error_summary.as_deref(), Some("The provider failed"));
    let status = h.engine.environment_status(h.env).await.unwrap().unwrap();
    assert_eq!(status.machines[0].state.as_deref(), Some("poweroff"));
}

#[tokio::test]
async fn mutating_operations_on_one_environment_run_one_at_a_time() {
    let (runner, gate) = FakeRunner::new("", 0).with_status(STATUS_RUNNING).gated();
    let h = harness(runner).await;
    let first = h
        .engine
        .start(lifecycle_request(h.env, LifecycleAction::Up, &["web"]))
        .await
        .unwrap();
    let second = h
        .engine
        .start(lifecycle_request(h.env, LifecycleAction::Reload, &["web"]))
        .await
        .unwrap();
    let refresh = h.engine.start(status_request(h.env)).await.unwrap();
    eventually(|| h.runner.calls().len() == 1).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(h.runner.calls(), ["up --no-tty web"], "others wait");
    for id in [second.id, refresh.id] {
        let op = h.store.get_operation(id).await.unwrap().unwrap();
        assert_eq!(op.status, OperationStatus::Queued);
    }

    gate.add_permits(2);
    eventually(|| !h.engine.is_running(refresh.id)).await;
    assert_eq!(
        h.runner.calls(),
        [
            "up --no-tty web",
            "status --machine-readable",
            "reload --no-tty web",
            "status --machine-readable",
            "status --machine-readable",
        ],
        "requests start in order, each lifecycle command with its refresh"
    );
    for id in [first.id, second.id, refresh.id] {
        let op = h.store.get_operation(id).await.unwrap().unwrap();
        assert_eq!(op.status, OperationStatus::Succeeded);
    }
}

#[tokio::test]
async fn different_environments_run_in_parallel() {
    let (runner, gate) = FakeRunner::new("", 0).with_status(STATUS_RUNNING).gated();
    let h = harness(runner).await;
    let other = h
        .store
        .insert_environment(
            h.project,
            &NewEnvironment {
                name: "staging".into(),
                root_path: "/p/staging".into(),
                vagrantfile_path: Some("Vagrantfile".into()),
                default_provider: None,
                environment_type: EnvironmentType::Staging,
            },
        )
        .await
        .unwrap()
        .id;
    h.engine
        .start(lifecycle_request(h.env, LifecycleAction::Halt, &["web"]))
        .await
        .unwrap();
    h.engine
        .start(lifecycle_request(other, LifecycleAction::Halt, &["web"]))
        .await
        .unwrap();
    eventually(|| h.runner.calls().len() == 2).await;
    gate.add_permits(2);
}

#[tokio::test]
async fn duplicate_requests_are_refused_as_conflicts() {
    let (runner, gate) = FakeRunner::new("", 0).with_status(STATUS_RUNNING).gated();
    let h = harness(runner).await;
    let first = h
        .engine
        .start(lifecycle_request(h.env, LifecycleAction::Halt, &["web"]))
        .await
        .unwrap();
    let err = h
        .engine
        .start(lifecycle_request(h.env, LifecycleAction::Halt, &["web"]))
        .await
        .unwrap_err();
    assert!(matches!(err, EngineError::Conflict(ref m) if m.contains("vagrant halt --no-tty web")));
    // A different target is fine.
    h.engine
        .start(lifecycle_request(h.env, LifecycleAction::Halt, &["db"]))
        .await
        .unwrap();
    gate.add_permits(2);
    eventually(|| !h.engine.is_running(first.id)).await;
    assert_eq!(
        h.store
            .list_operations(&Default::default(), 10)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn cancelling_a_queued_operation_means_it_never_runs() {
    let (runner, gate) = FakeRunner::new("", 0).with_status(STATUS_RUNNING).gated();
    let h = harness(runner).await;
    let first = h
        .engine
        .start(lifecycle_request(h.env, LifecycleAction::Up, &["web"]))
        .await
        .unwrap();
    let queued = h
        .engine
        .start(lifecycle_request(
            h.env,
            LifecycleAction::Provision,
            &["web"],
        ))
        .await
        .unwrap();
    eventually(|| h.runner.calls().len() == 1).await;
    assert!(h.engine.cancel(queued.id).await.unwrap());
    eventually(|| !h.engine.is_running(queued.id)).await;
    let op = h.store.get_operation(queued.id).await.unwrap().unwrap();
    assert_eq!(op.status, OperationStatus::Cancelled);
    assert_eq!(op.started_at, None);
    assert_eq!(
        op.error_summary.as_deref(),
        Some("Cancelled before it started.")
    );

    gate.add_permits(1);
    eventually(|| !h.engine.is_running(first.id)).await;
    assert!(!h.runner.calls().iter().any(|c| c.starts_with("provision")));
}

#[tokio::test]
async fn cancelled_lifecycle_still_refreshes_status() {
    let mut runner = FakeRunner::new("==> web: Booting VM...", 0).with_status(STATUS_WEB_OFF);
    runner.hang_until_cancelled = true;
    let h = harness(runner).await;
    let op = h
        .engine
        .start(lifecycle_request(h.env, LifecycleAction::Up, &["web"]))
        .await
        .unwrap();
    eventually(|| h.runner.calls().len() == 1).await;
    h.engine.cancel(op.id).await.unwrap();
    eventually(|| !h.engine.is_running(op.id)).await;
    assert_eq!(
        h.store.get_operation(op.id).await.unwrap().unwrap().status,
        OperationStatus::Cancelled
    );
    assert_eq!(h.runner.calls()[1], "status --machine-readable");
    assert!(h.engine.environment_status(h.env).await.unwrap().is_some());
}

#[tokio::test]
async fn destroy_requires_a_matching_single_use_confirmation() {
    let h = harness(
        FakeRunner::new("", 0)
            .with_status("1,web,provider-name,virtualbox\n1,web,state,not_created"),
    )
    .await;
    let destroy = || lifecycle_request(h.env, LifecycleAction::Destroy, &["web"]);
    let proof = |c: &Confirmation, typed: &str| {
        Some(ConfirmationProof {
            token: c.token.clone(),
            typed: typed.into(),
        })
    };

    let err = h.engine.run(destroy()).await.unwrap_err();
    assert!(matches!(err, EngineError::Confirmation(_)));

    let c = h
        .engine
        .confirm(Some(h.env), destroy().command, "web".into());
    let mut req = destroy();
    req.confirmation = proof(&c, "db");
    assert!(matches!(
        h.engine.run(req).await,
        Err(EngineError::Confirmation(_))
    ));

    // A token for one machine can't destroy another.
    let c = h
        .engine
        .confirm(Some(h.env), destroy().command, "web".into());
    let mut req = lifecycle_request(h.env, LifecycleAction::Destroy, &["db"]);
    req.confirmation = proof(&c, "web");
    assert!(matches!(
        h.engine.run(req).await,
        Err(EngineError::Confirmation(_))
    ));

    let c = h
        .engine
        .confirm(Some(h.env), destroy().command, "web".into());
    let mut req = destroy();
    req.confirmation = proof(&c, " web ");
    let op = h.engine.run(req.clone()).await.unwrap();
    assert_eq!(op.status, OperationStatus::Succeeded);
    assert_eq!(h.runner.calls()[0], "destroy --force --no-tty web");
    assert!(
        matches!(h.engine.run(req).await, Err(EngineError::Confirmation(_))),
        "tokens are single-use"
    );
    assert_eq!(
        h.runner.calls().len(),
        2,
        "nothing ran after the first destroy"
    );
}

#[tokio::test]
async fn provisioner_failures_include_the_guest_output() {
    // Vagrant 2.4.9 prints guest output via `ui.detail` (stdout) and only its
    // generic SSH error to stderr.
    let h = harness(
        FakeRunner::new(
            "==> web: Running provisioner: shell...\n    web: Running: inline script\n    web: E: Could not get lock /var/lib/dpkg/lock-frontend. It is held by process 1234 (unattended-upgr)\n\n!The SSH command responded with a non-zero exit status. Vagrant\n!assumes that this means the command failed.",
            1,
        )
        .with_status(STATUS_RUNNING),
    )
    .await;
    let op = h
        .engine
        .run(lifecycle_request(
            h.env,
            LifecycleAction::Provision,
            &["web"],
        ))
        .await
        .unwrap();
    assert_eq!(op.status, OperationStatus::Failed);
    let summary = op.error_summary.unwrap();
    assert!(
        summary.starts_with("The SSH command responded"),
        "{summary}"
    );
    assert!(
        summary.contains("Last output before the error:\n==> web: Running provisioner: shell...\nweb: Running: inline script\nweb: E: Could not get lock"),
        "{summary}"
    );
    let results = h.engine.operation_provisioning(op.id).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].machine, "web");
    assert_eq!(
        results[0].runs[0].status,
        vagrant_studio_vagrant_adapter::provision::ProvisionerStatus::Failed
    );
}

#[tokio::test]
async fn provisioner_results_are_stored_per_machine() {
    use vagrant_studio_vagrant_adapter::provision::ProvisionerStatus;
    let h =
        harness(FakeRunner::new(&fixture("up-provision-multi.txt"), 0).with_status(STATUS_RUNNING))
            .await;
    let up = h
        .engine
        .run(lifecycle_request(
            h.env,
            LifecycleAction::Up,
            &["web", "db"],
        ))
        .await
        .unwrap();
    assert_eq!(up.status, OperationStatus::Succeeded);
    assert!(h.events.kinds().contains(&"operation.provisioning"));

    let results = h.engine.operation_provisioning(up.id).await.unwrap();
    let machines: Vec<_> = results.iter().map(|m| m.machine.as_str()).collect();
    assert_eq!(machines, ["web", "db"]);
    assert_eq!(results[0].runs.len(), 3);
    assert!(
        results
            .iter()
            .flat_map(|m| &m.runs)
            .all(|r| r.status == ProvisionerStatus::Succeeded)
    );

    // A later run replaces only the machines it provisioned as "latest".
    let engine = OperationEngine::new(
        h.store.clone(),
        Arc::new(FakeRunner::new(&fixture("provision-fail.txt"), 1).with_status(STATUS_RUNNING)),
        h.events.clone(),
        logs_dir(h._dir.path()),
    );
    let failed = engine
        .run(lifecycle_request(
            h.env,
            LifecycleAction::Provision,
            &["web"],
        ))
        .await
        .unwrap();
    assert_eq!(failed.status, OperationStatus::Failed);
    let latest = engine.latest_provisioning(h.env).await.unwrap();
    let summary: Vec<_> = latest
        .iter()
        .map(|m| (m.machine.as_str(), m.operation_id, m.runs[0].status))
        .collect();
    assert_eq!(
        summary,
        [
            ("db", up.id, ProvisionerStatus::Succeeded),
            ("web", failed.id, ProvisionerStatus::Failed),
        ]
    );

    // Commands that don't provision record nothing.
    let halt = engine
        .run(lifecycle_request(h.env, LifecycleAction::Halt, &["web"]))
        .await
        .unwrap();
    assert!(
        engine
            .operation_provisioning(halt.id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn environment_lock_is_refused_while_busy_and_holds_operations_back() {
    let (runner, gate) = FakeRunner::new("", 0).with_status(STATUS_RUNNING).gated();
    let h = harness(runner).await;
    let up = h
        .engine
        .start(lifecycle_request(h.env, LifecycleAction::Up, &["web"]))
        .await
        .unwrap();
    assert!(h.engine.try_lock_environment(h.env).is_none(), "running");
    gate.add_permits(1);
    eventually(|| !h.engine.is_running(up.id)).await;

    let lock = h.engine.try_lock_environment(h.env).expect("idle");
    assert!(
        h.engine.try_lock_environment(h.env).is_none(),
        "already held"
    );
    let status = h.engine.start(status_request(h.env)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let calls = h.runner.calls().len();
    let op = h.store.get_operation(status.id).await.unwrap().unwrap();
    assert_eq!(op.status, OperationStatus::Queued, "waits for the lock");
    drop(lock);
    gate.add_permits(1);
    eventually(|| !h.engine.is_running(status.id)).await;
    assert_eq!(h.runner.calls().len(), calls + 1);
}

fn box_ref(version: &str) -> vagrant_studio_vagrant_adapter::boxes::BoxRef {
    serde_json::from_value(serde_json::json!({
        "name": "almalinux/9", "provider": "vmware_desktop",
        "version": version, "architecture": "amd64",
    }))
    .unwrap()
}

/// A host-level box command (no project or environment).
fn box_request(command: VagrantCommand) -> OperationRequest {
    OperationRequest {
        operation_type: match command {
            VagrantCommand::BoxRemove(_) => OperationType::BoxRemove,
            VagrantCommand::BoxUpdate { .. } => OperationType::BoxUpdate,
            _ => OperationType::BoxAdd,
        },
        command,
        program: "vagrant".into(),
        working_dir: "/tmp".into(),
        project_id: None,
        environment_id: None,
        machine_name: None,
        target_label: "Boxes".into(),
        env: vec![],
        timeout: Some(Duration::from_secs(60)),
        confirmation: None,
    }
}

#[tokio::test]
async fn box_remove_needs_a_confirmation_for_that_exact_box() {
    let h = harness(FakeRunner::new("", 0)).await;
    let remove = |v: &str| box_request(VagrantCommand::BoxRemove(box_ref(v)));
    assert!(matches!(
        h.engine.run(remove("1")).await,
        Err(EngineError::Confirmation(_))
    ));

    let c = h
        .engine
        .confirm(None, remove("1").command, "almalinux/9".into());
    let mut other = remove("2");
    other.confirmation = Some(ConfirmationProof {
        token: c.token.clone(),
        typed: "almalinux/9".into(),
    });
    assert!(matches!(
        h.engine.run(other).await,
        Err(EngineError::Confirmation(_))
    ));
    // Nor can a box token be used for an environment.
    let c = h
        .engine
        .confirm(None, remove("1").command, "almalinux/9".into());
    let mut in_env = remove("1");
    in_env.environment_id = Some(h.env);
    in_env.confirmation = Some(ConfirmationProof {
        token: c.token,
        typed: "almalinux/9".into(),
    });
    assert!(h.engine.run(in_env).await.is_err());

    let c = h
        .engine
        .confirm(None, remove("1").command, "almalinux/9".into());
    let mut req = remove("1");
    req.confirmation = Some(ConfirmationProof {
        token: c.token,
        typed: "almalinux/9".into(),
    });
    let op = h.engine.run(req).await.unwrap();
    assert_eq!(op.status, OperationStatus::Succeeded);
    assert_eq!(op.environment_id, None);
    assert_eq!(op.operation_type, OperationType::BoxRemove);
    assert_eq!(
        h.runner.calls().len(),
        1,
        "no status refresh for box commands"
    );
}

#[tokio::test]
async fn box_changes_wait_for_running_ups_and_hold_new_ones_back() {
    let (runner, gate) = FakeRunner::new("", 0).with_status(STATUS_RUNNING).gated();
    let h = harness(runner).await;
    let up = h
        .engine
        .start(lifecycle_request(h.env, LifecycleAction::Up, &["web"]))
        .await
        .unwrap();
    eventually(|| h.runner.calls().len() == 1).await;
    let update = h
        .engine
        .start(box_request(VagrantCommand::BoxUpdate {
            name: "almalinux/9".into(),
            provider: "vmware_desktop".into(),
            architecture: None,
        }))
        .await
        .unwrap();
    // A duplicate box change is refused rather than queued twice.
    assert!(matches!(
        h.engine
            .start(box_request(VagrantCommand::BoxUpdate {
                name: "almalinux/9".into(),
                provider: "vmware_desktop".into(),
                architecture: None,
            }))
            .await,
        Err(EngineError::Conflict(_))
    ));
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(h.runner.calls().len(), 1, "update waits for the running up");

    // up (1 permit) → status; then the update starts and blocks the next up.
    gate.add_permits(1);
    eventually(|| !h.engine.is_running(up.id)).await;
    eventually(|| h.runner.calls().iter().any(|c| c.starts_with("box update"))).await;
    let second_up = h
        .engine
        .start(lifecycle_request(h.env, LifecycleAction::Up, &["db"]))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        !h.runner.calls().iter().any(|c| c == "up --no-tty db"),
        "up waits for the box change"
    );
    gate.add_permits(2);
    eventually(|| !h.engine.is_running(update.id) && !h.engine.is_running(second_up.id)).await;
    assert!(h.runner.calls().iter().any(|c| c == "up --no-tty db"));
}
