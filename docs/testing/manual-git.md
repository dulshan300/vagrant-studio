# Manual test: Git on the project page (Phase 7)

Run on Windows 11 with the local build (`pnpm build:win-local`) and Git for
Windows installed. Vagrant Studio never pushes, pulls, discards changes or
rewrites history.

## New project
1. **New project** → empty folder or a template. Expected: *Initialize a Git
   repository* is ticked. Create.
2. Expected: the header shows the branch and "N changed"; the **Version
   control** panel lists the Vagrantfile and `.gitignore` as new files.
3. Open `.gitignore` in the folder. Expected: `.vagrant/`, `*.log`, `*.box`.

## Existing project
4. Open a project that isn't a repository. Expected: the panel says so and
   offers **Initialize Git**; confirm. Expected: "Repository created…".
5. Optional: add your own `.gitignore` first, then initialize. Expected:
   "The existing .gitignore was kept", and the file is unchanged.

## Stage and commit
6. Tick one file. Expected: it is staged (checkbox ticked); **Commit 1
   change** becomes available once a message is typed.
7. Untick it. Expected: unstaged; the file itself is unchanged.
8. **Stage all**, type a message, **Commit** (or Ctrl+Enter). Expected: the
   change list empties, the history shows the commit with your Git name,
   the header says "clean".
   If Git doesn't know your name and email yet, the error explains the two
   `git config --global` commands to run.
9. Edit a file in the folder (outside the app) → **Refresh**. Expected: it
   appears as modified (M).
10. Click the header's Git badge. Expected: the page scrolls to the panel.

Report per step, or "all pass".
