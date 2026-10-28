# ADR-0003: Provider-agnostic architecture, one fully tested provider path at launch

- Status: Accepted
- Date: 2026-10-28

## Context
Provider behaviour differs a lot between Windows, Ubuntu and Fedora
(kernel modules, Hyper-V conflicts, plugin availability). Advertising broad
support without testing would mislead users.

## Decision
- Providers sit behind a `ProviderAdapter` capability interface
  (`docs/provider-capabilities.md`). The UI gates every action on detected
  capabilities.
- **VirtualBox** is the first provider path, targeted for *Verified* status
  on Windows 11 and Ubuntu 24.04. On Fedora it is *Detected* only until it is
  tested; libvirt is the preferred Fedora candidate in Phase 8.
- The Vagrant **docker** provider is used as the automated lifecycle test
  harness on Linux CI, where nested virtualization isn't available.
- Other providers are shown as *Detected (not verified)* with generic
  actions only.

## Consequences
- The first release makes narrow but honest claims.
- Adding a provider means implementing an adapter, its prerequisite checks
  and its integration tests, plus a row in the verification log.
