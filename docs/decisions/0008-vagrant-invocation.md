# ADR-0008: How Vagrant is invoked

- Status: Accepted
- Date: 2026-10-28

## Context
Phase 3 runs the host's Vagrant CLI from a GUI application on Windows and
Linux. The behaviour below was checked against Vagrant 2.4.9 on Windows 11
(with VirtualBox 7.1.18 and VMware Workstation 25.0.0 + vagrant-vmware-desktop
3.0.5); raw output is kept in `fixtures/command-output/windows-vagrant-2.4.9/`.

## Decision
**Process**
- `process-wrap` spawns Vagrant with an argv vector (never a shell), stdin
  closed, stdout/stderr piped, and an explicit working directory (the
  environment folder).
- Windows: `CREATE_NO_WINDOW` (no console window flashes from the GUI) and a
  **Job Object**, so cancelling or timing out kills `vagrant.exe` and its Ruby
  child processes together. Console-less processes can't receive Ctrl+C, so
  Windows cancellation is an immediate tree kill.
- Linux: the child leads its own **process group**. Cancellation sends SIGINT
  (Vagrant cleans up), then kills the group after 10 seconds.
- `VAGRANT_CHECKPOINT_DISABLE=1` is always set (offline-first: no version-check
  network calls).
- `VAGRANT_DEFAULT_PROVIDER` is set from the environment's provider hint, so
  machines that don't exist yet are reported for the intended provider
  (`status` has no `--provider` flag).

**Flags**
- Parsed commands use `--machine-readable`.
- **`--no-tty` is never combined with `--machine-readable`.** Both select
  Vagrant's UI and the last one wins: `status --machine-readable --no-tty`
  prints human text. `--no-tty` is used for `validate` and lifecycle commands.
- `destroy` always gets `--force`; the confirmation happens in the UI first
  and is checked by the backend (one-time token, architecture.md §9).
- Lifecycle commands always name their machines (`vagrant up --no-tty web db`):
  without names, `up` skips `autostart: false` machines. Names are passed in
  Vagrantfile order; Vagrant reverses it for `destroy` itself.
- `up` gets no `--provider` flag: it fails for machines already created with
  another provider. `VAGRANT_DEFAULT_PROVIDER` selects the provider for new
  machines instead (Phase 4).
- Machine names are validated (`[A-Za-z0-9._-]`, not starting with `-`), so a
  name can never be read as an option.

**Output**
- Lines split on `\n`, `\r\n` and bare `\r` (progress updates); invalid UTF-8
  is decoded lossily.
- Machine-readable unescaping: `%!(VAGRANT_COMMA)` → `,`, a literal `\n` →
  newline. Unknown record types are ignored. Failures come from `error-exit`
  records (class + message); otherwise the last stderr lines are used.
- Output is redacted (assignments of password/secret/token/key variables,
  URL credentials, bearer tokens, private-key blocks) before it reaches log
  files or events.

**Operations**
- Every invocation started by the user is an operation row plus a log file
  (`<app data>/logs/operations/<id>.log`). An operation succeeds only on exit
  code 0. Cancelled and timed-out commands are recorded as such and never
  treated as a rollback. Unfinished operations become `interrupted` at the
  next startup.
- Host readiness checks (Vagrant version, plugins, provider prerequisites)
  are not recorded as operations. The VMware Utility check is a TCP connect
  to `127.0.0.1:9922`, which needs no admin rights. Nothing is installed or
  started on the user's behalf.

## Consequences
- Fixtures must be refreshed when supporting a new Vagrant version.
- Real-Vagrant tests are opt-in: `cargo test -p vagrant-studio-vagrant-adapter --
  --ignored`, with `VAGRANT_STUDIO_TEST_PROVIDER` selecting the provider.
- Log retention isn't implemented yet (release hardening).
