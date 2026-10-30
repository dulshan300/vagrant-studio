use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use tauri::ipc::Channel;
use vagrant_studio_domain::EnvironmentId;
use vagrant_studio_operation_engine::OperationEngine;
use vagrant_studio_platform::{spawn_terminal, system_terminal_launch};
use vagrant_studio_project_service::ProjectService;
use vagrant_studio_terminal::{Size, SpawnRequest, TerminalError, TerminalSink, Terminals};
use vagrant_studio_vagrant_adapter::{MachineName, StateClass, ssh_args};

use super::error::{CommandError, CommandResult};
use super::operations::vagrant_program;

/// `vagrant ssh <machine>` for a machine the last `vagrant status` reports as
/// running: the program, its arguments and the environment folder.
struct SshTarget {
    program: PathBuf,
    args: Vec<OsString>,
    root: PathBuf,
    machine: MachineName,
}

async fn ssh_target(
    projects: &ProjectService,
    engine: &OperationEngine,
    environment_id: EnvironmentId,
    machine: String,
) -> CommandResult<SshTarget> {
    let machine = MachineName::try_from(machine).map_err(CommandError::validation)?;
    let (_, env) = projects.environment(environment_id).await?;
    if !Path::new(&env.root_path).is_dir() {
        return Err(CommandError::unavailable(format!(
            "The environment folder {} no longer exists.",
            env.root_path
        )));
    }
    // Checked against the last `vagrant status`; if that is stale, `vagrant ssh`
    // reports the problem in the terminal itself.
    let status = engine.environment_status(environment_id).await?;
    let reported = status
        .iter()
        .flat_map(|s| &s.machines)
        .find(|m| m.name == machine.as_str());
    match reported {
        None => {
            return Err(CommandError::validation(format!(
                "Vagrant doesn't report a machine named {}. Refresh the status and try again.",
                machine.as_str()
            )));
        }
        Some(m) if m.state.as_deref().map(StateClass::of) != Some(StateClass::Running) => {
            return Err(CommandError::validation(format!(
                "{} isn't running. Start it first; SSH needs a running machine.",
                machine.as_str()
            )));
        }
        Some(_) => {}
    }
    Ok(SshTarget {
        program: vagrant_program(projects).await?,
        args: ssh_args(&machine).into_iter().map(OsString::from).collect(),
        root: PathBuf::from(env.root_path),
        machine,
    })
}

/// Opens `vagrant ssh <machine>` in a new terminal window, in the environment
/// folder. The session belongs to the user: it isn't an operation, holds no
/// lock and isn't logged. Returns the name of the terminal that was opened.
#[tauri::command]
pub async fn open_ssh_terminal(
    projects: State<'_, ProjectService>,
    engine: State<'_, OperationEngine>,
    environment_id: EnvironmentId,
    machine: String,
) -> CommandResult<String> {
    let SshTarget {
        program,
        args,
        root,
        machine,
    } = ssh_target(&projects, &engine, environment_id, machine).await?;
    let title = format!("vagrant ssh {}", machine.as_str());
    let launch = system_terminal_launch(&title, &program, &args).ok_or_else(|| {
        CommandError::unavailable(
            "No terminal application was found. Install one, or run vagrant ssh from a terminal \
             in the environment folder.",
        )
    })?;
    spawn_terminal(&launch, &root).map_err(|e| {
        CommandError::unavailable(format!("{} couldn't be started: {e}", launch.terminal))
    })?;
    tracing::info!(
        terminal = launch.terminal,
        machine = machine.as_str(),
        "opened ssh terminal"
    );
    Ok(launch.terminal.to_owned())
}

/// Output of an embedded terminal session, sent over the session's channel.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TerminalEvent {
    Output {
        data: String,
    },
    /// `code` is `null` when the session was closed from the app.
    Exit {
        code: Option<u32>,
    },
}

struct ChannelSink(Channel<TerminalEvent>);

impl TerminalSink for ChannelSink {
    fn output(&self, data: String) {
        if let Err(e) = self.0.send(TerminalEvent::Output { data }) {
            tracing::debug!(error = %e, "terminal output not delivered");
        }
    }
    fn exited(&self, code: Option<u32>) {
        if let Err(e) = self.0.send(TerminalEvent::Exit { code }) {
            tracing::debug!(error = %e, "terminal exit not delivered");
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSession {
    pub id: String,
    pub title: String,
}

impl From<TerminalError> for CommandError {
    fn from(e: TerminalError) -> Self {
        match e {
            TerminalError::TooMany | TerminalError::Spawn(_) => {
                CommandError::unavailable(e.to_string())
            }
            TerminalError::NotFound => CommandError::validation(e.to_string()),
            TerminalError::Io(_) => CommandError::internal(e.to_string()),
        }
    }
}

/// Starts `vagrant ssh <machine>` in the app's terminal panel. Like the
/// external terminal, the session is the user's: not an operation, no lock,
/// nothing stored.
#[tauri::command]
pub async fn start_ssh_session(
    projects: State<'_, ProjectService>,
    engine: State<'_, OperationEngine>,
    terminals: State<'_, Terminals>,
    environment_id: EnvironmentId,
    machine: String,
    size: Size,
    on_event: Channel<TerminalEvent>,
) -> CommandResult<TerminalSession> {
    let target = ssh_target(&projects, &engine, environment_id, machine).await?;
    let mut env = vec![("VAGRANT_CHECKPOINT_DISABLE".to_owned(), "1".to_owned())];
    if !cfg!(windows) {
        // What xterm.js understands; `ssh` passes it on to the guest.
        env.push(("TERM".to_owned(), "xterm-256color".to_owned()));
        env.push(("COLORTERM".to_owned(), "truecolor".to_owned()));
    }
    let id = terminals.spawn(
        SpawnRequest {
            program: target.program,
            args: target.args,
            cwd: target.root,
            env,
            size,
        },
        Arc::new(ChannelSink(on_event)),
    )?;
    Ok(TerminalSession {
        id,
        title: format!("ssh {}", target.machine.as_str()),
    })
}

/// Keyboard input from the terminal panel.
#[tauri::command]
pub fn terminal_write(
    terminals: State<'_, Terminals>,
    id: String,
    data: String,
) -> CommandResult<()> {
    Ok(terminals.write(&id, data.as_bytes())?)
}

#[tauri::command]
pub fn terminal_resize(
    terminals: State<'_, Terminals>,
    id: String,
    size: Size,
) -> CommandResult<()> {
    Ok(terminals.resize(&id, size)?)
}

/// Ends the session's program. Closing an ended session is not an error.
#[tauri::command]
pub fn terminal_close(terminals: State<'_, Terminals>, id: String) {
    terminals.close(&id);
}
