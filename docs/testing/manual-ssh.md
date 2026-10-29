# Manual test: SSH in an external terminal (Phase 7)

Run on Windows 11 with the local build (`pnpm build:win-local`). Use an
environment with two machines you can recreate (VirtualBox or VMware Desktop).

1. On **Machines**, with every machine stopped. Expected: no terminal button
   on any row.
2. Start one machine. Expected: once it shows *Running*, its row has a
   terminal button (tooltip "vagrant ssh: …"); the stopped machine has none.
3. Click it. Expected: a new Windows Terminal window titled
   `vagrant ssh <machine>` opens and logs in (`vagrant@<machine>:~$`). Run
   `hostname` and `pwd`, then `exit`. Expected: the window closes.
4. Click it twice. Expected: two independent sessions. The Operations page
   shows no new operations for them.
5. While a session is open, run **Reload** on that machine. Expected: the
   reload runs (SSH holds no lock); the session is disconnected and its
   window stays open with Vagrant's error.
6. In a terminal, `vagrant halt <machine>` outside the app, then click the
   terminal button again without refreshing. Expected: the terminal opens and
   Vagrant says the machine isn't running; the window stays open. Refresh:
   the button disappears.
7. Optional: a project folder whose path contains a space, e.g.
   `C:\Users\<you>\vs ssh test`. Expected: step 3 works.

Report per step, or "all pass".
