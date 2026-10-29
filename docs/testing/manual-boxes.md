# Manual box management test plan (Phase 6)

Run on a native host with the app built for that OS (on Windows,
`pnpm build:win-local` from WSL). Record the date, Vagrant version and result
in `docs/implementation-status.md`.

Automated coverage: argument building and validation, the `box outdated
--global` parser and the machine-index reader against Windows fixtures
(`cargo test -p vagrant-studio-vagrant-adapter`); confirmation tokens, the box
write lock and duplicate refusal (`cargo test -p vagrant-studio-operation-engine`);
the Boxes page flow (`BoxesPage.spec.ts`). The opt-in
`real_vagrant_box_store_queries` test runs `box list`, reads the index and runs
the update check against the host's Vagrant. Box add, update and remove against
a real catalog are covered only by this plan.

Downloads are large (often 0.5–2 GB per box). **Never confirm removal of a
box that a machine you care about still uses**: linked clones (the VMware
Desktop default) read their disk from the box.

## Steps

| # | Action | Expected |
|---|---|---|
| 1 | Open **Boxes** | Every box from `vagrant box list` is listed with provider, version and architecture (none shown for `n/a`). **Used by** shows the machines Vagrant's index records for that exact version, linked to the project when the folder is a registered environment. Index entries whose folder no longer exists are not counted. |
| 2 | Click **Check for updates** | Takes a few seconds per box. Each row shows *newest*, *N available*, *not from a catalog* or *check failed* (hover for the reason). **Update** appears only on outdated rows. |
| 3 | Click **Add box**, source *Catalog name*, enter `ubuntu`  | Refused: not of the form owner/box. |
| 4 | Source *URL*, enter `https://user:pw@example.com/x.box`, any name | Refused before anything runs: credentials in URLs aren't accepted. |
| 5 | Source *Catalog name*, enter a small box you don't have yet for the provider under test (e.g. `bento/ubuntu-24.04`), click **Add box** | A *Box changes* panel shows *Add box* queued, then running; **Output** opens the streaming log of `vagrant box add --no-tty --provider … bento/ubuntu-24.04`. When it succeeds the list re-reads and shows the box. |
| 6 | While step 5 downloads, start a machine in any project | The **Start** operation stays *queued* until the box download ends (box changes block `up`). |
| 7 | Start step 5 again with the same box while it is still running | Refused with "already queued or running". |
| 8 | Add a box and **Cancel** it from the drawer while it downloads | Operation *Cancelled* with "Refresh the box list…"; the list shows whether anything was added (normally nothing). |
| 9 | On an outdated row, click **Update** | `vagrant box update --no-tty --box NAME --provider P` runs; afterwards both the old and the new version are listed. Machines keep using the old version. |
| 10 | **Remove** on a box that a machine still uses | The dialog warns, lists the machine(s) with project/environment or folder and the last recorded state, and the button stays disabled until the box name is typed. **Cancel** — nothing runs. |
| 11 | **Remove** on a box version nothing uses, type its name, confirm | `vagrant box remove --force --no-tty --provider … --box-version … NAME` runs; only that version/provider disappears from the list. |
| 12 | Open **Operations** | Add/update/remove are listed as *Add box*, *Update box*, *Remove box* with target `Box · …`, and their logs are kept after a restart. |
