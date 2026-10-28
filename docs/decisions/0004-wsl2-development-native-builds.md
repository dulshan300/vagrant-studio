# ADR-0004: Develop in WSL2; build and validate natively per platform

- Status: Accepted
- Date: 2026-10-28

## Context
Development happens in WSL2 Ubuntu on a Windows 11 host. Release targets are
Windows 11 x64, Ubuntu x64 and Fedora x64. A WSL2 Linux build says nothing
about the Windows build. Inside WSL, the Windows host and the WSL distro have
**separate** Vagrant installations, providers and VM inventories (e.g. Linux
`vagrant` with `VAGRANT_WSL_ENABLE_WINDOWS_ACCESS=1` driving Windows
VirtualBox is a WSL-only arrangement).

## Decision
- The repository lives on the WSL Linux filesystem (not `/mnt/c`).
- Day-to-day development: frontend, pure Rust crates and fixture/mocked tests
  in WSL2. The desktop crate builds in WSL once the WebKitGTK packages are
  installed (see README), and runs through WSLg.
- CI has three native build jobs, each running checks, tests and packaging:
  - **Windows** — `windows-latest` runner → NSIS installer (`.exe`).
  - **Ubuntu** — `ubuntu-24.04` runner → `.deb` and AppImage.
  - **Fedora** — `fedora` container on an Ubuntu runner → `.rpm`. Because
    this is a container, CI does not prove the app launches on a Fedora
    desktop; release validation includes a manual smoke test on a real Fedora
    install (or VM).
- The shipped app runs the host OS's own Vagrant. Windows builds never call
  into WSL, and Linux builds never call Windows executables or `/mnt/c` paths.
- Tests that need a real VM are separated (`#[ignore]` + an explicit opt-in
  environment variable) from tests that run anywhere.

## Consequences
- "Verified on X" claims require the relevant CI job or a recorded manual
  test, never a WSL build.
- Supported versions for the first release: Windows 11 x64 (23H2+),
  Ubuntu 24.04 LTS x64, the current Fedora release x64 (CI pins the image tag).
- ARM64 is out of scope initially.
