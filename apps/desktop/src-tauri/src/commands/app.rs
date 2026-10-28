use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use vagrant_studio_platform::{HostInfo, PlatformAdapter, SystemPlatform};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub app_version: String,
    pub tauri_version: &'static str,
    pub host: HostInfo,
    pub data_dir: Option<String>,
    pub vagrant_path: Option<String>,
}

/// Static application and host facts for the settings and readiness views.
#[tauri::command]
pub fn get_app_info<R: Runtime>(app: AppHandle<R>) -> AppInfo {
    let platform = SystemPlatform;
    AppInfo {
        app_version: app.package_info().version.to_string(),
        tauri_version: tauri::VERSION,
        host: platform.host_info(),
        data_dir: app
            .path()
            .app_data_dir()
            .ok()
            .map(|p| p.display().to_string()),
        vagrant_path: platform
            .find_executable("vagrant")
            .map(|p| p.display().to_string()),
    }
}
