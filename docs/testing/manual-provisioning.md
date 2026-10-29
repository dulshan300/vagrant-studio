# Manual test: shell provisioners (Phase 6)

Run on Windows 11 with the local build (`pnpm build:win-local`). Use a test
environment with two machines you can recreate (VMware Desktop or VirtualBox).

## Named provisioners in the editor
1. Open the environment → **Vagrantfile** → **Visual**. In the shared settings
   add a shell provisioner named `hello` with the inline script
   `echo hello from $(hostname)`.
2. In the first machine add a provisioner named `fail`, set to **Run only
   when chosen by name**, with the script `echo about to fail` / `exit 3`.
3. Add an unnamed provisioner set to *Run only when chosen by name*.
   Expected: a hint says to give it a name.
4. Type `a,b` as a name. Expected: an error under the row; Save is blocked.
   Give a machine provisioner the same name as the shared one (`hello`).
   Expected: an error. Fix both.
5. Save with "Validate after saving". Expected: the diff shows
   `config.vm.provision "hello", type: "shell", …`; Vagrant validates it.

## Running provisioners
1. On **Machines**, start both machines. Expected: each row shows
   "N provisioners ran · just now". Click it. Expected: the operation drawer
   lists the provisioners per machine with their output.
2. Select the first machine → **Provision**. Expected: a dialog offers the
   named provisioners (`hello` shared, `fail` "only when chosen"), says the
   unnamed ones can't be chosen alone, and offers *Restart, then provision*.
3. Choose **Only these** → `fail` → **Run**. Expected: the command line is
   `vagrant provision --provision-with fail --no-tty <machine>`; the
   operation fails; the drawer shows `“fail” Failed` opened with
   `about to fail`; the machine row shows "“fail” failed" in red.
4. **Provision** → *All provisioners* → *Restart, then provision* → **Run**.
   Expected: `vagrant reload --provision --no-tty <machine>`; `fail` doesn't
   run; the row shows success again.
5. Stop the first machine, select it → **Provision**. Expected: the button is
   enabled and the dialog offers *Start, then provision* and *Restart, then
   provision*, not plain provisioning.
6. Select both machines → **Provision**. Expected: only `hello` (defined for
   both) can be chosen. Run it. Expected: one result per machine.
7. Cancel a provision while a long script runs (e.g. `sleep 60`). Expected:
   that provisioner shows *Interrupted*.

Record results in `docs/implementation-status.md`.
