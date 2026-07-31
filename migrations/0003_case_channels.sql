-- Case chat channels ("threads"): a case's chat is no longer one flat thread but
-- a set of named channels, each with its own message list.
--
-- Two kinds exist, stored as the text `slug()` of `ChannelKind` so the Rust
-- `from_slug()` helper round-trips cleanly:
--
--   'standard'       an ordinary channel, visible to everyone who can view the
--                    case (clients included). Every case starts with exactly one
--                    ("General"); holders of the `manage_channels` capability can
--                    add and remove more.
--   'volunteer_only' the private back-channel for volunteers and admins. Exactly
--                    one per case, created automatically with the case, and
--                    never deletable — clients assigned to the case must never
--                    see it or its messages.
--
-- Ordering is by `ord` then `seq`: the volunteer-only channel is pinned first
-- (`ord` 0) for the staff who can see it, "General" next, and channels created
-- later fall in behind in creation order.

CREATE TABLE case_channels (
    id         TEXT PRIMARY KEY,
    case_id    TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    kind       TEXT NOT NULL DEFAULT 'standard',
    ord        INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Monotonic sequence so channel ordering is stable across renames.
    seq        BIGSERIAL
);

CREATE INDEX case_channels_case_id_idx ON case_channels(case_id);

-- Channel names are unique per case, case-insensitively, so the picker never
-- shows two indistinguishable entries.
CREATE UNIQUE INDEX case_channels_case_name_idx ON case_channels(case_id, lower(name));

-- At most one volunteer-only channel per case. Enforced in the database rather
-- than only in application code because the "is this the protected channel?"
-- check that guards deletion assumes it is unique.
CREATE UNIQUE INDEX case_channels_one_volunteer_only_idx
    ON case_channels(case_id)
    WHERE kind = 'volunteer_only';

-- Backfill: every existing case gets its volunteer-only channel and a "General"
-- channel. Ids come from the same `app_id_seq` the application uses via
-- `server::db::ids::next`, so migrated and runtime-created channels share one id
-- space and can never collide.
INSERT INTO case_channels (id, case_id, name, kind, ord)
SELECT 'ch-' || nextval('app_id_seq'), c.id, 'Volunteer only', 'volunteer_only', 0
FROM cases c;

INSERT INTO case_channels (id, case_id, name, kind, ord)
SELECT 'ch-' || nextval('app_id_seq'), c.id, 'General', 'standard', 1
FROM cases c;

-- Messages now belong to a channel. Existing messages are all case-wide history
-- that clients could already read, so they move into "General" rather than the
-- private volunteer channel.
ALTER TABLE messages ADD COLUMN channel_id TEXT REFERENCES case_channels(id) ON DELETE CASCADE;

UPDATE messages m
SET channel_id = ch.id
FROM case_channels ch
WHERE ch.case_id = m.case_id AND ch.kind = 'standard' AND ch.name = 'General';

ALTER TABLE messages ALTER COLUMN channel_id SET NOT NULL;

CREATE INDEX messages_channel_id_idx ON messages(channel_id);

-- Backfill the new `manage_channels` capability onto every existing "Full
-- access" assignment.
--
-- `manage_channels` is defined (see `CaseCapability::requires`) to demand the
-- complete capability set, so it belongs to exactly the users who already hold
-- all seven pre-existing capabilities on a case — case owners, and anyone an
-- admin gave the Manager preset. Without this they would silently drop out of
-- the "Full access" group the moment the capability was introduced, leaving
-- nobody able to manage a migrated case's channels.
INSERT INTO case_assignments (user_id, case_id, capability)
SELECT a.user_id, a.case_id, 'manage_channels'
FROM case_assignments a
WHERE a.capability IN (
    'view_case', 'edit_case', 'add_notes', 'view_evidence',
    'upload_evidence', 'delete_evidence', 'send_messages'
)
GROUP BY a.user_id, a.case_id
HAVING count(DISTINCT a.capability) = 7
ON CONFLICT DO NOTHING;
