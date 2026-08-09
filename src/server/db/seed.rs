//! One-time database seed. Ports the in-memory fixtures from [`crate::mockdata`]
//! into PostgreSQL, hashing the demo passwords so the existing "Demo autofill"
//! logins keep working. Runs only when the database is empty.

use crate::server::auth::hash_password;
use crate::server::db::{case_folders, case_properties, channels, ids, messages, pool, users};
use crate::server_fns::audit::ChangeLogEntry;
use crate::server_fns::channels::{ChannelKind, DEFAULT_CHANNEL_NAME, VOLUNTEER_CHANNEL_NAME};
use crate::server_fns::users::{AccountRole, User};

/// Seed the database from the mock fixtures, but only if there are no users yet.
pub async fn seed_if_empty() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if users::count().await? > 0 {
        tracing::debug!("database already populated; skipping seed");
        return Ok(());
    }
    seed().await?;
    Ok(())
}

/// Force-refresh the demo/test data: wipe every domain table and re-insert the
/// mock fixtures. Invoked by `mommys-heart-crm seed`, which `etc/dev-db.sh` runs
/// once to bake the pre-seeded database image that every instance starts from.
/// Not used by the normal server startup path, which only seeds an empty
/// database via [`seed_if_empty`].
pub async fn reseed() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("wiping existing CRM data before reseeding");
    // One statement so FK constraints are satisfied atomically; RESTART IDENTITY
    // resets the audit_log sequence so ids are reproducible across reseeds.
    sqlx::query(
        "TRUNCATE users, sessions, grants, cases, case_properties, case_notes,
                  evidence, case_folders, case_channels, messages, case_assignments, audit_log
         RESTART IDENTITY CASCADE",
    )
    .execute(pool())
    .await?;
    seed().await?;
    Ok(())
}

async fn seed() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = pool();

    // 1. Users (hash the plaintext demo passwords) + their per-user audit log.
    for (u, password) in crate::mockdata::users() {
        let password_hash = hash_password(&password)?;
        sqlx::query(
            "INSERT INTO users (id, first_name, last_name, email, phone, home_address, password_hash, role)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&u.id)
        .bind(&u.first_name)
        .bind(&u.last_name)
        .bind(&u.email)
        .bind(&u.phone)
        .bind(&u.home_address)
        .bind(&password_hash)
        .bind(u.role.slug())
        .execute(pool)
        .await?;

        // The subtype record each role implies, so a fresh database satisfies
        // the same invariant `users::set_role_in` maintains at runtime.
        match u.role {
            AccountRole::Volunteer => {
                sqlx::query(
                    "INSERT INTO volunteers (user_id, status, agreement_version, decided_by_name)
                     VALUES ($1, 'approved', '', 'Seed')",
                )
                .bind(&u.id)
                .execute(pool)
                .await?;
            }
            AccountRole::Client => {
                sqlx::query("INSERT INTO clients (user_id) VALUES ($1)")
                    .bind(&u.id)
                    .execute(pool)
                    .await?;
            }
            _ => {}
        }

        insert_audit(&u.id, "user", &Vec::<ChangeLogEntry>::new()).await?;
    }

    // 2. Cases + their default chat channels, notes, evidence, properties, and
    //    case audit log.
    for c in crate::mockdata::cases() {
        sqlx::query(
            "INSERT INTO cases (id, name, status, review_reason, owner_id)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&c.id)
        .bind(&c.name)
        .bind(c.status.slug())
        .bind(&c.review_reason)
        .bind(&c.owner_id)
        .execute(pool)
        .await?;

        // Same two channels every runtime-created case gets: the permanent
        // volunteer-only back-channel plus "General".
        let mut conn = pool.acquire().await?;
        channels::create_defaults(&mut conn, &c.id).await?;
        drop(conn);

        // Seeded cases carry the same folders and intake/outtake fields a case
        // created through the app gets, so the demo data shows what a real case
        // actually looks like rather than a simplified version of one.
        let mut tx = pool.begin().await?;
        case_folders::create_for_new_case(&mut tx, &c.id).await?;
        case_properties::add_for_new_case(&mut tx, &c.id, c.properties.iter().cloned()).await?;
        tx.commit().await?;

        for n in &c.notes {
            sqlx::query(
                "INSERT INTO case_notes (id, case_id, author, body, created_at)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(&n.id)
            .bind(&c.id)
            .bind(&n.author)
            .bind(&n.body)
            .bind(&n.created_at)
            .execute(pool)
            .await?;
        }

        // The hand-written demo files are the client-facing kind, so they go in
        // a folder the client can see.
        let mut tx = pool.begin().await?;
        let folder = case_folders::find_by_path_in(&mut tx, &c.id, &["Supporting Documents"])
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;
        tx.commit().await?;
        for e in &c.evidence {
            sqlx::query(
                "INSERT INTO evidence
                    (id, case_id, name, uploaded_by, uploaded_at, description,
                     folder_id, visibility)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(&e.id)
            .bind(&c.id)
            .bind(&e.name)
            .bind(&e.uploaded_by)
            .bind(&e.uploaded_at)
            .bind(&e.description)
            .bind(&folder.id)
            .bind(folder.visibility.slug())
            .execute(pool)
            .await?;
        }

        insert_audit(&c.id, "case", &Vec::<ChangeLogEntry>::new()).await?;
    }

    // 3. Case assignments (after both users and cases exist).
    for (u, _) in crate::mockdata::users() {
        insert_assignments(&u).await?;
    }

    // 4. Case chat messages, routed into the channel each one belongs to.
    let mut channel_ids: std::collections::HashMap<(String, &'static str), String> =
        std::collections::HashMap::new();
    for sm in crate::mockdata::messages() {
        let m = &sm.message;
        let name = match sm.channel {
            ChannelKind::Standard => DEFAULT_CHANNEL_NAME,
            ChannelKind::VolunteerOnly => VOLUNTEER_CHANNEL_NAME,
        };
        let key = (m.case_id.clone(), name);
        let channel_id = match channel_ids.get(&key) {
            Some(id) => id.clone(),
            None => {
                let id = channels::id_for(&m.case_id, name)
                    .await?
                    .ok_or_else(|| format!("seed: case {} has no \"{name}\" channel", m.case_id))?;
                channel_ids.insert(key, id.clone());
                id
            }
        };
        messages::insert(
            &m.id,
            &m.case_id,
            &channel_id,
            &m.author_id,
            &m.author,
            &m.body,
            &m.sent_at,
        )
        .await?;
    }

    // 5. A handful of real audit-log entries, date-spread across the last ~6
    //    weeks, so the change-log views (which fetch the audit log as their own
    //    paginated, date-filtered data source) have data to show and page
    //    through in the demo. Real edits made in the running app append more.
    seed_audit_fixtures().await?;

    // 6. Advance the shared id sequence past every seeded id. Seed ids are
    //    `prefix-<n>` numbered per prefix from 1 (e.g. `m-1`..`m-13000`), and
    //    those counts can exceed the sequence's START value. Since `ids::next`
    //    hands out `<prefix>-<nextval>` from this one global sequence, leaving it
    //    below the largest seeded suffix makes the first newly-created record
    //    collide with seed data (e.g. `m-5001`) and violate the primary key.
    //    Bump it above the max suffix across every table that receives new ids.
    advance_id_sequence().await?;

    tracing::info!("database seeded from mock fixtures");
    Ok(())
}

/// Set `app_id_seq` above the largest numeric suffix of any seeded id so that
/// `ids::next` never reissues an id that already exists.
///
/// `users` is deliberately absent: user ids are random hex from `ids::opaque`,
/// not sequence-derived, so their suffix is not a number to compare against
/// (and casting one to `bigint` would error).
async fn advance_id_sequence() -> Result<(), sqlx::Error> {
    sqlx::query(
        "SELECT setval('app_id_seq', GREATEST(
             (SELECT COALESCE(max(split_part(id, '-', 2)::bigint), 0) FROM grants),
             (SELECT COALESCE(max(split_part(id, '-', 2)::bigint), 0) FROM cases),
             (SELECT COALESCE(max(split_part(id, '-', 2)::bigint), 0) FROM case_notes),
             (SELECT COALESCE(max(split_part(id, '-', 2)::bigint), 0) FROM evidence),
             (SELECT COALESCE(max(split_part(id, '-', 2)::bigint), 0) FROM case_channels),
             (SELECT COALESCE(max(split_part(id, '-', 2)::bigint), 0) FROM messages),
             (SELECT COALESCE(max(split_part(id, '-', 2)::bigint), 0) FROM audit_log)
         ) + 1, false)",
    )
    .execute(pool())
    .await?;
    Ok(())
}

/// A single seeded audit entry: how many days before "now" it occurred plus the
/// change it records. Kept as fixtures so the demo change logs have real,
/// date-spread data to page through and filter.
struct SeedAudit {
    entity_type: &'static str,
    entity_id: &'static str,
    days_ago: i64,
    actor: &'static str,
    field: &'static str,
    old_value: &'static str,
    new_value: &'static str,
}

/// Insert a small, realistic set of audit entries against a few fixture users
/// and cases, dated across the last ~6 weeks so the default "last 2 weeks" view
/// shows some and widening the range reveals the rest. Ordered oldest-first so
/// the newest change gets the highest `seq` and sorts to the top (matching how
/// live edits accumulate).
async fn seed_audit_fixtures() -> Result<(), sqlx::Error> {
    let pool = pool();
    let mut fixtures = [
        SeedAudit {
            entity_type: "case",
            entity_id: "c-1",
            days_ago: 41,
            actor: "Maria Nguyen",
            field: "status",
            old_value: "Open",
            new_value: "Monitor",
        },
        SeedAudit {
            entity_type: "user",
            entity_id: "u-2",
            days_ago: 33,
            actor: "Maria Nguyen",
            field: "role",
            old_value: "Client",
            new_value: "Volunteer",
        },
        SeedAudit {
            entity_type: "case",
            entity_id: "c-2",
            days_ago: 28,
            actor: "Dana Patel",
            field: "owner",
            old_value: "James Garcia",
            new_value: "Dana Patel",
        },
        SeedAudit {
            entity_type: "user",
            entity_id: "u-1",
            days_ago: 20,
            actor: "Maria Nguyen",
            field: "permissions on c-3",
            old_value: "Viewer",
            new_value: "Manager",
        },
        SeedAudit {
            entity_type: "case",
            entity_id: "c-1",
            days_ago: 12,
            actor: "Dana Patel",
            field: "name",
            old_value: "Maria-Nguyen custody matter",
            new_value: "Nguyen custody matter",
        },
        SeedAudit {
            entity_type: "case",
            entity_id: "c-3",
            days_ago: 9,
            actor: "Maria Nguyen",
            field: "status",
            old_value: "Monitor",
            new_value: "Closed",
        },
        SeedAudit {
            entity_type: "user",
            entity_id: "u-2",
            days_ago: 5,
            actor: "Maria Nguyen",
            field: "permissions on c-1",
            old_value: "none",
            new_value: "Contributor",
        },
        SeedAudit {
            entity_type: "case",
            entity_id: "c-2",
            days_ago: 2,
            actor: "Dana Patel",
            field: "Docket",
            old_value: "FC-2026-0002",
            new_value: "FC-2026-0002-A",
        },
        SeedAudit {
            entity_type: "user",
            entity_id: "u-1",
            days_ago: 1,
            actor: "Maria Nguyen",
            field: "role",
            old_value: "Volunteer",
            new_value: "Site admin",
        },
        SeedAudit {
            entity_type: "case",
            entity_id: "c-1",
            days_ago: 0,
            actor: "Maria Nguyen",
            field: "status",
            old_value: "Monitor",
            new_value: "Open",
        },
    ];
    // Oldest first so the newest change ends up with the highest `seq`.
    fixtures.sort_by_key(|f| std::cmp::Reverse(f.days_ago));

    for f in fixtures {
        let id = ids::next(pool, "cl").await?;
        // User fixtures name their subject positionally ("u-2" = the second
        // seeded user); resolve that to the generated opaque id.
        let entity_id = match f
            .entity_id
            .strip_prefix("u-")
            .and_then(|n| n.parse::<usize>().ok())
        {
            Some(n) if f.entity_type == "user" => crate::mockdata::user_id(n - 1),
            _ => f.entity_id.to_string(),
        };
        let at = (chrono::Local::now() - chrono::Duration::days(f.days_ago))
            .format("%Y-%m-%d %H:%M")
            .to_string();
        sqlx::query(
            "INSERT INTO audit_log
                (id, entity_type, entity_id, actor, field, old_value, new_value, at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now() - make_interval(days => $9))",
        )
        .bind(&id)
        .bind(f.entity_type)
        .bind(&entity_id)
        .bind(f.actor)
        .bind(f.field)
        .bind(f.old_value)
        .bind(f.new_value)
        .bind(&at)
        .bind(f.days_ago as i32)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Insert audit entries preserving their ids/timestamps. Entries are stored
/// newest-first in the fixtures; insert in reverse so `seq DESC` reproduces that
/// order.
async fn insert_audit(
    entity_id: &str,
    entity_type: &str,
    entries: &[ChangeLogEntry],
) -> Result<(), sqlx::Error> {
    for e in entries.iter().rev() {
        sqlx::query(
            "INSERT INTO audit_log (id, entity_type, entity_id, actor, field, old_value, new_value, at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&e.id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(&e.actor)
        .bind(&e.field)
        .bind(&e.old_value)
        .bind(&e.new_value)
        .bind(&e.at)
        .execute(pool())
        .await?;
    }
    Ok(())
}

async fn insert_assignments(u: &User) -> Result<(), sqlx::Error> {
    for a in &u.assigned_cases {
        for cap in &a.capabilities {
            sqlx::query(
                "INSERT INTO case_assignments (user_id, case_id, capability) VALUES ($1, $2, $3)
                 ON CONFLICT DO NOTHING",
            )
            .bind(&u.id)
            .bind(&a.case_id)
            .bind(cap.slug())
            .execute(pool())
            .await?;
        }
    }
    Ok(())
}
