# Manual lifecycle test plan (Phase 4)

Run on a native host with the app built for that OS (on Windows,
`pnpm build:win-local` from WSL, or the CI installer). Record the date,
Vagrant, provider and result in `docs/provider-capabilities.md` (verification
log) and `docs/implementation-status.md`.

Automated coverage exists for scheduling, confirmation and follow-up status
(fake runner, `cargo test -p vagrant-studio-operation-engine`) and for real
lifecycle with the docker provider on Linux
(`cargo test -p vagrant-studio-operation-engine --test docker_lifecycle -- --ignored`).
This plan covers what only a real VM provider can show.

## Setup

1. Create a project from the **web-database** template (machines `web` and
   `db`) with the provider under test (VMware Desktop, then VirtualBox).
2. Open the project and click **Refresh status**. Both machines show
   *Not created*.

## Steps

| # | Action | Expected |
|---|---|---|
| 1 | Select `web` only, click **Start** | Operation `vagrant up --no-tty web` runs; output streams in the drawer. When it ends, status refreshes by itself: `web` *Running*, `db` *Not created*. Operation *Succeeded* with no warning. |
| 2 | While step 1 runs, click **Start** on `db`'s row | It shows as *queued* and starts only after step 1 finishes (one lifecycle operation per environment). |
| 3 | Click **Start** on `web` again while it is queued or running | Refused with "already queued or running". |
| 4 | Clear the selection, click **Stop** (all machines) | `vagrant halt --no-tty web db`; both end *Stopped*. |
| 5 | Select `web`, **Start**, then **Suspend** | `web` *Suspended* (VMware: `suspended`, VirtualBox: `saved`). |
| 6 | **Resume** on `web`'s row | `web` *Running*. |
| 7 | Select `web`, **Provision** | Provisioner output appears; *Succeeded*. |
| 8 | Select `web`, **Reload** | `web` *Running* after the restart. |
| 9 | Start `db`; while it boots, open the drawer and **Cancel** | Operation *Cancelled* with the "left part-way" note; status is refreshed right after and shows whatever Vagrant reports. Refresh again later to check it is consistent. |
| 10 | Queue **Stop** for both, then cancel the queued one before it starts | Shows *Cancelled before it started*; Vagrant never ran it (no command output). |
| 11 | Select `web`, **Destroy** | Dialog asks to type `web`; the button stays disabled until it matches. Cancel the dialog: nothing runs. |
| 12 | Repeat, type `web`, confirm | `vagrant destroy --force --no-tty web`; `web` *Not created*; `db` untouched. |
| 13 | Clear the selection, **Destroy** | The phrase is the environment name; after confirming, all machines *Not created*. Project files and the Vagrantfile remain. |
| 14 | Close the app while a **Start** runs, reopen | The operation shows *Interrupted*; Refresh status reports the real state. |
| 15 | Rename `web` to `web2` in the Vagrantfile without refreshing, then click Start on `web` | The operation fails with Vagrant's "machine not found" message; the follow-up refresh then lists `web2` instead of `web`. Restore the name afterwards. |

Clean up with step 13 if anything is left running.
