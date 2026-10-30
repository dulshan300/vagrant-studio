# Manual test: host port warnings across projects (Phase 7)

Run on Windows 11 with the local build (`pnpm build:win-local`). Use two
projects (A and B) whose Vagrantfiles the visual editor can read.

1. In project A, give a machine a forwarded port: guest 80 → host 8090,
   no host IP. Save.
2. Open project B's Vagrantfile, add the same host port 8090 to one of its
   machines. Expected, before saving: under that port, "Also forwarded by
   A · <environment> (<machine>). Only one of them can be running at a
   time." Save.
3. Open project B's page. Expected: a yellow strip on its environment card
   names port 8090, the machine, and A as a link. Click it. Expected:
   project A's page shows the same warning naming B.
4. In project B change the host IP to `127.0.0.1` and A's to `127.0.0.2`.
   Expected: no warning (different addresses). Put A back to no host IP.
   Expected: the warning returns (no host IP means every address).
5. Change B's port to 8091. Expected: the warnings disappear on both pages.
6. Optional: start A's machine, then try to start B's with 8090. Expected:
   Vagrant reports the port collision, as the warning predicted.

Report per step, or "all pass".
