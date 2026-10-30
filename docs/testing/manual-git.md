# Manual test: Git status on the project page (Phase 7)

Run on Windows 11 with the local build (`pnpm build:win-local`) and Git for
Windows installed.

1. Open a project whose folder is not a Git repository. Expected: no Git
   badge in the header; nothing else changes.
2. In that folder run `git init` (and commit the Vagrantfile). Open the
   project again. Expected: the badge shows the branch and "clean".
3. Edit a file in the folder and add a new one. Click the badge. Expected:
   "2 changed"; the tooltip lists 1 modified, 1 untracked.
4. Stage one change (`git add`). Click the badge. Expected: the tooltip
   shows 1 staged.
5. Optional, with a remote: commit without pushing. Expected: "↑1"; the
   tooltip names the upstream.
6. Optional: `git checkout <commit>` (detached). Expected: "detached at …".

Vagrant Studio only reads status; it never runs a Git command that changes
the repository.

Report per step, or "all pass".
