use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HostOs {
    Windows,
    Linux,
    Macos,
    Other,
}

impl HostOs {
    pub fn current() -> Self {
        match std::env::consts::OS {
            "windows" => Self::Windows,
            "linux" => Self::Linux,
            "macos" => Self::Macos,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    X86_64,
    Aarch64,
    Other,
}

impl Arch {
    pub fn current() -> Self {
        match std::env::consts::ARCH {
            "x86_64" => Self::X86_64,
            "aarch64" => Self::Aarch64,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInfo {
    pub os: HostOs,
    pub arch: Arch,
    /// True when running as a Linux process inside WSL. This is a development
    /// scenario only: Vagrant and providers inside WSL are separate from the
    /// Windows host's installation.
    pub is_wsl: bool,
}

impl HostInfo {
    pub fn detect() -> Self {
        let os = HostOs::current();
        let is_wsl = os == HostOs::Linux && detect_wsl();
        Self {
            os,
            arch: Arch::current(),
            is_wsl,
        }
    }
}

fn detect_wsl() -> bool {
    if std::env::var_os("WSL_DISTRO_NAME").is_some() {
        return true;
    }
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|release| is_wsl_kernel_release(&release))
        .unwrap_or(false)
}

/// WSL kernels report releases such as `5.15.167.4-microsoft-standard-WSL2`.
pub fn is_wsl_kernel_release(release: &str) -> bool {
    let release = release.to_ascii_lowercase();
    release.contains("microsoft") || release.contains("wsl")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_wsl_kernel_releases() {
        assert!(is_wsl_kernel_release("5.15.167.4-microsoft-standard-WSL2"));
        assert!(is_wsl_kernel_release("4.4.0-19041-Microsoft"));
        assert!(!is_wsl_kernel_release("6.11.4-301.fc41.x86_64"));
        assert!(!is_wsl_kernel_release("6.8.0-45-generic"));
    }

    #[test]
    fn detect_matches_compile_target() {
        let info = HostInfo::detect();
        assert_eq!(info.os, HostOs::current());
        if info.os != HostOs::Linux {
            assert!(!info.is_wsl);
        }
    }

    #[test]
    fn serializes_for_frontend() {
        let info = HostInfo {
            os: HostOs::Linux,
            arch: Arch::X86_64,
            is_wsl: true,
        };
        assert_eq!(
            serde_json::to_string(&info).unwrap(),
            r#"{"os":"linux","arch":"x86_64","isWsl":true}"#
        );
    }
}
