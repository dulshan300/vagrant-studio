use std::sync::Mutex as StdMutex;

use vagrant_studio_domain::{EnvironmentType, NewEnvironment, NewProject};
use vagrant_studio_vagrant_adapter::runner::{LineSink, RunFuture};
use vagrant_studio_vagrant_adapter::{OutputLine, RunResult};

use super::*;

/// Replays scripted output; optionally waits for cancellation instead of exiting.
struct FakeRunner {
    lines: Vec<(Stream, String)>,
    exit_code: i32,
    hang_until_cancelled: bool,
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
            seen: StdMutex::new(Vec::new()),
        }
    }
}

impl Runner for FakeRunner {
    fn run<'a>(
        &'a self,
        invocation: Invocation,
        mut on_line: LineSink<'a>,
        cancel: CancellationToken,
    ) -> RunFuture<'a> {
        self.seen.lock().unwrap().push(invocation);
        Box::pin(async move {
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
    }
}

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
        Some("first problem\nThe VM failed")
    );
    assert_eq!(h.engine.environment_status(h.env).await.unwrap(), None);
}

#[tokio::test]
async fn cancellation_is_recorded_and_never_reported_as_success() {
    let mut runner = FakeRunner::new("==> web: Booting VM...", 0);
    runner.hang_until_cancelled = true;
    let h = harness(runner).await;
    let mut req = status_request(h.env);
    req.operation_type = OperationType::Up;
    req.command = VagrantCommand::Up {
        machine: None,
        provider: None,
    };
    let queued = h.engine.start(req).await.unwrap();
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
