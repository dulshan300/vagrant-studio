/** Mirrors `commands::app::AppInfo` in src-tauri (camelCase serde). */
export interface AppInfo {
  appVersion: string
  tauriVersion: string
  host: HostInfo
  dataDir: string | null
  /** Absolute path of the host's `vagrant` executable, or null when not found. */
  vagrantPath: string | null
}

/** Mirrors `vagrant_studio_platform::HostInfo`. */
export interface HostInfo {
  os: 'windows' | 'linux' | 'macos' | 'other'
  arch: 'x86_64' | 'aarch64' | 'other'
  isWsl: boolean
}
