# Manual Vagrantfile editor test plan (Phase 5)

Run on Windows with a fresh `pnpm build:win-local` build. Record the result in
`docs/implementation-status.md`. No VM needs to start: the checks use
`vagrant validate` and file contents only.

The automated tests cover the recognizer, the generator, save safety
(stale file, replace confirmation, backups, atomic write) and the page flow
with mocked services. This plan covers the real WebView, CodeMirror, dialogs,
the window close guard and Windows files.

## Setup

1. Create a project from the **web-database** template with VMware Desktop
   (or VirtualBox).
2. Open the project. The environment row shows a **Vagrantfile** button.

## Steps

| # | Action | Expected |
|---|---|---|
| 1 | Click **Vagrantfile** | The page opens on **Visual**, with the badge *Hand-written; the visual editor can show it*. The machine list shows *Shared settings*, `web` (primary) and `db`. **Review and save** is disabled. |
| 2 | Select `web`, set CPUs to 2, add a forwarded port | *Unsaved changes* appears. Switch to **Raw**: the text has the visual editor's header, `v.vmx["numvcpus"] = "2"` and the new port. |
| 3 | Back in **Visual**, set `db`'s host port to the same value as `web`'s | The port shows an error, the machine gets a red dot, and Save is disabled. Switching to Raw is refused with a message. Undo the change. |
| 4 | Click **Review and save** | The diff shows the changes inline. Because the template file wasn't written by the visual editor, a *Replace this hand-written Vagrantfile* checkbox is required. Save with *Check it with vagrant validate* ticked. |
| 5 | After saving | The badge says *Written by the visual editor*, and the validate operation succeeds in the drawer. Open the file in Notepad: the content matches. |
| 6 | In **Raw**, change the hostname by hand, use Ctrl+F to find `apt-get` and replace one occurrence, then press Ctrl+S | The diff dialog opens. Save works without the replace checkbox. |
| 7 | In **Raw**, add `config.vm.box_check_update = false` | The status line says visual editing is unavailable and links to the line. **Visual** explains why and **Line N** jumps to it. Save the file. Reopening the page lands on **Raw**, with the badge *raw editing only*. Remove the line again and save. |
| 8 | Edit in the app without saving, change the Vagrantfile in Notepad and save it there, then click **Review and save** → Save | A banner says the file changed on disk and nothing was saved. **Keep my changes and compare again**, then Save: the diff is now against Notepad's version. |
| 9 | **Backups** | The list shows one backup per save (newest first), stored under `%APPDATA%\dev.vagrantstudio.desktop\backups\<environment id>\`. **Load into editor** on the oldest puts the original template text in Raw. Save it to restore. |
| 10 | Make an edit, then click **Projects** in the sidebar | *Discard unsaved changes?* appears. **Keep editing** stays on the page. Repeat with **Discard changes**, which leaves the page. |
| 11 | Make an edit, then close the window | The same dialog appears. **Keep editing** keeps the app open. **Discard changes** closes it. |
| 12 | Start `web` (Phase 4 actions) and while it runs, try to save an edit | Refused: *A Vagrant operation is queued or running…*. After it finishes, the save works. |
| 13 | Add an environment for an empty subfolder, then click **Create Vagrantfile** | A starter file (box `bento/ubuntu-24.04`, the environment's provider) appears in Visual. **Review and create** writes it. The project page now enables Refresh status and Validate for that environment. |
| 14 | Convert the Vagrantfile to CRLF line endings (for example with VS Code's status bar), reopen the page and save a visual edit | The file still has CRLF line endings, and `vagrant validate` passes. |
| 15 | Switch the app to dark theme | Editor, syntax colors and the diff stay readable. |
