# ADR-0009: Embedded terminal for `vagrant ssh`

- Status: Accepted
- Date: 2026-10-30

## Context
Phase 7 first opened `vagrant ssh` in an external terminal window
(ADR-0008). The roadmap then called for an embedded terminal, evaluating
portable-pty and xterm.js. `vagrant ssh` needs a real terminal (it runs
`ssh` with a TTY), so it can't run through the adapter's `Runner`, which
pipes output and closes stdin.

## Decision
- **Backend: `portable-pty` 0.9** (from WezTerm) in a new crate,
  `crates/terminal`. It wraps ConPTY on Windows and a Unix PTY on Linux and
  spawns an argv vector (no shell) with an explicit working directory (the
  environment root). The argv comes from the adapter's `ssh_args`, the
  checks (machine reported running by the last `vagrant status`) are shared
  with the external terminal.
- **Frontend: `@xterm/xterm` 6.0 + `@xterm/addon-fit` 0.11**, loaded only
  when a terminal opens. Sessions live in a panel docked under the page,
  with tabs; it survives navigation and hides with Ctrl+`.
- Output is decoded to text without splitting UTF-8 characters and streamed
  over a Tauri `Channel` per session (not a global event). Keyboard input
  and resizes go back through commands; input typed before the session id
  is known is queued, because **ConPTY starts by asking for the cursor
  position (`ESC[6n`) and waits for the answer**, which xterm.js gives.
- **Ending a session:** on Windows `portable-pty`'s kill is
  `TerminateProcess` on `vagrant.exe` only, so the session also closes the
  pseudo-console, which ends every process attached to it (Ruby, `ssh`). On
  Linux, closing the master sends SIGHUP to the session. ConPTY keeps the
  output pipe open after the program exits until the pseudo-console is
  closed, so a waiter thread closes it once the program ends. All sessions
  are closed when the app exits.
- Sessions are the user's, as with the external terminal: no operation row,
  no lock, nothing stored (output only lives in xterm.js's scrollback), at
  most 8 at once. `VAGRANT_CHECKPOINT_DISABLE=1` is set; on Linux
  `TERM=xterm-256color`.
- The external terminal stays available: Settings → "SSH opens in" chooses
  the panel (default) or a new terminal window.

## Consequences
- `crates/terminal` tests use real pseudo-terminals (`/bin/sh`, and
  `cmd.exe` under ConPTY as a Windows binary); they need no Vagrant.
- Whether Vagrant's child processes really end when a tab closes on
  Windows is checked in `docs/testing/manual-terminal.md`.
- A plain host shell in the project folder would reuse the same crate; it
  isn't offered yet.
