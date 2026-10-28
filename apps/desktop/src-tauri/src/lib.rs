mod commands;

use tracing_subscriber::EnvFilter;

pub fn run() {
    init_tracing();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "starting Vagrant Studio"
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::app::get_app_info])
        .run(tauri::generate_context!())
        .expect("error while running Vagrant Studio");
}

/// Logs to stderr; `RUST_LOG` overrides the default filter.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,vagrant_studio=debug,vagrant_studio_lib=debug"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
