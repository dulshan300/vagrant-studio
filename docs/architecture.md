# Vagrant Studio — Architecture

Vagrant Studio is a desktop application for managing Vagrant projects and
multi-machine environments. It is a **modular monolith**: one Tauri 2 process
with a Rust backend and a Vue 3 frontend. There are no extra services, no
account and no network dependency for core features.

Vagrant stays in charge of the machine lifecycle. Vagrant Studio manages
projects, edits configuration, runs the operations the user asks for, and
shows what happened. It never drives a hypervisor directly to do work that
Vagrant owns.

## 1. Runtime topology

```
┌──────────────────────── Vagrant Studio process ────────────────────────┐
│                                                                         │
│  WebView (Vue 3 + Pinia + Router)                                       │
│    pages ─► stores ─► services/ (typed invoke + event wrappers)         │
│                    ▲                         │                          │
│          Tauri events (operation.*,          │ Tauri commands           │
│          machine.status_updated, …)          ▼ (typed, capability-gated)│
│  Rust backend (apps/desktop/src-tauri)                                  │
│    commands/ ─► application services ─► crates/*                        │
│                                                                         │
│    crates/domain            entities, IDs, state machines (no I/O)      │
│    crates/project-store     SQLx/SQLite repositories + migrations       │
│    crates/project-service   project use cases; only writer of files     │
│    crates/vagrant-adapter   VagrantExecutor, arg builders, parsers      │
│    crates/operation-engine  OperationScheduler, conflict detection      │
│    crates/provider-registry capability model + provider detection       │
│    crates/config-model      Vagrantfile model, generator, safety checks │
│    crates/platform          PlatformAdapter (paths, exe discovery, OS)  │
└───────────────┬─────────────────────────────────────────────────────────┘
                │ argv-only process spawn, explicit cwd
                ▼
        host `vagrant` CLI  ─►  provider (VirtualBox, …)  ─►  VMs
```

## 2. Crate boundaries and dependency rules

| Crate | Responsibility | Depends on | Must not |
|---|---|---|---|
| `domain` | Entity types (`Project`, `Environment`, `MachineConfig`, `Operation`), ID newtypes, operation state machine, validation rules | serde, uuid, time | do I/O or know about Tauri |
| `platform` | `PlatformAdapter`: OS/arch detection, WSL detection, executable discovery (`PATH`, `PATHEXT`, well-known install dirs), path normalization, opening folders and terminals | std | depend on Tauri or the DB |
| `project-store` | Project, environment, tag and settings persistence (`Store`); later operations; SQLx migrations in `/migrations` | domain, sqlx | hold machine runtime state as truth |
| `project-service` | `ProjectService`: create (templates), inspect, import, update, archive, delete (metadata, or folder → OS trash), portable `project.json` | domain, project-store | write to project folders except through its documented safe paths (ADR-0007) |
| `vagrant-adapter` | `VagrantExecutor` trait and process implementation, typed `VagrantCommand` → argv builder, output parsers (`--machine-readable`), fixture tests | domain, platform, tokio | build shell strings |
| `operation-engine` | `OperationScheduler`: queue, per-environment mutual exclusion, cancellation, log files, event emission through an `EventSink` trait | domain, vagrant-adapter, project-store | depend on Tauri directly |
| `provider-registry` | `ProviderAdapter` trait, capability flags, prerequisite checks | domain, platform, vagrant-adapter | promise capabilities that detection didn't confirm |
| `config-model` | Supported-subset Vagrantfile model, generator, "can this be edited visually" analysis, backups and diffs | domain | rewrite user Ruby it doesn't understand |
| `src-tauri` (app) | Tauri commands, event bridge (`EventSink` impl), state wiring, tracing setup, capabilities | all crates | contain business logic |

Crates are created **when the phase that needs them starts** (see the
roadmap), so the workspace never holds empty scaffolding. Present so far:
`platform` (Phase 1), plus `domain`, `project-store` and `project-service`
(Phase 2).

Every trait that touches the outside world (`VagrantExecutor`,
`PlatformAdapter`, `EventSink`) has a fake implementation for tests.
Persistence is tested against real in-memory SQLite, and destructive file
actions go through injectable functions (ADR-0007). Business logic is tested without a real
Vagrant, VM or WebView.

## 3. Process execution model

- Commands are built as typed values (`VagrantCommand::Up { machine:
  Option<MachineName>, provision: bool }`) and turned into an **argument
  vector**. Nothing is interpolated into a shell string. No shell is used
  unless an operation really requires one, such as launching an external
  terminal, and that case is isolated in `platform`.
- Every invocation has an explicit working directory: the environment root,
  validated as an existing directory that contains the Vagrantfile.
- Read-only queries (`status`, `box list`, `version`, `plugin list`) use
  `--machine-readable` and fixture-tested parsers.
- stdout and stderr are streamed line by line to a log file on disk (one
  file per operation) and as `operation.stdout` / `operation.stderr` events.
  SQLite stores only the operation row, its final state and a short error
  summary.
- Child processes are tracked so the app can cancel them (graceful signal
  first, then kill after a timeout; on Windows, job objects kill the whole
  process tree). An operation that is cancelled or fails is recorded exactly
  as that, never as a rollback.
- On startup, any operation still marked `running` from a previous session
  is set to `interrupted` (crash recovery). The environment's status is then
  re-queried rather than guessed.

## 4. Operation scheduling

- Each environment has a single **write lock**. Mutating operations (`up`,
  `halt`, `reload`, `suspend`, `resume`, `provision`, `destroy`) take it, so
  at most one runs per environment and further requests queue or are
  refused with a conflict error that the UI shows.
- Read-only operations (`status`) may run concurrently with each other but
  not alongside a mutating operation on the same environment. The scheduler
  re-runs `status` itself after each mutating operation finishes.
- Box operations (`box add/remove/update`) take a global box lock, because
  Vagrant's box store is shared across environments.
- An operation is reported as `succeeded` only after the process exits with
  code 0 **and** the follow-up status refresh has finished. A refresh that
  contradicts the expected state produces a warning, not a silent success.

### Operation state machine

```
queued ─► running ─► succeeded
   │         ├─────► failed
   │         ├─────► cancel_requested ─► cancelled
   │         └─────► interrupted   (process lost / app crash, set on recovery)
   └─► cancelled  (removed from queue before start)
```

## 5. Event protocol (backend → frontend)

All events share one envelope, emitted on a single Tauri channel name per
category:

```jsonc
{
  "type": "operation.stdout",        // operation.started|stdout|stderr|progress|warning|completed|failed|cancel_requested|cancelled, environment.status_updated, machine.status_updated
  "timestamp": "2026-10-28T12:00:00Z",
  "operationId": "…",                // when applicable
  "environmentId": "…",              // when applicable
  "machineId": "…",                  // when applicable
  "payload": { "line": "…" }
}
```

The frontend never treats its own memory as the record. Operation history
is always loaded from the backend, and events only update live views.

## 6. Source of truth

| Data | Authority | Vagrant Studio's copy |
|---|---|---|
| Machine runtime state | `vagrant status` | Cached snapshot with a timestamp, shown as "observed at …" |
| Machine definitions | The Vagrantfile, evaluated by Vagrant | `machine_config` rows hold app-managed settings and cached metadata only |
| Project/environment metadata | SQLite (local) and optional `.vagrant-studio/project.json` (portable) | — |
| Operation logs | Log files under the app data dir | SQLite holds an index row per operation |

The UI always distinguishes **configured** values (from the Vagrantfile or
the app) from **observed** values (from Vagrant or the provider). Metrics
that the provider can't reliably supply are shown as "unavailable", never
estimated.

## 7. Vagrantfile editing safety

Vagrantfiles are Ruby. The app never claims to fully parse them.

1. **Raw mode** is always available and saves exactly what the user typed.
2. **Visual mode** works only on files the app generated itself (marked with
   a header and a model checksum) or on files that pass a conservative
   recognizer for the supported subset. Anything else opens read-only in
   visual mode with an explanation.
3. **Diff preview** is required before any write the app generates. Writes
   are atomic (temp file plus rename), and a timestamped backup is made
   before any automated edit.
4. Imported files are never overwritten with generated content without an
   explicit "replace" confirmation that shows the diff.

## 8. Platform strategy

- One codebase. Platform differences live behind `PlatformAdapter` and
  `#[cfg(target_os)]` inside `crates/platform`.
- The app runs **the Vagrant installed on the OS it is running on**. A
  Windows build never reaches into WSL, and Linux builds never call `.exe`
  files or `/mnt/c` paths. When the app itself runs inside WSL (a development
  scenario only), it shows a "development host: WSL" notice and treats
  `VAGRANT_WSL_ENABLE_WINDOWS_ACCESS` setups as unsupported for release
  validation.
- Executable discovery checks the configured override, then `PATH` (with
  `PATHEXT` on Windows), then well-known install locations
  (`C:\Program Files\Vagrant\bin`, `/usr/bin`, `/opt/vagrant/bin`).
- Paths are stored as UTF-8 where possible. Non-UTF-8 paths are rejected
  with a clear error rather than mangled. Paths containing spaces and
  non-ASCII characters are part of the test matrix.

## 9. Security

- Tauri capabilities grant only the commands the app defines and the
  minimal core permissions. There is no generic `shell` or `fs` plugin access
  from the WebView. The frontend can't name an executable or pass free-form
  argv.
- Destructive actions (`destroy`, `box remove`, deleting project files)
  require a typed confirmation in the UI **and** a confirmation token
  checked by the backend command.
- Log output passes through a redaction filter for common secret patterns
  (private key blocks, `password=`, tokens in URLs, values of environment
  variables whose names look like secrets) before it reaches disk or events.
- The diagnostics export omits secrets and replaces absolute paths with
  placeholders by default.

## 10. Frontend structure

```
apps/desktop/src/
  app/          bootstrap, router, theme
  components/   ui/ (primitives built on reka-ui), layout/, status/, feature folders
  pages/        one per route (Dashboard, Projects, Machines, Boxes, Providers, Operations, Settings, project workspace)
  stores/       Pinia stores (one per domain area; never the system of record)
  services/     the only place that calls invoke()/listen(); typed wrappers
  types/        shared TS types mirroring Rust DTOs (camelCase serde)
```

## 11. Technical risks and unknowns

| # | Risk / unknown | Impact | Mitigation | Phase |
|---|---|---|---|---|
| R1 | No GitHub Fedora runner | Fedora builds are only checked in a container; nobody launches the app on a Fedora desktop in CI | `fedora` container job for build and RPM; manual smoke test on a real Fedora VM before each release | 1, 9 |
| R2 | WebKitGTK system libraries missing in the WSL dev environment (needs sudo) | The desktop crate can't compile or launch locally until they're installed | Documented install command; pure crates and the frontend build without them | 1 |
| R3 | `vagrant status --machine-readable` output differs across Vagrant versions and providers | Wrong state shown | Parser fixtures per Vagrant version; unknown states map to `unknown` | 3 |
| R4 | Multi-machine targeting semantics vary by command (e.g. `ssh` needs exactly one machine, `destroy` without `-f` prompts) | Wrong machine targeted or hung prompts | Typed command builder encodes per-command rules; always pass `-f` and machine names explicitly; stdin closed | 3–4 |
| R5 | Cancelling `vagrant up` mid-boot can leave a VM partly created | Inconsistent state | Never assume rollback; force a status refresh; show "may be partially created" guidance | 4 |
| R6 | Vagrantfile is Ruby; the visual editor can't represent arbitrary logic | Data loss | Conservative recognizer, read-only fallback, diff and backup before writes | 5 |
| R7 | VirtualBox on Fedora needs RPM Fusion and kernel modules (Secure Boot signing) | Poor experience on the first provider path for Fedora | Detection explains it; libvirt evaluated as the Fedora-native provider in Phase 8 | 6, 8 |
| R8 | Hyper-V and VirtualBox coexistence on Windows (and WSL2 itself uses Hyper-V) | VirtualBox runs slowly or fails | Diagnostics detect it and explain; no automatic changes to host settings | 6 |
| R9 | Interactive SSH needs a PTY on both OSes | Terminal feature complexity | Evaluate `portable-pty` + xterm.js in Phase 7; external terminal fallback first | 7 |
| R10 | Several dependency majors are newer than the team's experience (vue-router 5, pinia 4, vitest 5, eslint 10, TypeScript 7) | Unexpected API changes | Pinned versions; TypeScript held at 6.0 because typescript-eslint supports `<6.1` (see ADR-0006) | 1 |
| R11 | Real VM lifecycle tests need nested virtualization that CI runners lack | Lifecycle is never exercised in CI | Fake executor and fixtures for unit tests; Vagrant's **docker** provider for automated lifecycle integration tests on Linux; real VirtualBox tests run manually on native hosts | 3–4, 9 |
| R12 | Code-signing for Windows installers and Linux packages | SmartScreen warnings | Out of scope until release hardening; documented as a known limitation | 9 |
