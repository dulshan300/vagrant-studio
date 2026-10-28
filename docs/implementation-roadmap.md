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

## Phase 2 — Persistence and project management
- [ ] `crates/domain`: Project, Environment, MachineConfig, Operation, tags, notes, tasks; ID newtypes; validation
- [ ] `crates/project-store`: SQLx SQLite pool in the app data dir, `migrations/` (all tables from spec §7 with FKs and indexes), repositories, migration tests
- [ ] App settings move to SQLite (theme stays in localStorage as a per-device UI preference)
- [ ] Commands + services + stores for projects/environments
- [ ] Projects list (search, sort, filter, tags, favorites, archive/restore), dashboard counts, recents
- [ ] Create / import (read-only import, no file changes) / edit / rename / delete metadata (files untouched by default; separate confirmed file deletion)
- [ ] Portable `.vagrant-studio/project.json` (schemaVersion 1, validation, no overwrite without confirmation)
- [ ] Templates (`templates/single-linux-vm`, `web-database`, `multi-tier-app`)
- [ ] Tests: repositories, migrations, restart persistence, import does not modify files

## Phase 3 — Vagrant CLI integration
- [ ] `crates/vagrant-adapter`: discovery (via `platform`), `--version`, typed `VagrantCommand` → argv, tokio process runner with streaming, timeouts, cancellation, process-tree kill
- [ ] `--machine-readable` parsers for status, box list, plugin list; fixtures under `fixtures/command-output/`
- [ ] `crates/operation-engine`: operation rows, log files, redaction, `EventSink`, crash recovery (`running` → `interrupted`)
- [ ] Host readiness: Vagrant missing/version/plugins
- [ ] Operations center page and live output drawer
- [ ] Tests: fake executor, fixture parsers, real `vagrant --version` (opt-in)

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
