# Manual test: network settings (Phase 6)

Run on Windows 11 with the local build (`pnpm build:win-local`). Use a test
environment whose Vagrantfile you don't mind changing (backups are kept).

## Public networks
1. Open an environment → **Vagrantfile** → **Visual**. Under *Public networks*
   choose **Add**. Expected: DHCP is selected and the host adapter field is
   filled with an active adapter (VirtualBox installed), e.g. your Wi-Fi.
2. Click the adapter field. Expected: the active adapters are suggested;
   adapters that are down are not.
3. Clear the adapter field. Expected: an amber note explains that VirtualBox
   may ask which adapter to use and that starting would then fail.
4. Switch to **Static IP**, type `192.168.1` (incomplete). Expected: an error
   under the field; Save is blocked. Enter a valid free address on your LAN.
5. Save. Expected: the diff shows a line like
   `config.vm.network "public_network", ip: "…", bridge: "…"`; tick
   "Validate after saving" and confirm Vagrant validates it.
6. Reopen the editor. Expected: it opens in Visual with the same values.
7. Optional, with a machine you can recreate: `up` (or Reload) it and check
   from another device on the LAN that the address answers (e.g. ping).

## Host-port check
1. Start something on a port, e.g. in PowerShell:
   `python -m http.server 8099` (or any program listening on a port).
2. In the visual editor add a forwarded port with host port `8099`.
   Expected: within about a second an amber note says something already uses
   host port 8099.
3. Set its protocol to **UDP**. Expected: the note disappears (Vagrant
   doesn't check UDP ports).
4. Stop the listener and change the host port to trigger a re-check, then
   back to 8099. Expected: no note.
5. With a running machine that forwards a port, open its editor. Expected:
   that port shows the note (the text says it may be the machine itself).

Record results in `docs/implementation-status.md`.
