# Provider Capabilities

Vagrant Studio works with any provider at the architecture level, but the
first release supports **one provider path end to end**. Any other provider
appears in the UI only with the capabilities that runtime detection has
confirmed, and with an explicit "not yet verified" label until it passes the
same test plan (ADR-0003).

## Support levels

| Level | Meaning |
|---|---|
| **Verified** | Lifecycle, status, networking and box operations pass the integration test plan on that host OS |
| **Detected** | Vagrant reports the provider as usable, but Vagrant Studio hasn't verified it; generic actions only, with a warning |
| **Unavailable** | Not installed, or prerequisites missing; the UI explains how to fix it |

## Capability model

Each `ProviderAdapter` reports a set of capability flags. The UI enables an
action only when the flag is set **and** detection succeeded on this host.

| Capability | Description |
|---|---|
| `lifecycle.basic` | up / halt / reload / destroy / status |
| `lifecycle.suspend` | suspend / resume |
| `provision.shell` | shell provisioner |
| `network.forwarded_port` | `config.vm.network "forwarded_port"` |
| `network.private` | private network with a static IP or DHCP |
| `network.public` | bridged / public network |
| `synced_folder.default` | the provider's default synced-folder implementation |
| `config.cpu_memory` | CPU and memory settings in the provider block |
| `ssh` | `vagrant ssh` / `ssh-config` |
| `snapshot` | `vagrant snapshot` (Phase 8, gated) |
| `metrics` | reliable runtime metrics (Phase 8, gated; off by default) |

## Provider plan

| Provider | Windows 11 | Ubuntu 24.04 | Fedora | Status / phase |
|---|---|---|---|---|
| **VirtualBox 7.1** | First path | First path | Detected (needs RPM Fusion and kernel modules) | Integration in Phases 3–6; target **Verified** on Windows and Ubuntu for the first release |
| Docker (`docker` provider) | — | Test harness | Test harness | Used for automated lifecycle integration tests on Linux CI; shown to users as a container provider, clearly separated from VMs |
| libvirt (vagrant-libvirt plugin) | n/a | Candidate | **Preferred Fedora candidate** | Phase 8 |
| Hyper-V | Candidate | n/a | n/a | Phase 8 (needs admin and conflicts with VirtualBox; see R8) |
| **VMware Desktop** (vmware_desktop plugin + Vagrant VMware Utility) | **Windows verification path** (ADR-0003 amendment) | Candidate | Candidate | Detection in Phase 3; lifecycle verification on Windows in Phase 4 |

Compatibility constraint to check at runtime: each Vagrant version supports
only a range of VirtualBox versions. Detection compares `vagrant --version`
with `VBoxManage --version` and warns about combinations outside the
documented support range instead of blocking.

## Prerequisite checks (per provider)

- Is the executable present (`VBoxManage`, `virsh`, `docker`, …), and what version?
- Which plugin does the provider require (`vagrant plugin list --machine-readable`)?
- Host conflicts: Hyper-V enabled on Windows while VirtualBox is in use;
  missing `vboxdrv` kernel module or `/dev/kvm` access on Linux.

Every check returns a **finding** (ok, warning or error) with a
human-readable fix. Vagrant Studio never installs software or changes host
virtualization or network settings itself.

## Verification log

| Date | Host | Vagrant | Provider | Result |
|---|---|---|---|---|
| 2026-10-28 | Windows 11 x64 | 2.4.9 | VMware Desktop (Workstation 25.0.0, plugin 3.0.5) | **Read-only only:** version, plugin list, `status` of a multi-machine environment (not created), validate. Readiness detects a stopped VMware Utility. Lifecycle not yet tested (Phase 4). |
| 2026-10-28 | Windows 11 x64 | 2.4.9 | VirtualBox 7.1.18 | Read-only only: `status` (not created), box list, validate. Lifecycle not tested. |
