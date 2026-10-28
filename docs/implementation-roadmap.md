# Implementation Roadmap

Phases run in order. A phase is complete only when its acceptance criteria
have been checked, its tests pass (or failures are documented), its docs are
updated, and `docs/implementation-status.md` reflects it. Environmental
blockers (missing provider, missing system packages, no native host) are
recorded separately from test results.

## Phase 0 — Inspection and planning
- [x] Inspect repository and WSL2 toolchain
- [x] `docs/architecture.md` (including risks and unknowns)
- [x] `docs/provider-capabilities.md`
- [x] ADRs 0001–0006 (stack, host Vagrant, provider strategy, WSL2/native builds, editing safety, versions)
- [x] Supported OS versions and provider strategy defined
- [x] This roadmap and `docs/implementation-status.md`

## Phase 1 — Desktop foundation
Smallest sequence:
1. [x] Root workspace: `Cargo.toml` (workspace), `package.json`, `pnpm-workspace.yaml`, `.gitignore`, `.editorconfig`, `rustfmt.toml`
2. [x] `crates/platform`: host OS/arch/WSL detection and executable lookup (pure, unit-tested); the first real crate boundary
3. [x] `apps/desktop` frontend: Vite + Vue 3 + strict TS + Tailwind 4 + router + Pinia
4. [x] UI primitives (Button, Card, Badge, StatusBadge, segmented RadioGroup) and design tokens for light/dark
5. [x] App shell: sidebar with the 7 global sections, header, honest empty states per page
6. [x] Theme: light / dark / system, applied before first paint
7. [x] Settings page: theme, plus app and host info from the backend command `get_app_info`
8. [x] `src-tauri`: Tauri 2 app, tracing, minimal capabilities, `get_app_info` (compiles; launches under WSLg)
9. [x] ESLint (flat) + Prettier + vue-tsc + Vitest; rustfmt + clippy `-D warnings`
10. [x] CI: frontend checks, Rust checks, native build and package. Windows green; Ubuntu/Fedora jobs paused
11. [x] Verified launch on Windows 11 (2026-10-28). Ubuntu/Fedora deferred with Linux CI

Acceptance: app launches on target OSes; UI navigable; CI runs tests; no new warnings.

## Phase 2 — Persistence and project management (complete 2026-10-28, Windows)
- [x] `crates/domain`: Project, Environment, status/type enums, UUIDv7 IDs, validation (names, URLs without credentials, tags, slugs), portable format
- [x] `crates/project-store`: SQLx SQLite in the app data dir (WAL, FKs), `migrations/0001_initial.sql` (all spec §7 tables, FKs, indexes), project/environment/tag/settings persistence, migration tests
- [x] `crates/project-service`: create (empty or template), inspect (read-only), import, add/remove environments, update, favorite, archive/restore, delete (metadata; optional folder → OS trash with safeguards), portable metadata preview/write
- [x] App settings in SQLite (default project location); theme stays a per-device localStorage preference
- [x] Tauri commands with structured errors; folder picker via the dialog plugin (`dialog:allow-open` only)
- [x] Projects page (All/Favorites/Archived, search, tag filters, sort), project page (details editing, environments, archive, portable file, remove), dashboard counts and recent projects
- [x] Portable `.vagrant-studio/project.json` v1 (docs/project-format.md)
- [x] Templates `single-linux-vm`, `web-database`, `multi-tier-app` (validated by `vagrant validate`, Windows Vagrant 2.4.9)
- [x] Tests: repositories, migrations, restart persistence, import leaves files unchanged, metadata delete keeps files, deletion safeguards, portable round-trip; run on Linux and as Windows binaries
- [ ] Notes and tasks UI (tables exist; UI scheduled with the developer workspace in Phase 7)

## Phase 3 — Vagrant CLI integration
- [x] `crates/vagrant-adapter`: discovery (setting override → PATH → install locations), `--version`, typed `VagrantCommand` → argv, `ProcessRunner` (process-wrap; Job Object on Windows, process group on Linux; streaming; timeout; cancel; no console windows)
- [x] `--machine-readable` parsers for status, box list, plugin list, error-exit; fixtures captured from Windows Vagrant 2.4.9 (`fixtures/command-output/`)
- [x] `crates/operation-engine`: operation rows + log files, redaction, `EventSink` events, cancellation, crash recovery (`interrupted`), environment status snapshots
- [x] Migration 0002: history independent of environments, host-level operations, status cache; v1→v2 upgrade test
- [x] Host readiness: Vagrant missing/version, plugins, VirtualBox and VMware Desktop prerequisites (utility reachability); Providers page
- [x] Operations page, live output drawer with cancel; environment panel (Refresh status, Validate, machines); dashboard recent operations and machine counts; Virtual Machines page
- [x] Tests: fake runner, fixture parsers, real processes (cancel/timeout/argv), opt-in real Vagrant (`--ignored`), all also run as Windows binaries
- [x] Provider choice (user request): templates are provider-neutral (`Vagrantfile.tmpl`) and rendered for VirtualBox or VMware Desktop; New project picks from detected providers; app-wide default provider in Settings; per-environment provider editable; Add environment / Import set it too. All 3 templates × 2 providers pass `vagrant validate` (Windows Vagrant 2.4.9); `bento/ubuntu-24.04` is published for both providers.
- [ ] Manual check on Windows: user walkthrough

## Phase 4 — Multi-machine lifecycle
- [ ] Machine discovery from `vagrant status --machine-readable` per environment
- [ ] Per-machine and bulk up/halt/reload/suspend/resume/provision/destroy with verified targeting semantics
- [ ] Scheduler: per-environment write lock, global box lock, queueing and conflict errors
- [ ] Confirmation dialogs plus backend confirmation tokens for destructive actions
- [ ] Cancellation and unexpected termination handling; status refresh after every operation
- [ ] Integration tests with the docker provider (Linux, opt-in) and manual VirtualBox test plan

## Phase 5 — Vagrantfile editor
- [ ] Raw editor (CodeMirror 6, Ruby highlighting, search/replace)
- [ ] `crates/config-model`: supported subset, generator, recognizer, backup and atomic save
- [ ] Visual forms; diff preview; unsaved-change guard; read-only visual fallback
- [ ] Round-trip tests for generated files; `vagrant validate` as a check where available

## Phase 6 — Boxes, providers, networking, provisioning
- [ ] Box list/add/remove/update/inspect with in-use explanation
- [ ] `crates/provider-registry`: VirtualBox adapter, docker adapter, capability reporting, prerequisite findings
- [ ] Forwarded ports (+ host-port conflict detection), private/public networks, synced folders
- [ ] Shell provisioner management and results
- [ ] VirtualBox verification on Windows 11 and Ubuntu recorded in provider-capabilities.md

## Phase 7 — Developer workspace
- [ ] SSH: external terminal launch first, then an embedded PTY terminal (evaluate portable-pty + xterm.js)
- [ ] External editor integration, Git status, notes and tasks, project shortcuts, log navigation, cross-project port warnings

## Phase 8 — Additional providers and advanced capabilities
- [ ] Choose the next provider (libvirt on Fedora is the likely first); adapters, prerequisites, diagnostics, tests
- [ ] Capability-gated snapshots, optional metrics, export and recovery assistance

## Phase 9 — Release hardening
- [ ] Full test matrix on clean Windows/Ubuntu/Fedora; missing-Vagrant/provider cases; invalid Vagrantfiles; interrupted operations; paths with spaces and non-ASCII characters; DB upgrade tests
- [ ] Permission and process-execution review; secret-leak review of logs and diagnostics
- [ ] Installers (NSIS, deb, AppImage, rpm), install/troubleshooting docs, release checklist, known limitations
