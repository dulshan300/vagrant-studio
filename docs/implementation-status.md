# Implementation Status

_Last updated: 2026-10-28_

## Current phase
**Phase 5 — Vagrantfile editor: implemented, automated checks pass (2026-10-28).
Waiting for the user's Windows walkthrough** (`docs/testing/manual-vagrantfile-editor.md`)
before it is marked complete. Design: `docs/vagrantfile-editor.md`.

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

## Phase 5 results (2026-10-28)
| Check | Result |
|---|---|
| Rust tests, Linux | Pass: config-model 20 (new), project-service 30 + 1 opt-in (Vagrantfile read/save/stale/replace/backups/prune/create/symlink, templates editable visually), project-store 18, operation-engine 26 (environment lock), domain 15, platform 12, vagrant-adapter 26 |
| Same crates (config-model, project-service, project-store, operation-engine) as Windows binaries | Pass (symlink test is Unix-only) |
| `vagrant validate`, Windows Vagrant 2.4.9: 3 templates × 2 providers, their visual-editor regenerations, full-feature sample × 2 providers (CRLF) | **Pass**, 14/14 |
| Inline scripts in generated files evaluated by Vagrant's embedded Ruby | Equal to the model (incl. `#{`, backslash continuation, indentation, CRLF file) |
| clippy `-D warnings` / rustfmt | Pass |
| Frontend lint, format, typecheck, tests, production build | Pass, 52 tests (8 new: visual editor, page flow incl. leave guard) |
| Manual walkthrough on Windows | **Pending** |
| Windows CI for Phases 3–5 | Not yet pushed |

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
| Manual lifecycle on Windows, VirtualBox | Not reported |
| Windows CI for Phases 3–4 | Not yet pushed |

## Phase 3 results (2026-10-28)
| Check | Result |
|---|---|
| Rust tests, Linux | Pass: domain 15, platform 12, project-store 20, project-service 18, vagrant-adapter 24, operation-engine 11 |
| Same crates as Windows binaries | Pass |
| Opt-in real Vagrant test (Windows Vagrant 2.4.9, `vmware_desktop`) | Pass: version, multi-machine status |
| Templates × providers `vagrant validate` (3 × 2, Windows) | Pass |
| Frontend lint, typecheck, tests | Pass, 32 tests |
| Manual walkthrough on Windows (readiness, refresh status, validate, operations history, provider selection) | **Pass**: user, 2026-10-28 |
| Windows CI for Phase 3 | Not yet pushed |

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
| Windows CI for Phase 2 | Not yet pushed |

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
1. User walkthrough of `docs/testing/manual-vagrantfile-editor.md` on Windows
   (`pnpm build:win-local`); then mark Phase 5 complete.
2. Push and confirm the Windows CI run for Phases 3–5.
3. Phase 6: boxes, provider registry, networking and provisioning management
   (extends the visual subset: public networks, more provider options).
4. Optional: VirtualBox lifecycle run of `docs/testing/manual-lifecycle.md` on Windows.
5. Before release: install the CI NSIS installer by hand on Windows 11.
6. Later: re-enable the Ubuntu/Fedora CI jobs.
