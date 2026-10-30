# Manual test: embedded terminal (Phase 7)

Run on Windows 11 with the local build (`pnpm build:win-local`). Use an
environment with two machines (`web`, `db`), both running, and refresh its
status first. Keep Task Manager (Details tab) open for step 7.

1. Settings: "SSH opens in" shows "The app's terminal panel". Leave it.
2. Click the SSH icon on `web`. Expected: a panel opens at the bottom with a
   tab "ssh web" (spinner, then a green dot), and the guest's shell prompt
   appears. Type `uname -a`, `top` (then `q`), and `ls --color`. Expected:
   output, colours and full-screen redraw look right; arrow keys, Tab
   completion and Ctrl+C work.
3. Drag the panel's top edge up and down, and resize the window. Expected:
   the terminal reflows; `tput cols; tput lines` matches the visible size.
4. Open SSH on `db` too. Expected: a second tab; switching tabs keeps each
   session's scrollback. Navigate to another page. Expected: the panel and
   both sessions stay.
5. Press Ctrl+` (or the chevron). Expected: the panel hides, the sidebar
   shows "Terminals 2/2"; clicking it (or Ctrl+` again) brings it back.
6. In the `web` tab type `exit`. Expected: "[session ended with exit code 0]",
   grey dot, the tab stays until closed.
7. In the `db` tab, close it with its × while connected. Expected: in Task
   Manager no `vagrant.exe`, `ruby.exe` or `ssh.exe` from that session is
   left after a few seconds.
8. Stop `web` and click SSH on a machine that isn't running (refresh the
   status first if the icon is still shown). Expected: the tab shows why it
   couldn't start instead of a terminal.
9. Open a session, then quit the app. Expected: no `vagrant.exe`, `ruby.exe`
   or `ssh.exe` left in Task Manager.
10. Settings: choose "A new terminal window", save, click SSH. Expected:
    Windows Terminal (or a console) opens as before.

Report per step, or "all pass".
