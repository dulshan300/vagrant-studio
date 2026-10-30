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

Each `ProviderAdapter` (`crates/provider-registry`) reports a set of
capability flags. The UI enables an action only when the flag is set **and**
detection succeeded on this host. A provider that detection didn't find
reports no capabilities. Its support level is **Verified** only when it is
listed in `VERIFIED` (crates/provider-registry/src/lib.rs) for the host OS,
which happens together with a row in the verification log below. Today only
VirtualBox on Windows is listed; every other installed provider shows as
*Detected, not yet verified*.

Declared capabilities:

| Provider | Capabilities |
|---|---|
| VirtualBox, VMware Desktop, libvirt | everything below except `metrics` |
| Docker (containers) | `lifecycle.basic`, `network.forwarded_port`, `synced_folder.default`. No `lifecycle.suspend` (refused by the provider, see log), no `config.cpu_memory`; `ssh` and `provision.shell` need an SSH-enabled image, so they are not claimed |

Used so far: machine actions hide Suspend and Resume for a machine whose
provider lacks `lifecycle.suspend`, and the snapshot and export buttons appear
only with `snapshot` and `export`. Providers that aren't detected or aren't
in the registry are not limited (Vagrant decides).

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
| `snapshot` | `vagrant snapshot` save, list, restore, delete |
| `export` | `vagrant package` to a `.box` file |
| `metrics` | reliable runtime metrics. Not claimed by any provider: see below |

## Provider plan

| Provider | Windows 11 | Ubuntu 24.04 | Fedora | Status / phase |
|---|---|---|---|---|
| **VirtualBox 7.1** | First path | First path | Detected (needs RPM Fusion and kernel modules) | **Verified** on Windows 11 (2026-10-29); Ubuntu in the Phase 9 test matrix |
| Docker (`docker` provider) | — | Test harness | Test harness | Used for automated lifecycle integration tests on Linux CI; shown to users as a container provider, clearly separated from VMs |
| libvirt (vagrant-libvirt plugin) | n/a (not reported) | Candidate | **Preferred Fedora candidate** | Adapter, templates and visual settings in Phase 8; **never run on a real libvirt host**, so at most *Detected* |
| Hyper-V | Candidate | n/a | n/a | Not started: needs admin, conflicts with VirtualBox (R8), and the role isn't installed on the dev host |
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

Implemented (Phase 6):

| Provider | Checks |
|---|---|
| VirtualBox | `VBoxManage` on PATH or in the default install folder; `VBoxManage --version`; whether this Vagrant has a driver for that `major.minor` (table from each release's `plugins/providers/virtualbox/driver/meta.rb`: Vagrant 2.4.9 supports 4.0–7.2; unsupported is an error; an unknown Vagrant release is a warning, never a guess); Windows: CPUID "hypervisor present" bit (set when Hyper-V, WSL 2, Windows Sandbox or Memory Integrity run) is a warning (R8); Linux: `/dev/vboxdrv` missing is an error |
| VMware Desktop | `vmrun`; plugin `vagrant-vmware-desktop`; Vagrant VMware Utility reachable on 127.0.0.1:9922 |
| Docker | `docker`; daemon reachable (`docker version --format {{.Server.Version}}`), quoting Docker's error when not |
| libvirt (Linux only) | `virsh`; `virsh --version`; plugin `vagrant-libvirt`; `/dev/kvm`; daemon reachable on `qemu:///system` (`virsh -c qemu:///system version`), quoting virsh's error when not |

A provider is *unavailable* when its executable is missing, *needs
attention* when any check is a warning or error, and *ready* otherwise.

Every check returns a **finding** (ok, warning or error) with a
human-readable fix. Vagrant Studio never installs software or changes host
virtualization or network settings itself.

## Metrics

Phase 8 leaves `metrics` unclaimed. No source is both reliable and common to
the providers: `vmrun` has none, `VBoxManage metrics` has to be switched on per
VM first, and reading `/proc` over SSH works only on running Linux guests. The
app shows no CPU or memory usage rather than estimate it.

## Verification log

| Date | Host | Vagrant | Provider | Result |
|---|---|---|---|---|
| 2026-10-28 | Windows 11 x64 | 2.4.9 | VMware Desktop (Workstation 25.0.0, plugin 3.0.5) | **Read-only only:** version, plugin list, `status` of a multi-machine environment (not created), validate. Readiness detects a stopped VMware Utility. Lifecycle not yet tested (Phase 4). |
| 2026-10-28 | Windows 11 x64 | 2.4.9 | VirtualBox 7.1.18 | Read-only only: `status` (not created), box list, validate. Lifecycle not tested. |
| 2026-10-28 | WSL2 Ubuntu 24.04 (test harness) | 2.4.9 (Linux) | docker (Docker 29.7.2) | **Lifecycle through the engine:** status, `up` of named machines (including an `autostart: false` one), halt, reload, destroy with confirmation; follow-up status after each. `suspend` is refused by the provider ("not supported with the Docker provider"). Automated: `docker_lifecycle` test (opt-in). |
| 2026-10-28 | Windows 11 x64 | 2.4.9 | VMware Desktop | **Lifecycle used through the app** (Phase 4 build), accepted by the user without itemized step results. Observed: forwarded-port collision when resuming (Vagrant can't move ports on a suspended machine; Reload fixes it), template provisioner failure (cause not recorded), plugin warnings about `ethernet0/1.pcislotnumber` in the bento box VMX (harmless today). Not yet **Verified** per the full test plan. |
| 2026-10-29 | Windows 11 x64 | 2.4.9 | VirtualBox 7.1.18, VMware Desktop, Docker 29.7.2 | **Detection only** (`provider-registry` opt-in `real_host` test): VirtualBox supported by Vagrant 2.4.9, hypervisor bit set (WSL 2 is enabled) so VirtualBox needs attention; VMware ready; Docker daemon reachable. No lifecycle. |
| 2026-10-29 | Windows 11 x64 | 2.4.9 | VMware Desktop | **Shell provisioning through the app** (Phase 6 build, `almalinux/9`, two machines): named and unnamed provisioners, `--provision-with`, `run: "never"`, `up`/`reload --provision`, failing and cancelled provisioners. Walkthrough `docs/testing/manual-provisioning.md` passed (user, no itemized step results). Output captured the same day as parser fixtures. Not yet **Verified** per the full test plan. |
| 2026-10-29 | Windows 11 x64 (Hyper-V active) | 2.4.9 | VirtualBox 7.1.18 | **Verified.** CLI smoke test (`ubuntu/jammy64`: up 84 s, `--provision-with`, suspend → `saved`, resume, halt, destroy), then walkthrough `docs/testing/manual-virtualbox.md` through the app (Phase 6 build, `bento/ubuntu-24.04`): provider check, boxes, lifecycle (web-database template), private network ping, forwarded port, public network, host-port check, provisioning. All passed (user, no itemized step results; whether a UAC prompt appeared for the host-only adapter was not reported). Runs on top of Hyper-V; the hypervisor warning stays. |
| 2026-10-30 | Windows 11 x64 (Hyper-V active) | 2.4.9 | VirtualBox 7.1.18 | **Snapshots and export, CLI only** (`ubuntu/jammy64`, throwaway): save, duplicate name refused, list, restore with `--no-provision`, unknown name refused, delete; `package` of a halted machine (648 MB `.box`), existing output file refused. Output captured as fixtures. Restore forces the VM off and resumes it. One run hit Vagrant's boot timeout on `up` and on restore (SSH never answered, the VM itself was fine); a later `up` of the same box passed. Not yet run through the app. |
| 2026-10-30 | Windows 11 x64 | 2.4.9 | VMware Desktop | **Snapshots and export, CLI only** (`almalinux/9`, throwaway): save, list, restore with `--no-provision`, delete, empty list; `package` of a halted machine (473 MB `.box`), existing output file refused. List output has the same format as VirtualBox. Not yet run through the app. |
| 2026-10-30 | Windows 11 x64 | 2.4.9 | libvirt | **Not run.** Adapter tested with fakes only; no libvirt host available. Generated files pass `vagrant validate` on Windows, but without the plugin that checks Ruby syntax, not the plugin's settings. |
| 2026-10-30 | Windows 11 x64 | 2.4.9 | VMware Desktop | **Snapshots through the app** (Phase 8 build, `My Test`, `web`, running): save (about 4.5 minutes for a 2 GB machine), restore, delete, all succeeded per operation logs. User reported the walkthrough as passing with no itemized results; export of `db` and `box add` of the resulting file also succeeded. Not yet **Verified** per the full test plan. |
