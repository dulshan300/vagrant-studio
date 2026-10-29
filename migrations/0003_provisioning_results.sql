-- Provisioner runs read from the output of `up`, `reload` and `provision`:
-- one row per machine and operation, with the runs in order as JSON.
-- `seq` orders rows by insertion, so the latest per machine needs no
-- timestamp comparison.

CREATE TABLE provisioning_results (
    seq            INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id   TEXT NOT NULL REFERENCES operations (id) ON DELETE CASCADE,
    -- Kept (as NULL) when the environment is removed, like the operation history.
    environment_id TEXT REFERENCES environments (id) ON DELETE SET NULL,
    machine_name   TEXT NOT NULL,
    finished_at    TEXT NOT NULL,
    runs_json      TEXT NOT NULL DEFAULT '[]',
    UNIQUE (operation_id, machine_name)
);

CREATE INDEX idx_provisioning_environment ON provisioning_results (environment_id, machine_name);
