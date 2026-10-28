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

## Amendment (2026-10-28): VMware Desktop on Windows
At the user's request, **VMware Desktop (`vmware_desktop`) on Windows** is the
provider path used for real-VM verification on Windows. The development
machine has VMware Workstation 25.0.0, the `vagrant-vmware-desktop` 3.0.5
plugin and the Vagrant VMware Utility (`VagrantVMware` service, manual start).
VirtualBox remains a candidate path. The capability model is unchanged.

Prerequisite checks for this path: `vmrun` present, plugin installed, and the
utility service reachable (when it isn't, `vagrant status` fails with
`DriverAPIRequestUnexpectedError` / connection refused on 127.0.0.1:9922;
see fixture `status-multi-not-created-vmware.txt`). Vagrant Studio reports a
stopped service and explains how to start it; it never starts services itself.

## Amendment (2026-10-28): choosing a provider
Users choose the provider when creating a project from a template. Templates
are stored provider-neutral (`templates/*/Vagrantfile.tmpl`) and rendered for
the chosen provider, so a generated Vagrantfile contains only that provider's
block. The choice is stored as the environment's default provider and passed
to Vagrant as `VAGRANT_DEFAULT_PROVIDER`. An app-wide default (Settings) is
pre-selected; providers that readiness checks report as not installed can't be
chosen. Existing machines keep the provider they were created with.
