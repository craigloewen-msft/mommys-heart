-- Split the former all-powerful admin role into site and operations admins.
-- Existing administrators retain all of their access as site admins.
UPDATE users SET role = 'site_admin' WHERE role = 'admin';

-- Approval queue for operations-admin requests. One row represents either a
-- global role change or one case's complete capability-set replacement.
-- `current_*` snapshots make approval fail safely if a site admin changed the
-- target after the request was filed. NULL capabilities mean "not assigned".
CREATE TABLE admin_requests (
    id                       TEXT PRIMARY KEY,
    kind                     TEXT NOT NULL CHECK (kind IN ('role', 'case_capabilities')),
    status                   TEXT NOT NULL DEFAULT 'pending'
                                  CHECK (status IN ('pending', 'approved', 'denied')),
    requested_by             TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    target_user_id           TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    case_id                  TEXT REFERENCES cases(id) ON DELETE CASCADE,
    previous_role            TEXT,
    requested_role           TEXT,
    current_capabilities     TEXT[],
    requested_capabilities   TEXT[],
    request_note             TEXT NOT NULL DEFAULT '',
    decided_by               TEXT REFERENCES users(id) ON DELETE SET NULL,
    decision_note            TEXT NOT NULL DEFAULT '',
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_at               TIMESTAMPTZ,
    seq                      BIGSERIAL,
    CHECK (
        (kind = 'role'
         AND case_id IS NULL
         AND previous_role IS NOT NULL
         AND requested_role IS NOT NULL
         AND current_capabilities IS NULL
         AND requested_capabilities IS NULL)
        OR
        (kind = 'case_capabilities'
         AND case_id IS NOT NULL
         AND previous_role IS NULL
         AND requested_role IS NULL)
    )
);

CREATE INDEX admin_requests_requester_idx
    ON admin_requests(requested_by, seq DESC);
CREATE INDEX admin_requests_pending_idx
    ON admin_requests(seq DESC) WHERE status = 'pending';

-- Avoid conflicting duplicate work while still allowing a new request after a
-- prior one is resolved.
CREATE UNIQUE INDEX admin_requests_pending_role_target_idx
    ON admin_requests(target_user_id)
    WHERE status = 'pending' AND kind = 'role';
CREATE UNIQUE INDEX admin_requests_pending_case_target_idx
    ON admin_requests(target_user_id, case_id)
    WHERE status = 'pending' AND kind = 'case_capabilities';