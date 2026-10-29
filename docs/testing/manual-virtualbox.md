# VirtualBox verification on Windows 11 (Phase 6)

Marks VirtualBox **Verified** on Windows (docs/provider-capabilities.md):
lifecycle, status, networking and box operations through the app. Run with
the local build (`pnpm build:win-local`). This plan reuses the other manual
plans with VirtualBox as the provider; it lists only what to do differently.

Host note: Hyper-V is active on this PC (WSL 2), so VirtualBox runs on top of
it. The Providers page warns about this. Expect slower boots; a VM that never
boots is a failure to record, not to work around by changing host settings.

Smoke test before this plan (2026-10-29, Vagrant 2.4.9 CLI, `ubuntu/jammy64`,
throwaway VM, destroyed afterwards): up 84 s, `--provision-with`, suspend
(`saved`), resume, halt (`poweroff`), destroy (`not_created`) all succeeded
under Hyper-V. Harmless output: "Warning: Connection reset. Retrying..."
while the guest boots, and a Guest Additions version note for old boxes.

## 0. Provider check
1. Open **Providers**. Expected: VirtualBox 7.1.18 *needs attention*, *Detected,
   not yet verified*, with the hypervisor warning and a fix explaining it.

## 1. Boxes (`manual-boxes.md`)
Run steps 1–7 with VirtualBox: in step 5 add `bento/ubuntu-24.04` with
provider `virtualbox` (about 1 GB). Keep it: the next sections use it.
Then run the remaining steps of that plan with a box no machine uses.

## 2. Lifecycle (`manual-lifecycle.md`)
Create a project from the **web-database** template with provider
**VirtualBox**, then run steps 1–15. Differences:
- Step 1 creates the private network `192.168.56.0/24`. If VirtualBox has no
  host-only adapter for it yet, it creates one, and Windows may show a UAC
  prompt; that is VirtualBox, not Vagrant Studio. Record whether it appeared.
- Step 5: a suspended VirtualBox machine shows *Suspended* (Vagrant state
  `saved`).

## 3. Networking (`manual-network.md` and checks from the host)
With `web` running (from section 2, or Start it again):
1. Private network: in PowerShell, `ping 192.168.56.10`. Expected: replies.
2. Forwarded port: add a provisioner named `http` to `web`, set to *Run only
   when chosen by name*, with the script
   `nohup python3 -m http.server 80 >/dev/null 2>&1 &`. Save, **Provision** →
   only `http`. Then open http://127.0.0.1:8080 in a browser. Expected: a
   directory listing.
3. Public network: run the *Public networks* steps of `manual-network.md`
   with VirtualBox, including step 7 (`up` or Reload, then ping the LAN
   address from another device or from the host).
4. Host-port check: run that section of `manual-network.md` once.

## 4. Provisioning (`manual-provisioning.md`)
Run *Running provisioners* steps 1–4 on this environment.

## Clean up
Destroy all machines (lifecycle step 13). Remove the extra public network
from the Vagrantfile if you keep the project.

Record results per section in `docs/implementation-status.md` and a row in
the verification log of `docs/provider-capabilities.md`.
