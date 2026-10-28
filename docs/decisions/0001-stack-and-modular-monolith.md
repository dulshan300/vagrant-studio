# ADR-0001: Tauri 2 + Rust + Vue 3 modular monolith

- Status: Accepted
- Date: 2026-10-28

## Context
We need a cross-platform (Windows 11, Ubuntu, Fedora) desktop application that
drives a local CLI, works offline, and keeps a small footprint.

## Decision
One Tauri 2 application. The Rust backend is split into workspace crates by
responsibility (see `docs/architecture.md` §2). The frontend uses Vue 3,
strict TypeScript, Vite, Tailwind CSS 4, reka-ui primitives in the shadcn-vue
style, Pinia and Vue Router. There are no extra services, no Python backend
and no cloud component.

Crates are added only when the phase that needs them starts, so the
workspace never holds empty scaffolding.

## Consequences
- The app crate stays thin; business logic is testable without a WebView.
- Linux builds need WebKitGTK 4.1 development packages on the build host.
- Vue Flow is planned for diagrams but isn't installed until a phase uses it.
