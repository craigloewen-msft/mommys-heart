-- Add the existing information-area grant to the administrative approval flow.
ALTER TABLE admin_requests
    ADD COLUMN current_information_access BOOLEAN,
    ADD COLUMN requested_information_access BOOLEAN;

ALTER TABLE admin_requests
    DROP CONSTRAINT admin_requests_kind_check,
    DROP CONSTRAINT admin_requests_check;

ALTER TABLE admin_requests
    ADD CONSTRAINT admin_requests_kind_check
        CHECK (kind IN ('role', 'case_capabilities', 'information_access')),
    ADD CONSTRAINT admin_requests_shape_check CHECK (
        (kind = 'role'
         AND case_id IS NULL
         AND previous_role IS NOT NULL
         AND requested_role IS NOT NULL
         AND current_capabilities IS NULL
         AND requested_capabilities IS NULL
         AND current_information_access IS NULL
         AND requested_information_access IS NULL)
        OR
        (kind = 'case_capabilities'
         AND case_id IS NOT NULL
         AND previous_role IS NULL
         AND requested_role IS NULL
         AND current_information_access IS NULL
         AND requested_information_access IS NULL)
        OR
        (kind = 'information_access'
         AND case_id IS NULL
         AND previous_role IS NULL
         AND requested_role IS NULL
         AND current_capabilities IS NULL
         AND requested_capabilities IS NULL
         AND current_information_access IS NOT NULL
         AND requested_information_access IS NOT NULL)
    );

CREATE UNIQUE INDEX admin_requests_pending_information_target_idx
    ON admin_requests(target_user_id)
    WHERE status = 'pending' AND kind = 'information_access';
