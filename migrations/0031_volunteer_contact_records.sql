-- ---------------------------------------------------------------------------
-- Every volunteer account gets a contact record carrying the `volunteer` type.
--
-- At runtime `users::set_role_in` now does this whenever an account becomes a
-- volunteer. This backfills the accounts that became volunteers before that
-- existed: a client approved as a volunteer kept a contact typed `client` only,
-- and an account created outside registration may have had no contact at all.
-- ---------------------------------------------------------------------------

-- Volunteers with no contact record yet. `id` mirrors the user id, exactly as
-- migration 0019's backfill does, so re-running is a no-op.
INSERT INTO contacts (id, first_name, last_name, email, phone, address, user_id, types, source)
SELECT
    'ct-' || u.id,
    u.first_name,
    u.last_name,
    u.email,
    u.phone,
    u.home_address,
    u.id,
    ARRAY['volunteer'],
    'Volunteer backfill'
FROM users u
WHERE u.role = 'volunteer'
  -- A user with no surname and no employer would violate contacts_named_check.
  AND btrim(u.last_name) <> ''
  AND NOT EXISTS (SELECT 1 FROM contacts c WHERE c.user_id = u.id)
ON CONFLICT DO NOTHING;

-- Volunteers whose contact exists but is not typed as one. A full type list is
-- left alone rather than dropping a classification someone chose deliberately.
UPDATE contacts c
SET types = array_append(c.types, 'volunteer'),
    updated_at = now()
FROM users u
WHERE u.id = c.user_id
  AND u.role = 'volunteer'
  AND NOT ('volunteer' = ANY(c.types))
  AND cardinality(c.types) < 6;
