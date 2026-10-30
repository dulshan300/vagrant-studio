//! Runs Vagrant commands as recorded operations.
//!
//! Every operation gets a database row (status, exit code, summary) and a
//! log file. Output is redacted, written to the log and streamed as events.
//! The final state is persisted by the backend, so history never depends on
//! the frontend. Operations left running by a crash are marked
//! `interrupted` on the next start ([`OperationEngine::recover`]).
//!
//! Operations wait in `queued` until the scheduler's locks allow them to
//! start. After a command that changes machines, the engine runs
//! `vagrant status` while still holding the environment lock, so the
//! recorded state is never older than the operation's outcome.

mod confirm;
pub mod events;
pub mod lifecycle;
mod log;
mod provisioning;
pub mod redact;
mod scheduler;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use tokio_util::sync::CancellationToken;
use vagrant_studio_domain::{
    EnvironmentId, NewOperation, Operation, OperationId, OperationStatus, OperationType, ProjectId,
};
pub use vagrant_studio_project_store::OperationFilter;
use vagrant_studio_project_store::{EnvironmentStatusRecord, Store, StoreError};
use vagrant_studio_vagrant_adapter::machine_readable::{self, MachineStatus};
use vagrant_studio_vagrant_adapter::{
    Invocation, RunResult, Runner, Stream, Termination, VagrantCommand,
};

pub use confirm::{Confirmation, ConfirmationProof};
pub use events::{EventSink, OperationEvent};
pub use log::{LogLine, read_log};
pub use provisioning::MachineProvisioning;

/// Lines of stdout kept in memory for parsing (status output is small).
const MAX_CAPTURED_LINES: usize = 20_000;
/// Limit for the status refresh that follows a mutating command.
const FOLLOW_UP_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("could not access the log {path}: {source}")]
    Logs {
        path: String,
        source: std::io::Error,
    },
    /// The same command is already queued or running for the target.
    #[error("{0}")]
    Conflict(String),
    /// A destructive command without a valid confirmation.
    #[error("{0}")]
    Confirmation(String),
    /// The request can't be run as given (e.g. unknown machine names).
    #[error("{0}")]
    Invalid(String),
}

/// Everything needed to run one Vagrant command for a target.
#[derive(Debug, Clone)]
pub struct OperationRequest {
    pub operation_type: OperationType,
    pub command: VagrantCommand,
    pub program: PathBuf,
    pub working_dir: PathBuf,
    pub project_id: Option<ProjectId>,
    pub environment_id: Option<EnvironmentId>,
    pub machine_name: Option<String>,
    pub target_label: String,
    pub env: Vec<(String, String)>,
    pub timeout: Option<Duration>,
    /// Required for destructive commands; see [`OperationEngine::confirm`].
    pub confirmation: Option<ConfirmationProof>,
}

/// Parsed result of the last `vagrant status` for an environment.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentStatus {
    pub environment_id: EnvironmentId,
    pub operation_id: Option<OperationId>,
    #[serde(with = "time::serde::rfc3339")]
    pub observed_at: time::OffsetDateTime,
    pub machines: Vec<MachineStatusDto>,
    pub error: Option<StatusError>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineStatusDto {
    pub name: String,
    pub provider: Option<String>,
    pub state: Option<String>,
    pub state_short: Option<String>,
    pub state_long: Option<String>,
}

impl From<MachineStatus> for MachineStatusDto {
    fn from(m: MachineStatus) -> Self {
        Self {
            name: m.name,
            provider: m.provider,
            state: m.state,
            state_short: m.state_short,
            state_long: m.state_long,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusError {
    pub class: Option<String>,
    pub message: String,
}

/// Holds an environment's write lock; operations for it wait until dropped.
pub struct EnvironmentLock {
    _guard: tokio::sync::OwnedRwLockWriteGuard<()>,
}

#[derive(Clone)]
pub struct OperationEngine {
    inner: Arc<Inner>,
}

struct Inner {
    store: Store,
    runner: Arc<dyn Runner>,
    sink: Arc<dyn EventSink>,
    logs_dir: PathBuf,
    /// Queued and running operations. Entries are reserved (id `None`)
    /// before the database row exists, so duplicate checks can't race.
    active: Mutex<HashMap<u64, Active>>,
    next_slot: std::sync::atomic::AtomicU64,
    locks: scheduler::Locks,
    confirmations: confirm::Confirmations,
}

struct Active {
    id: Option<OperationId>,
    environment_id: Option<EnvironmentId>,
    command: VagrantCommand,
    token: CancellationToken,
}

/// Output of one process run.
struct Captured {
    outcome: std::io::Result<RunResult>,
    stdout: Vec<String>,
    stderr_tail: Vec<String>,
}

impl OperationEngine {
    pub fn new(
        store: Store,
        runner: Arc<dyn Runner>,
        sink: Arc<dyn EventSink>,
        logs_dir: PathBuf,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                store,
                runner,
                sink,
                logs_dir,
                active: Mutex::new(HashMap::new()),
                next_slot: Default::default(),
                locks: Default::default(),
                confirmations: Default::default(),
            }),
        }
    }

    /// Issues a one-time token the UI must send back with a destructive
    /// command for exactly this target, along with the typed `phrase`.
    pub fn confirm(
        &self,
        environment_id: Option<EnvironmentId>,
        command: VagrantCommand,
        phrase: String,
    ) -> Confirmation {
        self.inner
            .confirmations
            .issue(environment_id, command, phrase)
    }

    /// Marks operations left unfinished by a previous session as interrupted.
    pub async fn recover(&self) -> Result<u64, EngineError> {
        let n = self.inner.store.interrupt_unfinished_operations().await?;
        if n > 0 {
            tracing::warn!(
                count = n,
                "marked operations from a previous session as interrupted"
            );
        }
        Ok(n)
    }

    /// Records the operation and runs it in the background once the
    /// scheduler allows. Returns the queued record.
    pub async fn start(&self, req: OperationRequest) -> Result<Operation, EngineError> {
        let (op, slot, token) = self.prepare(&req).await?;
        let engine = self.clone();
        let queued = op.clone();
        tokio::spawn(async move {
            if let Err(e) = engine.execute(op, slot, req, token).await {
                tracing::error!(error = %e, "operation bookkeeping failed");
            }
        });
        Ok(queued)
    }

    /// Like [`start`](Self::start) but waits for the final record.
    pub async fn run(&self, req: OperationRequest) -> Result<Operation, EngineError> {
        let (op, slot, token) = self.prepare(&req).await?;
        self.execute(op, slot, req, token).await
    }

    /// Requests cancellation of a queued or running operation. Returns false
    /// if it isn't active here.
    pub async fn cancel(&self, id: OperationId) -> Result<bool, EngineError> {
        let token = self
            .inner
            .active
            .lock()
            .expect("lock")
            .values()
            .find(|a| a.id == Some(id))
            .map(|a| a.token.clone());
        let Some(token) = token else {
            return Ok(false);
        };
        self.inner.store.mark_operation_cancel_requested(id).await?;
        let op = self.inner.store.get_operation(id).await?;
        self.emit("operation.cancel_requested", op.as_ref(), json!({}));
        token.cancel();
        Ok(true)
    }

    pub async fn get(&self, id: OperationId) -> Result<Option<Operation>, EngineError> {
        Ok(self.inner.store.get_operation(id).await?)
    }

    /// Newest first, narrowed by `filter`.
    pub async fn list(
        &self,
        filter: &OperationFilter,
        limit: i64,
    ) -> Result<Vec<Operation>, EngineError> {
        Ok(self
            .inner
            .store
            .list_operations(filter, limit.clamp(1, 500))
            .await?)
    }

    /// Folder holding the operation logs (it may not exist yet).
    pub fn logs_folder(&self) -> &Path {
        &self.inner.logs_dir
    }

    /// The last `limit` lines of an operation's log (empty if it has none yet).
    pub async fn log(&self, id: OperationId, limit: usize) -> Result<Vec<LogLine>, EngineError> {
        let Some(path) = self.get(id).await?.and_then(|op| op.log_path) else {
            return Ok(Vec::new());
        };
        match read_log(Path::new(&path), limit) {
            Ok(lines) => Ok(lines),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(source) => Err(EngineError::Logs { path, source }),
        }
    }

    /// Takes the environment's write lock for work outside Vagrant (saving
    /// its Vagrantfile), so no operation runs on the environment meanwhile.
    /// `None` while any operation is queued or running for it.
    pub fn try_lock_environment(&self, environment_id: EnvironmentId) -> Option<EnvironmentLock> {
        let busy = self
            .inner
            .active
            .lock()
            .expect("lock")
            .values()
            .any(|a| a.environment_id == Some(environment_id));
        if busy {
            return None;
        }
        self.inner
            .locks
            .try_write(environment_id)
            .map(|guard| EnvironmentLock { _guard: guard })
    }

    /// Whether the operation is queued or running.
    pub fn is_running(&self, id: OperationId) -> bool {
        self.inner
            .active
            .lock()
            .expect("lock")
            .values()
            .any(|a| a.id == Some(id))
    }

    pub async fn environment_status(
        &self,
        environment_id: EnvironmentId,
    ) -> Result<Option<EnvironmentStatus>, EngineError> {
        Ok(self
            .inner
            .store
            .get_environment_status(environment_id)
            .await?
            .and_then(status_from_record))
    }

    /// Last observed status of every environment that has one.
    pub async fn all_environment_statuses(&self) -> Result<Vec<EnvironmentStatus>, EngineError> {
        Ok(self
            .inner
            .store
            .list_environment_statuses()
            .await?
            .into_iter()
            .filter_map(status_from_record)
            .collect())
    }

    /// Provisioner results of one operation, per machine.
    pub async fn operation_provisioning(
        &self,
        id: OperationId,
    ) -> Result<Vec<MachineProvisioning>, EngineError> {
        Ok(self
            .inner
            .store
            .operation_provisioning(id)
            .await?
            .into_iter()
            .filter_map(provisioning::from_record)
            .collect())
    }

    /// The most recent provisioner results of each machine of an environment.
    pub async fn latest_provisioning(
        &self,
        environment_id: EnvironmentId,
    ) -> Result<Vec<MachineProvisioning>, EngineError> {
        Ok(self
            .inner
            .store
            .latest_provisioning(environment_id)
            .await?
            .into_iter()
            .filter_map(provisioning::from_record)
            .collect())
    }

    async fn prepare(
        &self,
        req: &OperationRequest,
    ) -> Result<(Operation, u64, CancellationToken), EngineError> {
        if req.command.is_destructive() {
            let Some(proof) = &req.confirmation else {
                return Err(EngineError::Confirmation(
                    "This action needs confirmation.".into(),
                ));
            };
            self.inner
                .confirmations
                .redeem(proof, req.environment_id, &req.command)
                .map_err(|m| EngineError::Confirmation(m.into()))?;
        }
        let (slot, token) = self.reserve(req)?;
        match self.insert(req).await {
            Ok(op) => {
                if let Some(a) = self.inner.active.lock().expect("lock").get_mut(&slot) {
                    a.id = Some(op.id);
                }
                self.emit("operation.queued", Some(&op), json!({ "operation": op }));
                Ok((op, slot, token))
            }
            Err(e) => {
                self.inner.active.lock().expect("lock").remove(&slot);
                Err(e)
            }
        }
    }

    /// Registers the request as active unless the same command is already
    /// queued or running for the same environment (or, for host-level
    /// commands such as box changes, on the host).
    fn reserve(&self, req: &OperationRequest) -> Result<(u64, CancellationToken), EngineError> {
        let mut active = self.inner.active.lock().expect("lock");
        let checked = req.environment_id.is_some() || req.command.changes_boxes();
        if checked
            && active
                .values()
                .any(|a| a.environment_id == req.environment_id && a.command == req.command)
        {
            return Err(EngineError::Conflict(format!(
                "`{}` is already queued or running{}.",
                command_line(&req.command),
                if req.environment_id.is_some() {
                    " for this environment"
                } else {
                    ""
                }
            )));
        }
        let slot = self
            .inner
            .next_slot
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let token = CancellationToken::new();
        active.insert(
            slot,
            Active {
                id: None,
                environment_id: req.environment_id,
                command: req.command.clone(),
                token: token.clone(),
            },
        );
        Ok((slot, token))
    }

    async fn insert(&self, req: &OperationRequest) -> Result<Operation, EngineError> {
        std::fs::create_dir_all(&self.inner.logs_dir).map_err(|source| EngineError::Logs {
            path: self.inner.logs_dir.display().to_string(),
            source,
        })?;
        let log_template = self.inner.logs_dir.join("{id}.log");
        Ok(self
            .inner
            .store
            .insert_operation(
                &NewOperation {
                    project_id: req.project_id,
                    environment_id: req.environment_id,
                    machine_name: req.machine_name.clone(),
                    machines: named_machines(&req.command),
                    target_label: req.target_label.clone(),
                    operation_type: req.operation_type.clone(),
                    command_line: command_line(&req.command),
                    working_dir: Some(req.working_dir.display().to_string()),
                },
                &log_template.display().to_string(),
            )
            .await?)
    }

    async fn execute(
        &self,
        op: Operation,
        slot: u64,
        req: OperationRequest,
        token: CancellationToken,
    ) -> Result<Operation, EngineError> {
        let result = self.execute_inner(&op, &req, token).await;
        self.inner.active.lock().expect("lock").remove(&slot);
        result
    }

    async fn execute_inner(
        &self,
        op: &Operation,
        req: &OperationRequest,
        token: CancellationToken,
    ) -> Result<Operation, EngineError> {
        // Wait for the locks; cancelling while queued means it never starts.
        let held = tokio::select! {
            biased;
            () = token.cancelled() => None,
            held = self.inner.locks.acquire(req.environment_id, &req.command) => Some(held),
        };
        let Some(_held) = held else {
            let op = self
                .inner
                .store
                .finish_operation(
                    op.id,
                    OperationStatus::Cancelled,
                    None,
                    Some("Cancelled before it started."),
                )
                .await?;
            self.emit("operation.cancelled", Some(&op), json!({ "operation": op }));
            return Ok(op);
        };

        let op = self.inner.store.mark_operation_running(op.id).await?;
        self.emit("operation.started", Some(&op), json!({ "operation": op }));

        let log_path = PathBuf::from(op.log_path.clone().unwrap_or_default());
        let mut log = log::LogWriter::create(&log_path, &op);
        let mut redactor = redact::Redactor::default();

        let captured = self
            .invoke(
                &op,
                Invocation {
                    program: req.program.clone(),
                    args: req.command.args(),
                    working_dir: req.working_dir.clone(),
                    timeout: req.timeout,
                    env: req.env.clone(),
                },
                token,
                &mut log,
                &mut redactor,
            )
            .await;

        let parsed = machine_readable::parse(&captured.stdout.join("\n"));
        let vagrant_error = machine_readable::error_exit(&parsed);
        let (status, exit_code, mut summary) = match &captured.outcome {
            Err(e) => (
                OperationStatus::Failed,
                None,
                Some(format!("Could not start Vagrant: {e}")),
            ),
            Ok(r) => match r.termination {
                Termination::Cancelled => (
                    OperationStatus::Cancelled,
                    r.exit_code,
                    Some(if req.command.is_mutating() {
                        "Cancelled. The machine may have been left part-way; check its state before continuing.".to_owned()
                    } else if req.command.changes_boxes() {
                        "Cancelled. Refresh the box list to see what is installed now.".to_owned()
                    } else {
                        "Cancelled.".to_owned()
                    }),
                ),
                Termination::TimedOut => (
                    OperationStatus::Failed,
                    r.exit_code,
                    Some(format!(
                        "Timed out after {} seconds and was stopped.",
                        req.timeout.map(|t| t.as_secs()).unwrap_or_default()
                    )),
                ),
                Termination::Exited if r.exit_code == Some(0) => {
                    (OperationStatus::Succeeded, r.exit_code, None)
                }
                Termination::Exited => (
                    OperationStatus::Failed,
                    r.exit_code,
                    Some(failure_summary(
                        vagrant_error.as_ref().map(|e| e.message.as_str()),
                        &captured.stderr_tail,
                        &human_output_tail(&req.command, &captured.stdout),
                        r.exit_code,
                    )),
                ),
            },
        };

        if req.operation_type == OperationType::Status
            && let Some(env) = req.environment_id
            && matches!(status, OperationStatus::Succeeded | OperationStatus::Failed)
        {
            self.record_status(env, &op, &parsed, vagrant_error, summary.as_deref())
                .await?;
        }

        let provisioned = provisioning::collect(
            op.id,
            &req.command,
            &captured.outcome,
            status,
            &captured.stdout,
        );
        if !provisioned.is_empty() {
            let records: Vec<_> = provisioned
                .iter()
                .map(|m| provisioning::to_record(req.environment_id, m))
                .collect();
            self.inner.store.save_provisioning(&records).await?;
            self.emit(
                "operation.provisioning",
                Some(&op),
                json!({ "machines": provisioned }),
            );
        }

        if req.command.is_mutating()
            && let Some(env) = req.environment_id
            && captured.outcome.is_ok()
        {
            let warning = self
                .follow_up_status(env, &op, req, status, &mut log, &mut redactor)
                .await?;
            if let Some(warning) = warning {
                self.emit(
                    "operation.warning",
                    Some(&op),
                    json!({ "message": warning }),
                );
                if status == OperationStatus::Succeeded {
                    summary = Some(warning);
                }
            }
        }

        log.footer(status, exit_code, summary.as_deref());
        drop(log);

        let op = self
            .inner
            .store
            .finish_operation(op.id, status, exit_code, summary.as_deref())
            .await?;
        let kind = match status {
            OperationStatus::Succeeded => "operation.completed",
            OperationStatus::Cancelled => "operation.cancelled",
            _ => "operation.failed",
        };
        self.emit(kind, Some(&op), json!({ "operation": op }));
        Ok(op)
    }

    /// Runs `vagrant status` for the whole environment after a mutating
    /// command (whatever its outcome) and records it. Returns a warning when
    /// the refresh fails or, after success, contradicts the expected state.
    async fn follow_up_status(
        &self,
        environment_id: EnvironmentId,
        op: &Operation,
        req: &OperationRequest,
        outcome: OperationStatus,
        log: &mut log::LogWriter,
        redactor: &mut redact::Redactor,
    ) -> Result<Option<String>, EngineError> {
        let status_command = VagrantCommand::Status { machine: None };
        log.meta(&format!(
            "refreshing machine state: {}",
            command_line(&status_command)
        ));
        let captured = self
            .invoke(
                op,
                Invocation {
                    program: req.program.clone(),
                    args: status_command.args(),
                    working_dir: req.working_dir.clone(),
                    timeout: Some(FOLLOW_UP_TIMEOUT),
                    env: req.env.clone(),
                },
                // Not tied to the operation's token: a cancelled operation
                // still needs its machines' state checked.
                CancellationToken::new(),
                log,
                redactor,
            )
            .await;
        let parsed = machine_readable::parse(&captured.stdout.join("\n"));
        let vagrant_error = machine_readable::error_exit(&parsed);
        let failure = match &captured.outcome {
            Err(e) => Some(format!("could not start Vagrant: {e}")),
            Ok(r) if r.termination == Termination::TimedOut => Some(format!(
                "timed out after {} seconds",
                FOLLOW_UP_TIMEOUT.as_secs()
            )),
            Ok(r) if r.exit_code == Some(0) => None,
            Ok(r) => Some(failure_summary(
                vagrant_error.as_ref().map(|e| e.message.as_str()),
                &captured.stderr_tail,
                &[],
                r.exit_code,
            )),
        };
        if captured.outcome.is_ok() {
            self.record_status(
                environment_id,
                op,
                &parsed,
                vagrant_error,
                failure.as_deref(),
            )
            .await?;
        }
        if let Some(failure) = failure {
            return Ok(Some(format!(
                "Checking machine state afterwards failed: {failure}"
            )));
        }
        let VagrantCommand::Lifecycle {
            action, machines, ..
        } = &req.command
        else {
            return Ok(None);
        };
        if outcome != OperationStatus::Succeeded {
            return Ok(None);
        }
        let observed: Vec<MachineStatusDto> = machine_readable::parse_status(&parsed)
            .into_iter()
            .map(Into::into)
            .collect();
        Ok(lifecycle::state_mismatch(*action, machines, &observed))
    }

    /// Runs one process, writing redacted output to the log and as events.
    async fn invoke(
        &self,
        op: &Operation,
        invocation: Invocation,
        token: CancellationToken,
        log: &mut log::LogWriter,
        redactor: &mut redact::Redactor,
    ) -> Captured {
        let mut stdout: Vec<String> = Vec::new();
        let mut stderr_tail: Vec<String> = Vec::new();
        let sink = self.inner.sink.clone();
        let outcome = self
            .inner
            .runner
            .run(
                invocation,
                Box::new(|line| {
                    let Some(text) = redactor.line(&line.text) else {
                        return;
                    };
                    log.line(line.stream, &text);
                    match line.stream {
                        Stream::Stdout if stdout.len() < MAX_CAPTURED_LINES => {
                            stdout.push(text.clone())
                        }
                        Stream::Stderr if !text.trim().is_empty() => {
                            stderr_tail.push(text.clone());
                            if stderr_tail.len() > 5 {
                                stderr_tail.remove(0);
                            }
                        }
                        _ => {}
                    }
                    let kind = match line.stream {
                        Stream::Stdout => "operation.stdout",
                        Stream::Stderr => "operation.stderr",
                    };
                    sink.emit(OperationEvent::new(kind, Some(op), json!({ "line": text })));
                }),
                token,
            )
            .await;
        Captured {
            outcome,
            stdout,
            stderr_tail,
        }
    }

    async fn record_status(
        &self,
        environment_id: EnvironmentId,
        op: &Operation,
        parsed: &[machine_readable::Line],
        error: Option<machine_readable::VagrantError>,
        summary: Option<&str>,
    ) -> Result<(), EngineError> {
        let machines: Vec<MachineStatusDto> = machine_readable::parse_status(parsed)
            .into_iter()
            .map(Into::into)
            .collect();
        let error = match error {
            Some(e) => Some(StatusError {
                class: Some(e.class),
                message: e.message,
            }),
            None => summary.map(|s| StatusError {
                class: None,
                message: s.to_owned(),
            }),
        };
        let status = EnvironmentStatus {
            environment_id,
            operation_id: Some(op.id),
            observed_at: vagrant_studio_project_store::now(),
            machines,
            error,
        };
        self.inner
            .store
            .save_environment_status(&EnvironmentStatusRecord {
                environment_id: environment_id.to_string(),
                operation_id: Some(op.id.to_string()),
                observed_at: status.observed_at,
                machines_json: serde_json::to_string(&status.machines).expect("serializable"),
                error_class: status.error.as_ref().and_then(|e| e.class.clone()),
                error_message: status.error.as_ref().map(|e| e.message.clone()),
            })
            .await?;
        self.emit(
            "environment.status_updated",
            Some(op),
            json!({ "status": status }),
        );
        Ok(())
    }

    fn emit(&self, kind: &'static str, op: Option<&Operation>, payload: serde_json::Value) {
        self.inner.sink.emit(OperationEvent::new(kind, op, payload));
    }
}

fn status_from_record(record: EnvironmentStatusRecord) -> Option<EnvironmentStatus> {
    Some(EnvironmentStatus {
        environment_id: record.environment_id.parse().ok()?,
        operation_id: record.operation_id.and_then(|s| s.parse().ok()),
        observed_at: record.observed_at,
        machines: serde_json::from_str(&record.machines_json).unwrap_or_default(),
        error: record.error_message.map(|message| StatusError {
            class: record.error_class,
            message,
        }),
    })
}

/// `vagrant status --machine-readable` style display string (not executed).
pub fn command_line(command: &VagrantCommand) -> String {
    std::iter::once("vagrant".to_owned())
        .chain(command.args())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Lines of output shown with a failure from a human-output command.
const OUTPUT_CONTEXT_LINES: usize = 6;

/// The last non-empty stdout lines of a command with human output. Vagrant
/// prints guest output (including a failing provisioner's errors) to stdout
/// and only its generic error ("The SSH command responded with a non-zero
/// exit status…") to stderr, so the cause is in these lines.
fn human_output_tail(command: &VagrantCommand, stdout: &[String]) -> Vec<String> {
    if command.args().iter().any(|a| a == "--machine-readable") {
        return Vec::new();
    }
    let mut tail: Vec<String> = stdout
        .iter()
        .rev()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .take(OUTPUT_CONTEXT_LINES)
        .map(str::to_owned)
        .collect();
    tail.reverse();
    tail
}

fn failure_summary(
    vagrant_error: Option<&str>,
    stderr_tail: &[String],
    output_tail: &[String],
    code: Option<i32>,
) -> String {
    let mut text = match vagrant_error {
        Some(message) => message.to_owned(),
        None if !stderr_tail.is_empty() => stderr_tail.join("\n"),
        None => match code {
            Some(c) => format!("Vagrant exited with code {c}."),
            None => "Vagrant was terminated.".to_owned(),
        },
    };
    if !output_tail.is_empty() {
        text = format!(
            "{text}\n\nLast output before the error:\n{}",
            output_tail.join("\n")
        );
    }
    text.chars().take(1500).collect()
}

/// The machines a command names, recorded so history can be shown per machine.
fn named_machines(command: &VagrantCommand) -> Vec<String> {
    match command {
        VagrantCommand::Status { machine } => {
            machine.iter().map(|m| m.as_str().to_owned()).collect()
        }
        VagrantCommand::Lifecycle { machines, .. } => {
            machines.iter().map(|m| m.as_str().to_owned()).collect()
        }
        _ => Vec::new(),
    }
}

/// Folder for operation logs inside the app data folder.
pub fn logs_dir(app_data: &Path) -> PathBuf {
    app_data.join("logs").join("operations")
}

#[cfg(test)]
mod tests;
