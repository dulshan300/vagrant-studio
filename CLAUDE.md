# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Vagrant Studio: a Tauri 2 desktop app (Rust backend + Vue 3/TypeScript frontend) for managing Vagrant projects and multi-machine environments on Windows 11, Ubuntu and Fedora (x64). It is built in phases. Read `docs/implementation-status.md` (current phase, blockers, next task) and `docs/implementation-roadmap.md` before starting work, and update both when a phase task finishes.

## Commands

pnpm is pinned via `packageManager` and run through Corepack. Use `corepack pnpm …` if `pnpm` isn't on PATH, and set `COREPACK_ENABLE_DOWNLOAD_PROMPT=0` in non-interactive shells, or it hangs on a download prompt. The root scripts call `pnpm -r`, so without `pnpm` on PATH put a shim first (a script running `exec corepack pnpm "$@"`).

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
VAGRANT_STUDIO_TEST_PROVIDER=vmware_desktop WSLENV=VAGRANT_STUDIO_TEST_PROVIDER \
  cargo test -p vagrant-studio-vagrant-adapter --target x86_64-pc-windows-gnu -- --include-ignored   # real Windows Vagrant
cargo test -p vagrant-studio-operation-engine --test docker_lifecycle -- --ignored   # real lifecycle via Vagrant's docker provider (Linux)
cargo test -p vagrant-studio-provider-registry --target x86_64-pc-windows-gnu --test real_host -- --ignored --nocapture   # print real provider detection on Windows
pnpm build:win-local   # Windows .exe from WSL → target/x86_64-pc-windows-gnu/release/vagrant-studio.exe (runs via interop)
```

A plain `cargo build` of the app crate produces a *dev* binary that loads `devUrl` (localhost:1420). Standalone builds need `--features tauri/custom-protocol`, which `tauri build` and `build:win-local` add.

CI currently builds **Windows only** (MSVC + NSIS); the Ubuntu and Fedora jobs are commented out in `.github/workflows/ci.yml` until Linux work resumes. Path and process code must also pass on Windows, so run the windows-gnu test command above before pushing.

The `vagrant-studio` crate (apps/desktop/src-tauri) needs WebKitGTK 4.1 dev packages on Linux. Without them, check and test the other crates with `--workspace --exclude vagrant-studio`.

## Architecture (see docs/architecture.md)

- **Modular monolith.** The Cargo workspace root is the repo root; members are `apps/desktop/src-tauri` and `crates/*`. The app crate holds only Tauri commands (`src-tauri/src/commands/`, errors mapped to `CommandError { kind, message, field, existing }`), wiring and the event bridge. Current crates: `platform` (host/WSL detection, executable discovery), `domain` (types, validation, portable format; no I/O), `project-store` (SQLite via SQLx, the `Store` type), `project-service` (`ProjectService`, the only code that writes to project folders), `vagrant-adapter` (`Runner`/`ProcessRunner`, `VagrantCommand`, machine-readable parsers, Vagrant/plugin readiness), `operation-engine` (`OperationEngine`: operation records, redacted logs, events, cancellation, environment locks), `config-model` (Vagrantfile subset: recognizer, validation, generator; no I/O), `provider-registry` (`Registry` of `ProviderAdapter`s: VirtualBox, VMware Desktop, Docker; capability flags, support level, prerequisite findings; host facts behind `HostProbe`), `git` (read-only `git status` of project folders through the adapter's `Runner`). New crates are created only when their phase starts.
- **Running Vagrant:** always through `OperationEngine` (user-visible commands) or the adapter's `Runner` (readiness probes); never `std::process::Command` directly. Don't combine `--no-tty` with `--machine-readable`; the last flag wins and you get human text (ADR-0008). Events go to the WebView on `vagrant-studio://operation`. Test parsers against `fixtures/command-output/` (CRLF preserved; never normalize them).
- **Persistence:** migrations live in `/migrations` (embedded with `sqlx::migrate!`; never edit an applied migration, add a new one). SQLx 0.9 accepts only `&'static str` SQL; build variants with `concat!` macros, bind all values, and don't use `AssertSqlSafe`. Timestamps are RFC 3339 UTC with variable fraction digits, so compare parsed values, never strings (Rust or TS). Tests use `Store::open_in_memory()`, not mocks (ADR-0007).
- **Vagrant is the lifecycle authority.** Only invoke the host's `vagrant` CLI, with an argv vector (never a shell string) and an explicit working directory set to the environment root. Machine state shown in the UI comes from `vagrant status`, never from the DB or guesses. Don't invent metrics; show "unknown/unavailable".
- **Host Vagrant only** (ADR-0002): never bundle or install Vagrant, providers or plugins, and never change host virtualization or network settings.
- **WSL caveat** (ADR-0004): development runs in WSL2, but releases must use the native OS's Vagrant. `crates/platform` skips `/mnt/<drive>` PATH entries under WSL so Linux builds never pick up Windows executables. A WSL build proves nothing about Windows.
- **Frontend:** `src/services/` is the only place that calls `invoke()` and guards with `isTauri()`. Pinia stores are never the record of truth. Routes and the sidebar share `src/app/navigation.ts`. UI primitives in `src/components/ui/` follow shadcn-vue conventions on reka-ui. Use the semantic color tokens defined in `src/styles/main.css` (`bg-surface`, `text-muted`, `bg-status-*`), not raw palette colors. Dark mode is the `.dark` class on `<html>`.
- **TS types mirror Rust DTOs:** Rust structs use `#[serde(rename_all = "camelCase")]`, and `src/types/` must match them.
- **Tauri permissions are minimal** (`src-tauri/capabilities/default.json`). Don't add shell or fs plugin access for the WebView. External URLs must be allowlisted under `opener:allow-open-url`.

## Rules specific to this project

- Vagrantfile editing (docs/vagrantfile-editor.md): only `project-service` writes Vagrantfiles, through `save_vagrantfile` (revision check, backup in app data, atomic rename) while holding the engine's environment lock. Extending the visual subset means changing lexer/parser, generator and validation together. `generate` must read back to an identical model, and the `render_all` output must pass `vagrant validate`.

- Never overwrite user Vagrantfiles or project files without diff, backup and explicit confirmation (ADR-0005). Deleting app metadata must never delete files or destroy VMs. File deletion only goes to the OS trash, through `ProjectService::delete`'s safeguards (ADR-0007).
- Paths: canonicalize with `dunce`, reject non-UTF-8, and check environment folders stay inside the project root. Windows filesystems are case-insensitive and `DirEntry::metadata()` is stale on Windows; use `fs::metadata(path)` when accuracy matters.
- Tauri npm packages and Rust crates must match on major.minor (`tauri build` fails otherwise); `rust-version` in `Cargo.toml` limits which crate versions Cargo picks (ADR-0006).
- Don't claim a provider or platform works until it has been tested natively; record results in `docs/provider-capabilities.md` and `docs/implementation-status.md`. VirtualBox is the first provider path (ADR-0003).
- Templates live in `templates/*/Vagrantfile.tmpl` with `{{provider_name}}` and `{{provider target=… cpus=… memory=…}}` placeholders, rendered per provider in `crates/project-service/src/templates.rs`. After changing one, render all of them (`VAGRANT_STUDIO_RENDER_DIR=… cargo test -p vagrant-studio-project-service -- --ignored render_all`) and run `vagrant validate` on each with Windows Vagrant.
- TypeScript is held at 6.0.x because typescript-eslint requires `<6.1` (ADR-0006).
- Tests needing real Vagrant or a VM must be opt-in (`#[ignore]`), separate from tests that run anywhere. Shell scripts here run in zsh: unquoted `$var` is not word-split (use `${=var}`).
- Commit at verified milestones; end commit messages with the attribution trailer the session specifies.
