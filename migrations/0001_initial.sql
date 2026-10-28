-- Vagrant Studio schema v1.
-- IDs are UUIDv7 strings; timestamps are RFC 3339 text (UTC).
-- Machine runtime state is NOT stored here as truth: Vagrant is authoritative.

CREATE TABLE projects (
    id             TEXT PRIMARY KEY NOT NULL,
    name           TEXT NOT NULL,
    slug           TEXT NOT NULL,
    description    TEXT NOT NULL DEFAULT '',
    root_path      TEXT NOT NULL UNIQUE,
    repository_url TEXT,
    status         TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'archived')),
    favorite       INTEGER NOT NULL DEFAULT 0 CHECK (favorite IN (0, 1)),
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    archived_at    TEXT,
    last_opened_at TEXT
);
CREATE INDEX idx_projects_status ON projects (status);
CREATE INDEX idx_projects_last_opened ON projects (last_opened_at);

CREATE TABLE environments (
    id               TEXT PRIMARY KEY NOT NULL,
    project_id       TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    name             TEXT NOT NULL,
    root_path        TEXT NOT NULL,
    vagrantfile_path TEXT,
    default_provider TEXT,
    environment_type TEXT NOT NULL DEFAULT 'development'
        CHECK (environment_type IN ('development', 'testing', 'staging', 'other')),
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    UNIQUE (project_id, name COLLATE NOCASE)
);
CREATE INDEX idx_environments_project ON environments (project_id);

-- Application-managed configuration or cached metadata; never the actual VM state.
CREATE TABLE machine_configs (
    id             TEXT PRIMARY KEY NOT NULL,
    environment_id TEXT NOT NULL REFERENCES environments (id) ON DELETE CASCADE,
    machine_name   TEXT NOT NULL,
    box_name       TEXT,
    provider_hint  TEXT,
    cpu            INTEGER CHECK (cpu IS NULL OR cpu > 0),
    memory_mb      INTEGER CHECK (memory_mb IS NULL OR memory_mb > 0),
    autostart      INTEGER NOT NULL DEFAULT 0 CHECK (autostart IN (0, 1)),
    sort_order     INTEGER NOT NULL DEFAULT 0,
    UNIQUE (environment_id, machine_name)
);
CREATE INDEX idx_machine_configs_environment ON machine_configs (environment_id);

CREATE TABLE operations (
    id             TEXT PRIMARY KEY NOT NULL,
    environment_id TEXT NOT NULL REFERENCES environments (id) ON DELETE CASCADE,
    machine_id     TEXT REFERENCES machine_configs (id) ON DELETE SET NULL,
    operation_type TEXT NOT NULL,
    status         TEXT NOT NULL CHECK (status IN (
                       'queued', 'running', 'succeeded', 'failed',
                       'cancel_requested', 'cancelled', 'interrupted')),
    started_at     TEXT,
    finished_at    TEXT,
    exit_code      INTEGER,
    log_path       TEXT,
    error_summary  TEXT,
    created_at     TEXT NOT NULL
);
CREATE INDEX idx_operations_environment ON operations (environment_id, created_at);
CREATE INDEX idx_operations_status ON operations (status);

CREATE TABLE port_mappings (
    id         TEXT PRIMARY KEY NOT NULL,
    machine_id TEXT NOT NULL REFERENCES machine_configs (id) ON DELETE CASCADE,
    guest_port INTEGER NOT NULL CHECK (guest_port BETWEEN 1 AND 65535),
    host_port  INTEGER NOT NULL CHECK (host_port BETWEEN 1 AND 65535),
    protocol   TEXT NOT NULL DEFAULT 'tcp' CHECK (protocol IN ('tcp', 'udp')),
    host_ip    TEXT,
    guest_ip   TEXT
);
CREATE INDEX idx_port_mappings_machine ON port_mappings (machine_id);
CREATE INDEX idx_port_mappings_host_port ON port_mappings (host_port, protocol);

CREATE TABLE synced_folders (
    id          TEXT PRIMARY KEY NOT NULL,
    machine_id  TEXT NOT NULL REFERENCES machine_configs (id) ON DELETE CASCADE,
    host_path   TEXT NOT NULL,
    guest_path  TEXT NOT NULL,
    folder_type TEXT,
    disabled    INTEGER NOT NULL DEFAULT 0 CHECK (disabled IN (0, 1))
);
CREATE INDEX idx_synced_folders_machine ON synced_folders (machine_id);

CREATE TABLE provisioners (
    id               TEXT PRIMARY KEY NOT NULL,
    machine_id       TEXT NOT NULL REFERENCES machine_configs (id) ON DELETE CASCADE,
    name             TEXT NOT NULL,
    provisioner_type TEXT NOT NULL,
    config_json      TEXT NOT NULL DEFAULT '{}',
    run              TEXT NOT NULL DEFAULT 'once' CHECK (run IN ('once', 'always', 'never')),
    sort_order       INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_provisioners_machine ON provisioners (machine_id);

CREATE TABLE project_tasks (
    id           TEXT PRIMARY KEY NOT NULL,
    project_id   TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    title        TEXT NOT NULL,
    done         INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1)),
    sort_order   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    completed_at TEXT
);
CREATE INDEX idx_project_tasks_project ON project_tasks (project_id);

CREATE TABLE project_notes (
    id         TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    title      TEXT NOT NULL,
    body       TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_project_notes_project ON project_notes (project_id);

CREATE TABLE tags (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE TABLE project_tags (
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    tag_id     INTEGER NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (project_id, tag_id)
);
CREATE INDEX idx_project_tags_tag ON project_tags (tag_id);

CREATE TABLE app_settings (
    key        TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
