# Implementation Status

_Last updated: 2026-10-28_

## Current phase
**Phase 2 — Persistence and project management: complete (Windows).** Next: Phase 3, Vagrant CLI integration.

## Completed phases
- **Phase 0 — Inspection and planning** (2026-10-28): architecture, roadmap,
  provider capabilities, ADRs 0001–0006, risks (architecture.md §11).
- **Phase 1 — Desktop foundation** (2026-10-28, Windows): green Windows CI with NSIS
  installer; native Windows launch verified by the user.
- **Phase 2 — Persistence and project management** (2026-10-28, Windows): automated
  acceptance tests pass on Linux and as Windows binaries; user walkthrough on Windows
  (create from template, import, favorite/tags/archive, portable file, restart
  persistence, metadata-only remove) found no issues.

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
None recorded.

## Next recommended task
1. Push and confirm the Windows CI run for Phase 2.
2. Start Phase 3: `crates/vagrant-adapter` (Vagrant discovery and version, typed argv builder, streaming process runner with cancellation, `--machine-readable` parsers with fixtures) and `crates/operation-engine` (operation records, log files, crash recovery, events), then the Operations page.
3. Before release: install the CI NSIS installer by hand on Windows 11.
4. Later: re-enable the Ubuntu/Fedora CI jobs.
