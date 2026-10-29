Raw output captured from Vagrant 2.4.9 on Windows 11 (2026-10-28) in temporary
copies of the templates. CRLF line endings are intentional (see
`.gitattributes`). `status-vmware-utility-unreachable.txt` was captured while
the Vagrant VMware Utility service was stopped; it was recreated from the
captured text after being accidentally overwritten, with content unchanged.

Phase 6 (2026-10-29): `box-outdated-global.txt` is `vagrant box outdated --global
--machine-readable` (only `ui` lines). `machine-index.json` is a copy of Vagrant's
`~/.vagrant.d/data/machine-index/index` with the project folder paths renamed.

`virtualbox-list-bridgedifs.txt` (2026-10-29) is `VBoxManage list bridgedifs` from
VirtualBox 7.1.18 with GUIDs, IP addresses and MAC addresses replaced by
placeholders; names, statuses and line endings are as captured.

Provisioning (2026-10-29), stdout of each command unless named `-stderr`, captured
with a throwaway two-machine `almalinux/9` project on VMware Desktop (named,
unnamed, `path:`, `privileged: false` and `run: "never"` shell provisioners).
The Windows user name in temporary paths is replaced by `dev`.
`up-provision-multi.txt` is the first `up --no-tty web db`; `provision-with.txt`
is `provision --provision-with hello,setup --no-tty web` (runs in Vagrantfile
order); `provision-fail.txt` / `provision-fail-stderr.txt` are a script that
exits 3 (Vagrant exits 1; guest stderr appears on stdout);
`provision-not-created.txt` is `provision` on a machine never created (exit 0);
`reload-provision.txt` is `reload --provision --no-tty db`. Not stored because
the output was empty: `--provision-with nope` (an unknown name) printed nothing
and exited 0.
