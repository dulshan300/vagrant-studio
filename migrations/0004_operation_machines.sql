-- The machines an operation named, so history can be shown per machine.
-- `operations.machine_name` only holds a single target; operations on
-- several machines left it empty. Operations that named no machine (status,
-- validate, box commands) have no rows here.

CREATE TABLE operation_machines (
    operation_id TEXT NOT NULL REFERENCES operations (id) ON DELETE CASCADE,
    machine_name TEXT NOT NULL,
    PRIMARY KEY (operation_id, machine_name)
);

CREATE INDEX idx_operation_machines_name ON operation_machines (machine_name);

-- Earlier operations on one machine; those on several can't be recovered.
INSERT INTO operation_machines (operation_id, machine_name)
SELECT id, machine_name FROM operations WHERE machine_name IS NOT NULL AND machine_name <> '';
