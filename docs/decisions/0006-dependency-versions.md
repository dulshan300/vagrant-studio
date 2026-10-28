# ADR-0006: Dependency version policy

- Status: Accepted
- Date: 2026-10-28

## Decision
- Use current stable majors, pinned through lockfiles (`pnpm-lock.yaml`,
  `Cargo.lock`), which are committed.
- **TypeScript is held at 6.0.x.** TypeScript 7 (the native port) is the npm
  `latest` tag, but `typescript-eslint` declares `typescript <6.1.0`. We
  revisit this when the lint toolchain supports TypeScript 7.
- pnpm is pinned through the `packageManager` field (pnpm 10.x) and run
  through Corepack, so no global install is needed.
- Node.js ≥ 22.12 (required by Vite 8).
- Rust: stable toolchain, edition 2024; `Cargo.lock` committed because the
  workspace produces an application.
- **Tauri npm packages and Rust crates must match on major.minor**
  (`@tauri-apps/api` ↔ `tauri`, `@tauri-apps/plugin-*` ↔ `tauri-plugin-*`).
  `tauri build` refuses to run otherwise. Cargo's resolver respects the
  workspace `rust-version`, so a low MSRV silently pins older Tauri crates. The
  MSRV is 1.90 because Tauri 2.12 requires it. When bumping either side, bump
  the other and run `tauri build` locally.

Versions in use at the time of writing (October 2026): tauri 2, vue 3.5,
vite 8, vue-router 5, pinia 4, tailwindcss 4, reka-ui 2, vitest 5, eslint 10.
