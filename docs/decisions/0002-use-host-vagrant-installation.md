# ADR-0002: Use the user's existing Vagrant installation; never bundle or auto-install

- Status: Accepted
- Date: 2026-10-28

## Context
Vagrant could be bundled into the installer or downloaded by the app. Bundling
adds licensing questions (Vagrant is BUSL-licensed since 2.3.8), update
duties, compatibility coupling with providers, and support burden.

## Decision
- Vagrant Studio **detects and uses the Vagrant installed on the host OS it
  runs on**. Discovery order: user-configured path override → `PATH` (with
  `PATHEXT` on Windows) → well-known install locations.
- When Vagrant or a provider is missing, the app shows clear, platform-specific
  setup guidance and links. It **does not** download or install Vagrant,
  plugins, hypervisors or drivers, and **does not** change host networking or
  virtualization settings (Hyper-V, kernel modules, firewall).
- Version compatibility is reported as a warning finding; the app doesn't
  refuse to run on unknown versions.

## Consequences
- Fewer moving parts and a clear support boundary.
- Users must install Vagrant themselves; the host-readiness panel is
  responsible for making this easy.
