# Vagrant Studio

A desktop application for managing Vagrant projects and multi-machine
environments on Windows and Linux. Vagrant stays in charge of the VM
lifecycle; Vagrant Studio adds project organisation, configuration editing,
operation history and a visual interface on top of it.

> **Status:** early development. Phase 1 (desktop foundation) is in progress. See
> [docs/implementation-status.md](docs/implementation-status.md) and the
> [roadmap](docs/implementation-roadmap.md).

## Using the app

Vagrant Studio uses the **Vagrant installed on your computer**. It does not
bundle or install Vagrant, providers or plugins, and it never changes host
virtualization or network settings. Install these yourself first:

- [Vagrant](https://developer.hashicorp.com/vagrant/install) 2.4 or later
- A provider. VirtualBox 7.1 is the first provider path being validated (see
  [provider capabilities](docs/provider-capabilities.md)).

Planned release targets: Windows 11 x64, Ubuntu 24.04 LTS x64, and the
current Fedora release x64. None of these is verified yet.

## Development

Development happens primarily in WSL2 Ubuntu (see
[ADR-0004](docs/decisions/0004-wsl2-development-native-builds.md)). Keep the
clone on the Linux filesystem (`~/…`), not under `/mnt/c`.

### Prerequisites

- Rust stable (edition 2024) with `rustfmt` and `clippy`
- Node.js ≥ 22.12 with Corepack (pnpm is pinned in `package.json`; no global install needed)
- Tauri system packages for your OS:

  | OS | Command |
  |---|---|
  | Ubuntu / WSL2 Ubuntu | `sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev libxdo-dev libssl-dev build-essential file` |
  | Fedora | `sudo dnf install webkit2gtk4.1-devel openssl-devel libappindicator-gtk3-devel librsvg2-devel libxdo-devel gcc gcc-c++ make file` |
  | Windows | Microsoft C++ Build Tools and WebView2 (preinstalled on Windows 11) |

### Commands

```sh
corepack enable pnpm            # puts the pinned pnpm on PATH (root scripts call `pnpm`)
pnpm install

pnpm dev                        # run the desktop app (Tauri + Vite, hot reload)
pnpm --filter @vagrant-studio/desktop dev   # frontend only in a browser (no backend)
pnpm build                      # release build + installers for the current OS

pnpm lint && pnpm format:check && pnpm typecheck && pnpm test   # frontend checks
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p vagrant-studio-platform        # one crate (builds without WebKitGTK)
```

In WSL2 the desktop window opens through WSLg. `libEGL ... /dev/dri ... Permission
denied` warnings are harmless (software rendering); if the window stays blank, run with
`WEBKIT_DISABLE_DMABUF_RENDERER=1 pnpm dev`. A WSL build or launch says
nothing about the native Windows build; that is validated in CI on a
Windows runner.

### Layout

```
apps/desktop/            Vue 3 frontend (src/) and Tauri app crate (src-tauri/)
crates/platform/         host detection, executable discovery (more crates arrive per phase)
docs/                    architecture, roadmap, status, provider capabilities, ADRs
.github/workflows/ci.yml Windows, Ubuntu and Fedora (container) builds
```
