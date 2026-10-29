//! Read-only host facts that aren't process runs, behind a trait for tests.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::time::Duration;

pub trait HostProbe: Send + Sync {
    /// Whether a local TCP service accepts connections.
    fn is_open<'a>(&'a self, addr: &'a str) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;

    fn path_exists(&self, path: &Path) -> bool;

    /// Whether this OS runs on a hypervisor (CPUID "hypervisor present" bit).
    /// On Windows that is the case when Hyper-V, WSL 2, Windows Sandbox or
    /// virtualization-based security is enabled, or when Windows itself runs
    /// in a VM. `None` where it can't be read.
    fn hypervisor_present(&self) -> Option<bool>;
}

pub struct SystemProbe;

impl HostProbe for SystemProbe {
    fn is_open<'a>(&'a self, addr: &'a str) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            matches!(
                tokio::time::timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(addr))
                    .await,
                Ok(Ok(_))
            )
        })
    }

    fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn hypervisor_present(&self) -> Option<bool> {
        #[cfg(target_arch = "x86_64")]
        {
            Some(std::arch::x86_64::__cpuid(1).ecx & (1 << 31) != 0)
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            None
        }
    }
}
