use std::ffi::OsString;
use std::path::Path;

use tauri::State;
use vagrant_studio_domain::EnvironmentId;
use vagrant_studio_operation_engine::OperationEngine;
use vagrant_studio_platform::{spawn_terminal, system_terminal_launch};
use vagrant_studio_project_service::ProjectService;
use vagrant_studio_vagrant_adapter::{MachineName, StateClass, ssh_args};

use super::error::{CommandError, CommandResult};
use super::operations::vagrant_program;

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

    let program = vagrant_program(&projects).await?;
    let args: Vec<OsString> = ssh_args(&machine).into_iter().map(OsString::from).collect();
    let title = format!("vagrant ssh {}", machine.as_str());
    let launch = system_terminal_launch(&title, &program, &args).ok_or_else(|| {
        CommandError::unavailable(
            "No terminal application was found. Install one, or run vagrant ssh from a terminal \
             in the environment folder.",
        )
    })?;
    spawn_terminal(&launch, Path::new(&env.root_path)).map_err(|e| {
        CommandError::unavailable(format!("{} couldn't be started: {e}", launch.terminal))
    })?;
    tracing::info!(
        terminal = launch.terminal,
        machine = machine.as_str(),
        "opened ssh terminal"
    );
    Ok(launch.terminal.to_owned())
}
