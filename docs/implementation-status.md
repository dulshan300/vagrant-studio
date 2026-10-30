# Implementation Status

_Last updated: 2026-10-29_

## Current phase
**Phase 7 — Developer workspace: in progress** (branch
`phase-7-developer-workspace`). Phase 6 is merged into `main`; Windows CI
passed (run 40858003787, `2317b88`). SSH in an external terminal: done in
code and automated tests (terminal button on running machines with the `ssh`
capability; Windows Terminal, else a console window; Linux terminal list
untested); Windows Terminal detection checked on Windows (opt-in
`real_terminal` test); SSH through the app confirmed working on Windows by
the user (2026-10-29, no itemized results for `docs/testing/manual-ssh.md`).
Also done: project page redesign (full-width environment cards, details in an
Edit dialog, project/environment actions in menus; every action kept), and a
fix so the editor's CPU and memory section offers the environment's provider
first and marks settings for other providers as unused (user report,
2026-10-29). Workspace items chosen before the embedded terminal (user,
2026-10-30). Open in editor: done in code and automated tests (known editors
detected by install folder on Windows and `PATH` on Linux; VS Code, Zed and
Sublime Text found on the Windows dev host; editor choice in Settings, `.exe`
only on Windows; `ELECTRON_RUN_AS_NODE` cleared for the editor, found when the app
was started from a VS Code terminal). Confirmed working on Windows by the user
2026-10-30 (no itemized results for `docs/testing/manual-editor.md`).
Notes and tasks per project: done in code and automated tests (tables from
migration 0001, now used; plain-text notes; tasks with done state, rename and
ordering; ownership checked per project; not part of the portable
`project.json`). Confirmed working on Windows by the user 2026-10-30 (no
itemized results for `docs/testing/manual-notes-tasks.md`). Git status:
done in code and automated tests (new crate `crates/git`: read-only
`git status --porcelain=v2` through the adapter's `Runner`, `core.fsmonitor`
off, `--no-optional-locks`, counts scoped to the project folder; real-Git
test passes on Linux and as a Windows binary); badge in the project header;
walkthrough `docs/testing/manual-git.md` not yet run.

## Completed phases
- **Phase 0 — Inspection and planning** (2026-10-28): architecture, roadmap,
  provider capabilities, ADRs 0001–0006, risks (architecture.md §11).
- **Phase 1 — Desktop foundation** (2026-10-28, Windows): green Windows CI with NSIS
  installer; native Windows launch verified by the user.
- **Phase 2 — Persistence and project management** (2026-10-28, Windows): automated
  acceptance tests pass on Linux and as Windows binaries; user walkthrough on Windows
  (create from template, import, favorite/tags/archive, portable file, restart
  persistence, metadata-only remove) found no issues. Windows CI green.
- **Phase 3 — Vagrant CLI integration** (2026-10-28, Windows): process runner, operation
  engine, readiness checks, Operations/Providers/Machines pages, provider selection
  (VirtualBox, VMware Desktop). Tests pass on Linux and as Windows binaries, including
  opt-in real Vagrant 2.4.9 + VMware Desktop. User walkthrough on Windows passed.
- **Phase 4 — Multi-machine lifecycle** (2026-10-28, Windows): scheduler (per-environment
  lock, box lock, queueing, conflicts), explicit machine targeting, follow-up status with
  expected-state warnings, destroy confirmation tokens, machine actions UI. Automated tests
  and the opt-in docker lifecycle test pass. Real lifecycle used by the user with VMware
  Desktop on Windows; the user marked the phase done without itemized step results.
  VirtualBox lifecycle not reported.
- **Phase 5 — Vagrantfile editor** (2026-10-29, Windows): `crates/config-model`, visual
  forms and raw CodeMirror editor, diff before every save, revision check, backups in app
  data, atomic save under the environment lock. Automated tests pass on Linux and as
  Windows binaries; 14 generated files pass `vagrant validate`; user walkthrough of
  `docs/testing/manual-vagrantfile-editor.md` passed with no issues.
- **Phase 6 — Boxes, providers, networking, provisioning** (2026-10-29, Windows): box
  management, `crates/provider-registry` (capabilities, support level, prerequisite
  findings), public networks and host-port checks, shell provisioner management and
  per-machine results. Automated tests pass on Linux and as Windows binaries;
  walkthroughs for boxes, providers, networking and provisioning passed on Windows
  (user). VirtualBox **Verified on Windows 11** under Hyper-V (smoke test and
  `docs/testing/manual-virtualbox.md`). Ubuntu verification dropped from this phase
  by the user; it belongs to the Phase 9 test matrix.

## Phase 6 results (2026-10-29)
| Task | State |
|---|---|
| Box management: list with machine usage, update check, add (catalog / URL / file), update, remove with typed confirmation and token; box write lock | Done; Windows walkthrough accepted |
| `crates/provider-registry` (capabilities, docker adapter, VirtualBox/Vagrant compatibility, Hyper-V coexistence) | Done (automated + real-host detection on Windows); Providers page accepted by the user 2026-10-29 |
| Networking: public networks in the visual subset, host-port conflict detection | Done in code; automated tests pass; 14 generated files pass `vagrant validate`; walkthrough (`docs/testing/manual-network.md`) **passed** on Windows (user, 2026-10-29, "test ok", no itemized step results) |
| Shell provisioner management and results: named provisioners in the visual subset, Provision dialog (all / chosen names via `--provision-with`; Start/Restart with `--provision`), backend name check, results parsed from output and stored per machine (migration 0003), shown on machine rows and in the operation drawer | Done in code; automated tests pass; 14 generated files pass `vagrant validate`; walkthrough (`docs/testing/manual-provisioning.md`) **passed** on Windows with VMware Desktop (user, 2026-10-29, "all pass", no itemized step results) |
| VirtualBox verification on Windows 11 and Ubuntu | Windows: **Verified** 2026-10-29. CLI smoke test passed under Hyper-V (up, provision, suspend/resume, halt, destroy); walkthrough `docs/testing/manual-virtualbox.md` passed (user, no itemized step results; UAC prompt not reported). Added to `VERIFIED` in `crates/provider-registry`. Ubuntu: moved to Phase 9 (user, 2026-10-29) |

| Check | Result |
|---|---|
| Rust tests, Linux | Pass: vagrant-adapter 34 (box arguments, outdated parser, machine index, query), operation-engine 28 (box confirmation, box lock, duplicate refusal), project-service 31, project-store 18, domain 15 |
| Same crates as Windows binaries | Pass |
| Opt-in real Windows Vagrant 2.4.9 (`--include-ignored`) | Pass: `box list` (4 boxes), machine index (3 machines), `box outdated --global` (1 outdated) |
| clippy `-D warnings` / rustfmt | Pass |
| Frontend lint, format, typecheck, tests | Pass, 59 tests (7 new: box helpers, Boxes page flow) |
| Manual walkthrough on Windows (`docs/testing/manual-boxes.md`) | **Accepted by the user** 2026-10-29 ("all good", no itemized step results) |
| provider-registry tests, Linux and as Windows binaries | Pass, 14 (adapters with fake runner/platform/probe, VirtualBox version table, serialization); vagrant-adapter now 32 (provider checks moved out) |
| Opt-in `real_host` test (`cargo test -p vagrant-studio-provider-registry --test real_host -- --ignored --nocapture`) | Windows: Vagrant 2.4.9 + plugin; VirtualBox 7.1.18 supported, hypervisor bit set → needs attention (warning); VMware ready; Docker 29.7.2 ready. WSL: only Docker installed; the VMware Utility port shows as reachable because WSL forwards localhost to Windows (dev setup only) |
| Frontend after provider-registry | Pass, 60 tests (capability-gated Suspend/Resume) |
| Provisioning: fixtures | Captured 2026-10-29 with Windows Vagrant 2.4.9 + VMware Desktop (throwaway `almalinux/9` project, destroyed afterwards): first `up`, `--provision-with`, failing script, not-created machine, `reload --provision`. Findings: unknown `--provision-with` names are ignored with exit 0; `run: "never"` runs only by name; guest stderr appears on Vagrant's stdout |
| Provisioning: Rust tests, Linux and as Windows binaries | Pass: config-model 24, vagrant-adapter 39 (parser against fixtures, `--provision-with` arguments), project-store 19 (migration 0003), operation-engine 30 (results stored per machine, latest per machine) |
| Provisioning: frontend | Pass, 73 tests (provisioner choice across machines, outcome labels, Provision dialog) |
| Networking: config-model | Pass, 21 (public networks in the multi-machine sample, DHCP spellings, unsupported options) |
| Networking: provider-registry | Pass, 17 on Linux and as Windows binaries (bridged-interface parser on the anonymized Windows fixture, port check against a real listener) |
| Networking: `vagrant validate`, Windows Vagrant 2.4.9 | **Pass**, 14/14 (feature sample now has a static public network with `bridge:` and a DHCP one) |
| Networking: real Windows host | `VBoxManage list bridgedifs`: one usable adapter (Wi-Fi), two `Down` ones skipped. Port check: closed localhost ports take ~2 s to refuse on Windows, so checks run in parallel with a 250 ms limit (Vagrant uses 100 ms) |
| Networking: frontend | Pass, 63 tests (adapter suggestion, missing-adapter warning, in-use warning only for TCP) |

Facts checked against Vagrant 2.4.9 (Windows and its source): `box outdated --global
--machine-readable` prints only `ui` lines; `box remove` decides "in use" from the
machine index (name, provider, version, architecture) and `--force` skips its prompt;
`box add` asks interactively for a provider when a box offers several, so the UI
requires one for catalog boxes; `box list` doesn't load a Vagrantfile from parent
folders. The machine index records architecture `auto` while `box list` shows
`amd64`, so the app treats `auto` as matching any architecture (it may over-report
use, never under-report).

## Phase 5 results (2026-10-28)
| Check | Result |
|---|---|
| Rust tests, Linux | Pass: config-model 20 (new), project-service 30 + 1 opt-in (Vagrantfile read/save/stale/replace/backups/prune/create/symlink, templates editable visually), project-store 18, operation-engine 26 (environment lock), domain 15, platform 12, vagrant-adapter 26 |
| Same crates (config-model, project-service, project-store, operation-engine) as Windows binaries | Pass (symlink test is Unix-only) |
| `vagrant validate`, Windows Vagrant 2.4.9: 3 templates × 2 providers, their visual-editor regenerations, full-feature sample × 2 providers (CRLF) | **Pass**, 14/14 |
| Inline scripts in generated files evaluated by Vagrant's embedded Ruby | Equal to the model (incl. `#{`, backslash continuation, indentation, CRLF file) |
| clippy `-D warnings` / rustfmt | Pass |
| Frontend lint, format, typecheck, tests, production build | Pass, 52 tests (8 new: visual editor, page flow incl. leave guard) |
| Manual walkthrough on Windows (`docs/testing/manual-vagrantfile-editor.md`) | **Pass**: user, 2026-10-29, no issues |
| Windows CI for Phases 3–5 | **Pass** (run 40832387355, `7822262` on `main`, 2026-10-29): all checks, MSVC build, NSIS installer artifact (4.6 MB); covers Phases 2–6 |

New dependencies: `@codemirror/{view 6.43.14, state 6.7.6, commands 6.11.1, search 6.7.2,
language 6.13.1, legacy-modes 6.5.5, merge 6.12.2}` and `@lezer/highlight 1.2.5`, all
published at least two weeks before they were added. `sha2 0.10` (already in the lockfile).
New capability: `core:window:allow-destroy`, used by the unsaved-changes guard on window close.

## Phase 4 results (2026-10-28)
| Check | Result |
|---|---|
| Rust tests, Linux | Pass: domain 15, platform 12, project-store 18, project-service 20, vagrant-adapter 26, operation-engine 24 (scheduler, queueing, conflicts, cancel while queued, follow-up status, expected-state warnings, confirmation tokens) |
| Same crates (domain, vagrant-adapter, operation-engine) as Windows binaries | Pass |
| Opt-in docker lifecycle test (Linux Vagrant 2.4.9 + Docker 29.7.2, WSL2) | **Pass** (71 s): status, `up` of named machines incl. `autostart: false`, halt, reload, suspend refused by provider, confirmed destroy; no containers left |
| clippy `-D warnings` / rustfmt | Pass |
| Frontend lint, format, typecheck, tests | Pass, 37 tests |
| Manual lifecycle on Windows, VMware Desktop | **Accepted by the user** 2026-10-28 (no itemized step results). Seen during use: start/provision runs, a forwarded-port collision on resume (Vagrant error, reported as a failed operation), a provisioner failure whose summary hid the cause (fixed in `69e74ac`), VMX `pcislotnumber` warnings from the vmware plugin (harmless) |
| Manual lifecycle on Windows, VirtualBox | **Passed** 2026-10-29 as part of `docs/testing/manual-virtualbox.md` (user) |
| Windows CI for Phases 3–4 | **Pass** (run 40832387355, `7822262` on `main`, 2026-10-29): all checks, MSVC build, NSIS installer artifact (4.6 MB); covers Phases 2–6 |

## Phase 3 results (2026-10-28)
| Check | Result |
|---|---|
| Rust tests, Linux | Pass: domain 15, platform 12, project-store 20, project-service 18, vagrant-adapter 24, operation-engine 11 |
| Same crates as Windows binaries | Pass |
| Opt-in real Vagrant test (Windows Vagrant 2.4.9, `vmware_desktop`) | Pass: version, multi-machine status |
| Templates × providers `vagrant validate` (3 × 2, Windows) | Pass |
| Frontend lint, typecheck, tests | Pass, 32 tests |
| Manual walkthrough on Windows (readiness, refresh status, validate, operations history, provider selection) | **Pass**: user, 2026-10-28 |
| Windows CI for Phase 3 | **Pass** (run 40832387355, `7822262` on `main`, 2026-10-29): all checks, MSVC build, NSIS installer artifact (4.6 MB); covers Phases 2–6 |

## Phase 2 results (2026-10-28)
| Check | Result |
|---|---|
| Rust tests, Linux (`cargo test --workspace`) | Pass: domain 12, platform 12, project-store 13, project-service 18 |
| Same crates as Windows binaries (windows-gnu via WSL interop) | Pass, after fixing 2 Windows-only issues (case-insensitive Vagrantfile name; stale `DirEntry` metadata in a test) |
| clippy `-D warnings` / rustfmt | Pass |
| Frontend lint, typecheck, tests | Pass, 22 tests |
| Templates `vagrant validate` (Windows Vagrant 2.4.9) | Pass, all 3 |
| Acceptance: restart persistence, import changes no files, metadata delete keeps files, migration tests | Covered by automated tests, all passing |
| Windows app launch (local build) | Starts; database created in `%APPDATA%\dev.vagrantstudio.desktop` |
| Manual UI walkthrough on Windows | **Pass**: user, 2026-10-28, no issues |
| Windows CI for Phase 2 | **Pass** (run 40832387355, `7822262` on `main`, 2026-10-29): all checks, MSVC build, NSIS installer artifact (4.6 MB); covers Phases 2–6 |

## Phase 1 progress
| Task | State |
|---|---|
| Cargo workspace, pnpm workspace, editor/format configs | Done |
| `crates/platform` (host/WSL detection, executable discovery) | Done, tested |
| Vue 3 + strict TS + Vite 8 + Tailwind 4 + router + Pinia | Done |
| UI primitives, status badges, light/dark/system themes | Done, tested |
| App shell, 7 global sections, Ctrl/⌘+1…7 navigation | Done, tested |
| Settings page (theme, app/host info) | Done |
| Tauri app crate, minimal capabilities, `get_app_info` | Done; compiles, clippy clean, launches under WSLg |
| ESLint, Prettier, vue-tsc, Vitest, rustfmt, clippy | Done |
| CI: Windows build, test and package | **Pass** (run 40716119723, `93ffb16`, 2026-10-28): all checks, MSVC build, NSIS installer artifact (2.3 MB) |
| CI: Ubuntu / Fedora jobs | **Paused** by decision 2026-10-28 (Windows first); `.deb`/`.rpm` did build locally in WSL after the Tauri version fix |
| App launch in WSL2 (WSLg) | Verified by the user 2026-10-28: window renders, sidebar navigation works, host readiness shows WSL + Vagrant path |
| Windows 11 native launch | **Verified by the user 2026-10-28** (local windows-gnu build: UI renders, Windows host detected). NSIS installer from CI not yet installed by hand |
| Ubuntu / Fedora launch | Not verified (Linux CI paused) |

## Test results (WSL2 Ubuntu 24.04, 2026-10-28)
| Check | Result |
|---|---|
| `pnpm lint` (eslint, 0 warnings allowed) | Pass |
| `pnpm format:check` | Pass |
| `pnpm typecheck` (vue-tsc) | Pass |
| `pnpm test` (vitest) | Pass, 12 tests in 4 files |
| `pnpm build` (Vite production build) | Pass |
| `cargo fmt --all --check` | Pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass (incl. desktop crate) |
| `cargo test --workspace --locked` | Pass, 12 tests (platform); desktop crate has no tests yet |
| `pnpm dev` (WSLg) | App starts; only WSLg `libEGL`/`/dev/dri` permission warnings (software rendering) |

## Environmental blockers
1. ~~WebKitGTK packages missing in WSL2~~ — installed by the user 2026-10-28.
2. ~~Windows CI failing~~: fixed. The first run failed on two path tests that
   assumed Unix paths (reproduced and fixed locally by running the tests as a
   Windows binary, see below) and on a Tauri npm/crate version mismatch.
3. **Slow npm registry access** from this machine (10+ minutes for a full
   install). This doesn't affect correctness.

## Host facts (development machine)
- WSL2 Ubuntu 24.04.5, x86_64; Rust 1.99.1; Node 24.22.1; pnpm 10.34.9 via Corepack.
- Linux Vagrant 2.4.9 at `/usr/bin/vagrant` with `VAGRANT_WSL_ENABLE_WINDOWS_ACCESS=1`
  (a WSL-only setup); no plugins installed.
- Windows host: VirtualBox 7.1.18 and Hyper-V (`vmcompute.exe`) present.
  Hyper-V coexistence is risk R8.
- Docker 29.7.2 is available, a candidate harness for automated lifecycle tests.

## Local Windows testing from WSL
WSL has MinGW-w64 and the `x86_64-pc-windows-gnu` Rust target, and Windows
binaries run through WSL interop:
`cargo test -p vagrant-studio-platform --target x86_64-pc-windows-gnu` runs the
tests natively on Windows, and `pnpm build:win-local` builds a standalone
Windows `.exe` with the frontend embedded. This is a GNU-toolchain check only; release builds
use MSVC in CI. Windows has no local MSVC or Rust install.

## Known bugs
None open. Fixed during the Phase 4 walkthrough: provisioner failures showed only
Vagrant's generic "SSH command responded with a non-zero exit status" text, because
Vagrant prints guest output to stdout; failure summaries of human-output commands now
include the last output lines.

## Next recommended task
1. Run `docs/testing/manual-git.md` on Windows.
2. Phase 7: cross-project port warnings, shortcuts, log navigation.
3. Phase 7: embedded PTY terminal (evaluate portable-pty + xterm.js).
3. Before release: install the CI NSIS installer by hand on Windows 11.
4. Later: re-enable the Ubuntu/Fedora CI jobs.
