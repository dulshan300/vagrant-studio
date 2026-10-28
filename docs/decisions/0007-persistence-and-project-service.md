# ADR-0007: SQLite persistence and the project service layer

- Status: Accepted
- Date: 2026-10-28

## Context
Phase 2 adds local persistence and the first features that touch user files
(create from template, import, delete files, write project.json).

## Decision
- **SQLx 0.9 + SQLite** (bundled, without extension loading) in
  `crates/project-store`. Migrations live in `/migrations` and are embedded
  with `sqlx::migrate!`. The database is `vagrant-studio.db` in the OS app
  data folder, opened in WAL mode with foreign keys on. The workspace MSRV is
  1.94 because SQLx 0.9 requires it.
- **Queries are static SQL strings with bound parameters.** SQLx 0.9 only
  accepts `&'static str` without an explicit `AssertSqlSafe`; no
  `AssertSqlSafe` is used.
- **Timestamps** are UTC, truncated to milliseconds and stored as RFC 3339
  text. Because fractional digits vary, ordering is done on parsed values,
  never on strings.
- **Persistence test double = a real in-memory SQLite database**, not a mock
  trait. This exercises the SQL, constraints and migrations. Business logic
  that must not touch the real world gets injected seams instead (for
  example, the trash function in `ProjectService::with_trash`).
- **`crates/project-service`** holds the project use cases and is the only
  layer that writes to project folders. Its safety rules:
  - inspect and import never write;
  - create only uses a new or empty folder and creates files with `create_new`;
  - file deletion goes to the **OS trash** (never a permanent delete), requires
    the typed project name, refuses top-level and home folders, and refuses
    while `.vagrant/machines/*/*/id` exists (machines may still exist);
  - `project.json` writes are atomic (temp file + rename) and need explicit
    overwrite after the user has seen the existing content.
- Paths are canonicalized with `dunce` (no `\\?\` prefixes on Windows), and
  non-UTF-8 paths are rejected rather than stored lossily.

## Consequences
- Repository code has no trait abstraction yet. If a second backend or
  heavy mocking becomes necessary, traits can be extracted from `Store`'s
  inherent methods without changing callers much.
- Deleting files depends on the OS trash being available. When it isn't,
  deletion fails and the metadata is kept.
