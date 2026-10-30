# Manual test: log navigation (Phase 7)

Run on Windows 11 with the local build (`pnpm build:win-local`). Use a
project with an environment that has at least two machines (`web`, `db`)
and some operation history. The database is upgraded to schema 4 on start;
operations on several machines recorded before this build don't show up
under a single machine (operations on one machine do).

1. On a machine row, click the history (clock) icon next to the status.
   Expected: the Operations page opens with project, environment and machine
   selected, listing only commands that named that machine.
2. Start `web` and `db` together (select both, Start). Expected: the
   operation shows under machine `web` and under machine `db`, but not
   under a third machine. Status refreshes and validation appear only
   under "All machines".
3. Change the selectors: environment to "All environments", then project
   to "All projects and host commands". Expected: the list widens each
   time, and the URL-driven scope survives Reload. "Clear" shows
   everything again.
4. From the project page's ⋯ menu choose "Operation history", and from an
   environment card's ⋯ menu choose "Operation history". Expected: the
   list is scoped to that project or environment.
5. Open an operation with a long output (a Start with provisioning). Type a
   word that appears several times into "Search output". Expected: matches
   highlighted, "1 of N", the first match scrolled into view; Enter and
   Shift+Enter (and the arrows) step forward and back, wrapping around.
   Ctrl+F inside the drawer focuses the search box; Escape clears the
   search first, then a second Escape closes the drawer.
6. Click "Problems". Expected: only stderr lines and lines that mention
   errors or warnings; warnings in yellow, errors in red. Click "stderr":
   only Vagrant's stderr. Back to "All", click "Next problem" repeatedly.
   Expected: it jumps between those lines.
7. Click the copy icon and paste into an editor. Expected: the lines shown
   (respecting the current filter), as displayed.
8. Click the file icon. Expected: Explorer opens the logs folder with the
   operation's `.log` file selected.
9. On the Operations page click "Logs folder". Expected: Explorer opens
   `%APPDATA%\dev.vagrantstudio.desktop\logs\operations`.
10. While an operation is running, open its drawer and search. Expected:
    new output keeps arriving, but the view stays on your match instead of
    jumping to the end; clear the search to follow the output again.

Report per step, or "all pass".
