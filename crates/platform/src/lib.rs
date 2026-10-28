//! Host platform detection and executable discovery.
//!
//! Everything that differs between Windows and Linux hosts lives behind
//! [`PlatformAdapter`] so the rest of the backend can stay platform-neutral and
//! be tested with fakes.

mod discovery;
mod host;

pub use discovery::{ExecutableSearch, find_executable};
pub use host::{Arch, HostInfo, HostOs, is_wsl_kernel_release};

use std::path::PathBuf;

/// Platform operations the rest of the backend depends on.
pub trait PlatformAdapter: Send + Sync {
    /// Static facts about the host the application runs on.
    fn host_info(&self) -> HostInfo;

    /// Locates an executable by bare name (e.g. `"vagrant"`), honouring the
    /// host's search rules. Returns `None` when it is not installed.
    fn find_executable(&self, name: &str) -> Option<PathBuf>;
}

/// [`PlatformAdapter`] backed by the real process environment.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemPlatform;

impl PlatformAdapter for SystemPlatform {
    fn host_info(&self) -> HostInfo {
        HostInfo::detect()
    }

    fn find_executable(&self, name: &str) -> Option<PathBuf> {
        let host = self.host_info();
        find_executable(name, &ExecutableSearch::from_env(&host))
    }
}
