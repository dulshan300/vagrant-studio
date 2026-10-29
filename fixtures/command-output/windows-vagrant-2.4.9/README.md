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
