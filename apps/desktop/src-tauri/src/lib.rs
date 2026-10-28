mod commands;

use tauri::Manager;
use tracing_subscriber::EnvFilter;
use vagrant_studio_project_service::ProjectService;
use vagrant_studio_project_store::Store;

const DATABASE_FILE: &str = "vagrant-studio.db";

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
            app.manage(ProjectService::new(store));
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
            commands::projects::remove_environment,
            commands::projects::preview_project_metadata,
            commands::projects::write_project_metadata,
            commands::projects::list_templates,
            commands::projects::get_settings,
            commands::projects::save_settings,
            commands::projects::open_project_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vagrant Studio");
}

/// Logs to stderr; `RUST_LOG` overrides the default filter.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,vagrant_studio=debug,vagrant_studio_lib=debug"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
