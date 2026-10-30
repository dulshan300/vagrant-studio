mod commands;

use std::sync::Arc;

use tauri::Manager;
use tracing_subscriber::EnvFilter;
use vagrant_studio_operation_engine::{OperationEngine, logs_dir};
use vagrant_studio_project_service::ProjectService;
use vagrant_studio_project_store::Store;
use vagrant_studio_terminal::Terminals;
use vagrant_studio_vagrant_adapter::ProcessRunner;

const DATABASE_FILE: &str = "vagrant-studio.db";
/// Vagrantfile backups, one folder per environment id.
const BACKUPS_DIR: &str = "backups";

pub fn run() {
    init_tracing();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "starting Vagrant Studio"
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join(DATABASE_FILE);
            tracing::info!(path = %db_path.display(), "opening database");
            let store = tauri::async_runtime::block_on(Store::open(&db_path))?;
            let engine = OperationEngine::new(
                store.clone(),
                Arc::new(ProcessRunner),
                Arc::new(commands::operations::TauriSink(app.handle().clone())),
                logs_dir(&data_dir),
            );
            tauri::async_runtime::block_on(engine.recover())?;
            app.manage(ProjectService::new(store).with_backups_dir(data_dir.join(BACKUPS_DIR)));
            app.manage(engine);
            app.manage(Terminals::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::projects::list_projects,
            commands::projects::list_project_tags,
            commands::projects::get_project,
            commands::projects::create_project,
            commands::projects::inspect_project_folder,
            commands::projects::import_project,
            commands::projects::update_project,
            commands::projects::set_project_favorite,
            commands::projects::set_project_archived,
            commands::projects::mark_project_opened,
            commands::projects::delete_project,
            commands::projects::add_environment,
            commands::projects::set_environment_provider,
            commands::projects::remove_environment,
            commands::projects::preview_project_metadata,
            commands::projects::write_project_metadata,
            commands::projects::list_templates,
            commands::projects::get_settings,
            commands::projects::save_settings,
            commands::projects::open_project_folder,
            commands::vagrantfile::get_vagrantfile,
            commands::vagrantfile::analyze_vagrantfile,
            commands::vagrantfile::generate_vagrantfile,
            commands::vagrantfile::save_vagrantfile,
            commands::vagrantfile::list_vagrantfile_backups,
            commands::vagrantfile::read_vagrantfile_backup,
            commands::boxes::list_boxes,
            commands::boxes::check_box_updates,
            commands::boxes::add_box,
            commands::boxes::update_box,
            commands::boxes::request_box_removal,
            commands::boxes::remove_box,
            commands::operations::get_host_readiness,
            commands::network::check_host_ports,
            commands::network::list_forwarded_ports,
            commands::network::list_bridged_interfaces,
            commands::operations::refresh_environment_status,
            commands::operations::validate_environment,
            commands::operations::run_lifecycle,
            commands::operations::request_lifecycle_confirmation,
            commands::operations::get_environment_status,
            commands::operations::list_environment_statuses,
            commands::operations::list_operations,
            commands::terminal::start_ssh_session,
            commands::terminal::terminal_write,
            commands::terminal::terminal_resize,
            commands::terminal::terminal_close,
            commands::operations::reveal_operation_log,
            commands::operations::open_operation_logs_folder,
            commands::operations::get_operation,
            commands::operations::read_operation_log,
            commands::operations::cancel_operation,
            commands::provisioning::list_provisioners,
            commands::provisioning::get_operation_provisioning,
            commands::provisioning::get_latest_provisioning,
            commands::terminal::open_ssh_terminal,
            commands::editor::list_editors,
            commands::editor::open_in_editor,
            commands::git::get_git_status,
            commands::git::is_git_installed,
            commands::git::init_git_repository,
            commands::git::get_git_log,
            commands::git::git_stage,
            commands::git::git_unstage,
            commands::git::git_commit,
            commands::workspace::list_project_notes,
            commands::workspace::create_project_note,
            commands::workspace::update_project_note,
            commands::workspace::delete_project_note,
            commands::workspace::list_project_tasks,
            commands::workspace::create_project_task,
            commands::workspace::update_project_task,
            commands::workspace::delete_project_task,
            commands::workspace::reorder_project_tasks,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Vagrant Studio")
        .run(|app, event| {
            // Terminal sessions end with the app; ConPTY would otherwise
            // leave `vagrant ssh` running without a window.
            if let tauri::RunEvent::Exit = event {
                app.state::<Terminals>().close_all();
            }
        });
}

/// Logs to stderr; `RUST_LOG` overrides the default filter.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,vagrant_studio=debug,vagrant_studio_lib=debug"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
