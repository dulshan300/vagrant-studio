# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Vagrant Studio: a Tauri 2 desktop app (Rust backend + Vue 3/TypeScript frontend) for managing Vagrant projects and multi-machine environments on Windows 11, Ubuntu and Fedora (x64). It is built in phases. Read `docs/implementation-status.md` (current phase, blockers, next task) and `docs/implementation-roadmap.md` before starting work, and update both when a phase task finishes.

## Commands

pnpm is pinned via `packageManager` and run through Corepack. Use `corepack pnpm …` if `pnpm` isn't on PATH, and set `COREPACK_ENABLE_DOWNLOAD_PROMPT=0` in non-interactive shells, or it hangs on a download prompt.

```sh
pnpm install
pnpm dev                                   # desktop app (tauri dev → vite on :1420)
pnpm --filter @vagrant-studio/desktop dev  # frontend only in a browser; backend calls return null
pnpm lint && pnpm format:check && pnpm typecheck && pnpm test
pnpm --filter @vagrant-studio/desktop exec vitest run src/app/theme.spec.ts   # single test file

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p vagrant-studio-platform finds_first_match   # single crate / single test
cargo test -p vagrant-studio-platform --target x86_64-pc-windows-gnu   # run tests as a Windows .exe via WSL interop
```

CI currently builds **Windows only** (MSVC + NSIS); the Ubuntu and Fedora jobs are commented out in `.github/workflows/ci.yml` until Linux work resumes. Path and process code must also pass on Windows, so run the windows-gnu test command above before pushing.

The `vagrant-studio` crate (apps/desktop/src-tauri) needs WebKitGTK 4.1 dev packages on Linux. Without them, check and test the other crates with `--workspace --exclude vagrant-studio`.

## Architecture (see docs/architecture.md)

- **Modular monolith.** The Cargo workspace root is the repo root; members are `apps/desktop/src-tauri` and `crates/*`. The app crate holds only Tauri commands, wiring and the event bridge. Business logic goes in `crates/*` behind traits (`PlatformAdapter`, and later `VagrantExecutor`, repositories, `EventSink`) with fakes for tests. New crates (`domain`, `project-store`, `vagrant-adapter`, `operation-engine`, `provider-registry`, `config-model`) are created only when the phase that needs them starts.
- **Vagrant is the lifecycle authority.** Only invoke the host's `vagrant` CLI, with an argv vector (never a shell string) and an explicit working directory set to the environment root. Machine state shown in the UI comes from `vagrant status`, never from the DB or guesses. Don't invent metrics; show "unknown/unavailable".
- **Host Vagrant only** (ADR-0002): never bundle or install Vagrant, providers or plugins, and never change host virtualization or network settings.
- **WSL caveat** (ADR-0004): development runs in WSL2, but releases must use the native OS's Vagrant. `crates/platform` skips `/mnt/<drive>` PATH entries under WSL so Linux builds never pick up Windows executables. A WSL build proves nothing about Windows.
- **Frontend:** `src/services/` is the only place that calls `invoke()` and guards with `isTauri()`. Pinia stores are never the record of truth. Routes and the sidebar share `src/app/navigation.ts`. UI primitives in `src/components/ui/` follow shadcn-vue conventions on reka-ui. Use the semantic color tokens defined in `src/styles/main.css` (`bg-surface`, `text-muted`, `bg-status-*`), not raw palette colors. Dark mode is the `.dark` class on `<html>`.
- **TS types mirror Rust DTOs:** Rust structs use `#[serde(rename_all = "camelCase")]`, and `src/types/` must match them.
- **Tauri permissions are minimal** (`src-tauri/capabilities/default.json`). Don't add shell or fs plugin access for the WebView. External URLs must be allowlisted under `opener:allow-open-url`.

## Rules specific to this project

- Never overwrite user Vagrantfiles or project files without diff, backup and explicit confirmation (ADR-0005). Deleting app metadata must never delete files or destroy VMs.
- Don't claim a provider or platform works until it has been tested natively; record results in `docs/provider-capabilities.md` and `docs/implementation-status.md`. VirtualBox is the first provider path (ADR-0003).
- TypeScript is held at 6.0.x because typescript-eslint requires `<6.1` (ADR-0006).
- Tests needing a real VM must be opt-in (`#[ignore]` plus an env var), separate from tests that run anywhere.
- Commit at verified milestones; end commit messages with the attribution trailer the session specifies.
