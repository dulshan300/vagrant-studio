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

## Phase 3 — Vagrant CLI integration (complete 2026-10-28, Windows)
- [x] `crates/vagrant-adapter`: discovery (setting override → PATH → install locations), `--version`, typed `VagrantCommand` → argv, `ProcessRunner` (process-wrap; Job Object on Windows, process group on Linux; streaming; timeout; cancel; no console windows)
- [x] `--machine-readable` parsers for status, box list, plugin list, error-exit; fixtures captured from Windows Vagrant 2.4.9 (`fixtures/command-output/`)
- [x] `crates/operation-engine`: operation rows + log files, redaction, `EventSink` events, cancellation, crash recovery (`interrupted`), environment status snapshots
- [x] Migration 0002: history independent of environments, host-level operations, status cache; v1→v2 upgrade test
- [x] Host readiness: Vagrant missing/version, plugins, VirtualBox and VMware Desktop prerequisites (utility reachability); Providers page
- [x] Operations page, live output drawer with cancel; environment panel (Refresh status, Validate, machines); dashboard recent operations and machine counts; Virtual Machines page
- [x] Tests: fake runner, fixture parsers, real processes (cancel/timeout/argv), opt-in real Vagrant (`--ignored`), all also run as Windows binaries
- [x] Provider choice (user request): templates are provider-neutral (`Vagrantfile.tmpl`) and rendered for VirtualBox or VMware Desktop; New project picks from detected providers; app-wide default provider in Settings; per-environment provider editable; Add environment / Import set it too. All 3 templates × 2 providers pass `vagrant validate` (Windows Vagrant 2.4.9); `bento/ubuntu-24.04` is published for both providers.
- [x] Manual check on Windows: user walkthrough

## Phase 4 — Multi-machine lifecycle (complete 2026-10-28, Windows / VMware Desktop)
- [x] Machine discovery from `vagrant status --machine-readable` per environment (Phase 3 snapshot; lifecycle targets are checked against it)
- [x] Per-machine and bulk up/halt/reload/suspend/resume/provision/destroy: machines always named explicitly, in Vagrantfile order (bare `up` skips `autostart: false`); `up` without `--provider`
- [x] Scheduler: per-environment read/write lock, global box lock (read by `up`/`box list`; writers arrive in Phase 6), FIFO queueing, conflict errors for duplicate requests, cancel while queued
- [x] Typed confirmation dialog plus one-time backend confirmation tokens bound to the exact target for `destroy`
- [x] Status refresh after every mutating operation (inside the same operation and lock), expected-state warnings; cancellation and termination recorded, then state re-read
- [x] Integration test with the docker provider (Linux, opt-in, passing) and manual test plan (`docs/testing/manual-lifecycle.md`)
- [x] Manual lifecycle use on Windows with VMware Desktop (accepted by the user); VirtualBox lifecycle passed 2026-10-29 (Phase 6 verification)

## Phase 5 — Vagrantfile editor (complete 2026-10-29, Windows)
- [x] Raw editor (CodeMirror 6, Ruby highlighting, search/replace)
- [x] `crates/config-model`: supported subset, recognizer, validation, generator with checksum header (no I/O). Backup and atomic save live in `project-service` (the only writer of project files), with a revision check and the engine's environment lock
- [x] Visual forms (shared settings + machines: box, hostname, CPU/memory per provider, private networks, forwarded ports, synced folders, shell provisioners); diff preview for every save; unsaved-change guard (route and window close); explanation with line link when visual editing isn't possible; backups dialog; create a missing Vagrantfile
- [x] Round-trip tests for generated files; all templates × providers, their visual regenerations and a full-feature sample pass `vagrant validate` (Windows Vagrant 2.4.9); "Validate after saving" option
- [x] Manual walkthrough on Windows (`docs/testing/manual-vagrantfile-editor.md`): passed, user, 2026-10-29

## Phase 6 — Boxes, providers, networking, provisioning
- [x] Box list/add/remove/update/update check with in-use explanation (machine index), box write lock, typed confirmation + token for remove; manual plan `docs/testing/manual-boxes.md` (Windows walkthrough accepted by the user 2026-10-29)
- [x] `crates/provider-registry`: VirtualBox, VMware Desktop and docker adapters, capability flags and support level, prerequisite findings (VirtualBox/Vagrant version table, Windows hypervisor, `vboxdrv`, Docker daemon); Suspend/Resume hidden for providers without `lifecycle.suspend`. Automated tests pass on Linux and as Windows binaries; real-host detection checked on Windows. Providers page accepted by the user on Windows 2026-10-29
- [x] Forwarded ports (+ host-port conflict detection), private/public networks, synced folders: public networks (with VirtualBox adapter suggestions and a warning when none is set) added to the visual subset; host ports checked like Vagrant does; forwarded ports, private networks and synced folders were done in Phase 5. Walkthrough `docs/testing/manual-network.md` passed on Windows (user, 2026-10-29)
- [x] Shell provisioner management and results: named provisioners in the visual subset, Provision dialog (all or chosen names; Start/Restart with `--provision`), results per machine from the command output (migration 0003). Walkthrough `docs/testing/manual-provisioning.md` passed on Windows (user, 2026-10-29)
- [x] VirtualBox verification on Windows 11 recorded in provider-capabilities.md (smoke test and `docs/testing/manual-virtualbox.md` passed, user, 2026-10-29; listed in `VERIFIED`)
- Ubuntu verification of VirtualBox moved to the Phase 9 test matrix (user, 2026-10-29)

## Phase 7 — Developer workspace
- [ ] SSH: external terminal launch first, then an embedded PTY terminal (evaluate portable-pty + xterm.js). External terminal done in code and automated tests (Windows Terminal, console fallback, Linux terminals untested); SSH confirmed working on Windows by the user 2026-10-29; embedded terminal not started
- [ ] External editor integration, Git status, notes and tasks, project shortcuts, log navigation, cross-project port warnings. Done: open in editor (confirmed on Windows by the user 2026-10-30). Done: notes and tasks (confirmed on Windows by the user 2026-10-30)

## Phase 8 — Additional providers and advanced capabilities
- [ ] Choose the next provider (libvirt on Fedora is the likely first); adapters, prerequisites, diagnostics, tests
- [ ] Capability-gated snapshots, optional metrics, export and recovery assistance

## Phase 9 — Release hardening
- [ ] Full test matrix on clean Windows/Ubuntu/Fedora (including VirtualBox verification on Ubuntu); missing-Vagrant/provider cases; invalid Vagrantfiles; interrupted operations; paths with spaces and non-ASCII characters; DB upgrade tests
- [ ] Permission and process-execution review; secret-leak review of logs and diagnostics
- [ ] Installers (NSIS, deb, AppImage, rpm), install/troubleshooting docs, release checklist, known limitations
