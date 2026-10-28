-- Operation history that outlives environments, plus host-level operations
-- (e.g. `vagrant box list`) that have no environment.
-- SQLite can't relax NOT NULL in place, so the table is rebuilt.

CREATE TABLE operations_v2 (
    id             TEXT PRIMARY KEY NOT NULL,
    project_id     TEXT REFERENCES projects (id) ON DELETE SET NULL,
    environment_id TEXT REFERENCES environments (id) ON DELETE SET NULL,
    machine_id     TEXT REFERENCES machine_configs (id) ON DELETE SET NULL,
    machine_name   TEXT,
    -- Human label captured at start (e.g. "Web Platform · default") so history
    -- stays readable after the project or environment is removed.
    target_label   TEXT NOT NULL DEFAULT '',
    operation_type TEXT NOT NULL,
    status         TEXT NOT NULL CHECK (status IN (
                       'queued', 'running', 'succeeded', 'failed',
                       'cancel_requested', 'cancelled', 'interrupted')),
    command_line   TEXT NOT NULL DEFAULT '',
    working_dir    TEXT,
    started_at     TEXT,
    finished_at    TEXT,
    exit_code      INTEGER,
    log_path       TEXT,
    error_summary  TEXT,
    created_at     TEXT NOT NULL
);

INSERT INTO operations_v2 (id, environment_id, machine_id, operation_type, status, started_at,
                           finished_at, exit_code, log_path, error_summary, created_at)
SELECT id, environment_id, machine_id, operation_type, status, started_at,
       finished_at, exit_code, log_path, error_summary, created_at
FROM operations;

DROP TABLE operations;
ALTER TABLE operations_v2 RENAME TO operations;

CREATE INDEX idx_operations_created ON operations (created_at);
CREATE INDEX idx_operations_environment ON operations (environment_id);
CREATE INDEX idx_operations_status ON operations (status);

-- Last observed `vagrant status` result per environment (a cache with a
-- timestamp; Vagrant stays authoritative).
CREATE TABLE environment_status (
    environment_id TEXT PRIMARY KEY NOT NULL REFERENCES environments (id) ON DELETE CASCADE,
    operation_id   TEXT,
    observed_at    TEXT NOT NULL,
    machines_json  TEXT NOT NULL DEFAULT '[]',
    error_class    TEXT,
    error_message  TEXT
);
