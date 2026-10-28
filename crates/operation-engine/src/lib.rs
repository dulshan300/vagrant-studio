//! Runs Vagrant commands as recorded operations.
//!
//! Every operation gets a database row (status, exit code, summary) and a
//! log file. Output is redacted, written to the log and streamed as events.
//! The final state is persisted by the backend, so history never depends on
//! the frontend. Operations left running by a crash are marked
//! `interrupted` on the next start ([`OperationEngine::recover`]).

pub mod events;
mod log;
pub mod redact;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use tokio_util::sync::CancellationToken;
use vagrant_studio_domain::{
    EnvironmentId, NewOperation, Operation, OperationId, OperationStatus, OperationType, ProjectId,
};
use vagrant_studio_project_store::{EnvironmentStatusRecord, Store, StoreError};
use vagrant_studio_vagrant_adapter::machine_readable::{self, MachineStatus};
use vagrant_studio_vagrant_adapter::{Invocation, Runner, Stream, Termination, VagrantCommand};

pub use events::{EventSink, OperationEvent};
pub use log::{LogLine, read_log};

/// Lines of stdout kept in memory for parsing (status output is small).
const MAX_CAPTURED_LINES: usize = 20_000;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("could not access the log {path}: {source}")]
    Logs {
        path: String,
        source: std::io::Error,
    },
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

#[derive(Clone)]
pub struct OperationEngine {
    inner: Arc<Inner>,
}

struct Inner {
    store: Store,
    runner: Arc<dyn Runner>,
    sink: Arc<dyn EventSink>,
    logs_dir: PathBuf,
    running: Mutex<HashMap<OperationId, CancellationToken>>,
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
                running: Mutex::new(HashMap::new()),
            }),
        }
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

    /// Records the operation and runs it in the background. Returns the queued record.
    pub async fn start(&self, req: OperationRequest) -> Result<Operation, EngineError> {
        let (op, token) = self.prepare(&req).await?;
        let engine = self.clone();
        let queued = op.clone();
        tokio::spawn(async move {
            if let Err(e) = engine.execute(op, req, token).await {
                tracing::error!(error = %e, "operation bookkeeping failed");
            }
        });
        Ok(queued)
    }

    /// Like [`start`](Self::start) but waits for the final record.
    pub async fn run(&self, req: OperationRequest) -> Result<Operation, EngineError> {
        let (op, token) = self.prepare(&req).await?;
        self.execute(op, req, token).await
    }

    /// Requests cancellation. Returns false if the operation isn't running here.
    pub async fn cancel(&self, id: OperationId) -> Result<bool, EngineError> {
        let token = self.inner.running.lock().expect("lock").get(&id).cloned();
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

    /// Newest first, optionally for one environment.
    pub async fn list(
        &self,
        environment_id: Option<EnvironmentId>,
        limit: i64,
    ) -> Result<Vec<Operation>, EngineError> {
        Ok(self
            .inner
            .store
            .list_operations(environment_id, limit.clamp(1, 500))
            .await?)
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

    pub fn is_running(&self, id: OperationId) -> bool {
        self.inner.running.lock().expect("lock").contains_key(&id)
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

    async fn prepare(
        &self,
        req: &OperationRequest,
    ) -> Result<(Operation, CancellationToken), EngineError> {
        std::fs::create_dir_all(&self.inner.logs_dir).map_err(|source| EngineError::Logs {
            path: self.inner.logs_dir.display().to_string(),
            source,
        })?;
        let log_template = self.inner.logs_dir.join("{id}.log");
        let op = self
            .inner
            .store
            .insert_operation(
                &NewOperation {
                    project_id: req.project_id,
                    environment_id: req.environment_id,
                    machine_name: req.machine_name.clone(),
                    target_label: req.target_label.clone(),
                    operation_type: req.operation_type.clone(),
                    command_line: command_line(&req.command),
                    working_dir: Some(req.working_dir.display().to_string()),
                },
                &log_template.display().to_string(),
            )
            .await?;
        let token = CancellationToken::new();
        self.inner
            .running
            .lock()
            .expect("lock")
            .insert(op.id, token.clone());
        self.emit("operation.queued", Some(&op), json!({ "operation": op }));
        Ok((op, token))
    }

    async fn execute(
        &self,
        op: Operation,
        req: OperationRequest,
        token: CancellationToken,
    ) -> Result<Operation, EngineError> {
        let result = self.execute_inner(&op, &req, token).await;
        self.inner.running.lock().expect("lock").remove(&op.id);
        result
    }

    async fn execute_inner(
        &self,
        op: &Operation,
        req: &OperationRequest,
        token: CancellationToken,
    ) -> Result<Operation, EngineError> {
        let op = self.inner.store.mark_operation_running(op.id).await?;
        self.emit("operation.started", Some(&op), json!({ "operation": op }));

        let log_path = PathBuf::from(op.log_path.clone().unwrap_or_default());
        let mut log = log::LogWriter::create(&log_path, &op);
        let mut redactor = redact::Redactor::default();
        let mut stdout_lines: Vec<String> = Vec::new();
        let mut stderr_tail: Vec<String> = Vec::new();

        let sink = self.inner.sink.clone();
        let event_op = op.clone();
        let outcome = self
            .inner
            .runner
            .run(
                Invocation {
                    program: req.program.clone(),
                    args: req.command.args(),
                    working_dir: req.working_dir.clone(),
                    timeout: req.timeout,
                    env: req.env.clone(),
                },
                Box::new(|line| {
                    let Some(text) = redactor.line(&line.text) else {
                        return;
                    };
                    log.line(line.stream, &text);
                    match line.stream {
                        Stream::Stdout if stdout_lines.len() < MAX_CAPTURED_LINES => {
                            stdout_lines.push(text.clone())
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
                    sink.emit(OperationEvent::new(
                        kind,
                        Some(&event_op),
                        json!({ "line": text }),
                    ));
                }),
                token,
            )
            .await;

        let parsed = machine_readable::parse(&stdout_lines.join("\n"));
        let vagrant_error = machine_readable::error_exit(&parsed);
        let (status, exit_code, summary) = match &outcome {
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
                        "Cancelled. The machine may have been left part-way; refresh its status before continuing.".to_owned()
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
                        &stderr_tail,
                        r.exit_code,
                    )),
                ),
            },
        };
        log.footer(status, exit_code, summary.as_deref());
        drop(log);

        if req.operation_type == OperationType::Status
            && let Some(env) = req.environment_id
            && matches!(status, OperationStatus::Succeeded | OperationStatus::Failed)
        {
            self.record_status(env, &op, &parsed, vagrant_error, summary.as_deref())
                .await?;
        }

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

fn failure_summary(
    vagrant_error: Option<&str>,
    stderr_tail: &[String],
    code: Option<i32>,
) -> String {
    let text = match vagrant_error {
        Some(message) => message.to_owned(),
        None if !stderr_tail.is_empty() => stderr_tail.join("\n"),
        None => match code {
            Some(c) => format!("Vagrant exited with code {c}."),
            None => "Vagrant was terminated.".to_owned(),
        },
    };
    text.chars().take(1000).collect()
}

/// Folder for operation logs inside the app data folder.
pub fn logs_dir(app_data: &Path) -> PathBuf {
    app_data.join("logs").join("operations")
}

#[cfg(test)]
mod tests;
