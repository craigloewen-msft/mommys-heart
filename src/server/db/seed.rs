//! One-time database seed. Ports the in-memory fixtures from [`crate::mockdata`]
//! into PostgreSQL, hashing the demo passwords so the existing "Demo autofill"
//! logins keep working. Runs only when the database is empty.

use crate::server::auth::hash_password;
use crate::server::db::{ids, pool, users};
use crate::server_fns::audit::ChangeLogEntry;
use crate::server_fns::users::User;

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
/// mock fixtures. Used by `etc/dev-db.sh seed` to (re)populate a dev database on
/// demand. Not used by the normal server startup path, which only seeds an empty
/// database via [`seed_if_empty`].
pub async fn reseed() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("wiping existing CRM data before reseeding");
    // One statement so FK constraints are satisfied atomically; RESTART IDENTITY
    // resets the audit_log sequence so ids are reproducible across reseeds.
    sqlx::query(
        "TRUNCATE users, sessions, grants, cases, case_properties, case_notes,
                  evidence, messages, case_assignments, audit_log
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
    for u in crate::mockdata::users() {
        let password_hash = hash_password(&u.password)?;
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

        insert_audit(&u.id, "user", &Vec::<ChangeLogEntry>::new()).await?;
    }

    // 2. Grants.
    for g in crate::mockdata::grants() {
        sqlx::query("INSERT INTO grants (id, name) VALUES ($1, $2)")
            .bind(&g.id)
            .bind(&g.name)
            .execute(pool)
            .await?;
    }

    // 3. Cases + notes, evidence, properties, and case audit log.
    for c in crate::mockdata::cases() {
        sqlx::query("INSERT INTO cases (id, name, status, owner_id) VALUES ($1, $2, $3, $4)")
            .bind(&c.id)
            .bind(&c.name)
            .bind(c.status.slug())
            .bind(&c.owner_id)
            .execute(pool)
            .await?;

        for (ord, p) in c.properties.iter().enumerate() {
            sqlx::query(
                "INSERT INTO case_properties (case_id, ord, key, value) VALUES ($1, $2, $3, $4)",
            )
            .bind(&c.id)
            .bind(ord as i32)
            .bind(&p.key)
            .bind(&p.value)
            .execute(pool)
            .await?;
        }

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

        for e in &c.evidence {
            sqlx::query(
                "INSERT INTO evidence (id, case_id, name, uploaded_by, uploaded_at, description)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&e.id)
            .bind(&c.id)
            .bind(&e.name)
            .bind(&e.uploaded_by)
            .bind(&e.uploaded_at)
            .bind(&e.description)
            .execute(pool)
            .await?;
        }

        insert_audit(&c.id, "case", &Vec::<ChangeLogEntry>::new()).await?;
    }

    // 4. Case assignments (after both users and cases exist).
    for u in crate::mockdata::users() {
        insert_assignments(&u).await?;
    }

    // 5. Case chat messages.
    for m in crate::mockdata::messages() {
        sqlx::query(
            "INSERT INTO messages (id, case_id, author_id, author, body, sent_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&m.id)
        .bind(&m.case_id)
        .bind(&m.author_id)
        .bind(&m.author)
        .bind(&m.body)
        .bind(&m.sent_at)
        .execute(pool)
        .await?;
    }

    // 6. A handful of real audit-log entries, date-spread across the last ~6
    //    weeks, so the change-log views (which fetch the audit log as their own
    //    paginated, date-filtered data source) have data to show and page
    //    through in the demo. Real edits made in the running app append more.
    seed_audit_fixtures().await?;

    // 7. Advance the shared id sequence past every seeded id. Seed ids are
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
async fn advance_id_sequence() -> Result<(), sqlx::Error> {
    sqlx::query(
        "SELECT setval('app_id_seq', GREATEST(
             (SELECT COALESCE(max(split_part(id, '-', 2)::bigint), 0) FROM users),
             (SELECT COALESCE(max(split_part(id, '-', 2)::bigint), 0) FROM grants),
             (SELECT COALESCE(max(split_part(id, '-', 2)::bigint), 0) FROM cases),
             (SELECT COALESCE(max(split_part(id, '-', 2)::bigint), 0) FROM case_notes),
             (SELECT COALESCE(max(split_part(id, '-', 2)::bigint), 0) FROM evidence),
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
        SeedAudit { entity_type: "case", entity_id: "c-1", days_ago: 41, actor: "Maria Nguyen", field: "status", old_value: "Open", new_value: "Monitor" },
        SeedAudit { entity_type: "user", entity_id: "u-2", days_ago: 33, actor: "Maria Nguyen", field: "role", old_value: "Client", new_value: "Volunteer" },
        SeedAudit { entity_type: "case", entity_id: "c-2", days_ago: 28, actor: "Dana Patel", field: "owner", old_value: "James Garcia", new_value: "Dana Patel" },
        SeedAudit { entity_type: "user", entity_id: "u-1", days_ago: 20, actor: "Maria Nguyen", field: "permissions on c-3", old_value: "Viewer", new_value: "Manager" },
        SeedAudit { entity_type: "case", entity_id: "c-1", days_ago: 12, actor: "Dana Patel", field: "name", old_value: "Maria-Nguyen custody matter", new_value: "Nguyen custody matter" },
        SeedAudit { entity_type: "case", entity_id: "c-3", days_ago: 9, actor: "Maria Nguyen", field: "status", old_value: "Monitor", new_value: "Closed" },
        SeedAudit { entity_type: "user", entity_id: "u-2", days_ago: 5, actor: "Maria Nguyen", field: "permissions on c-1", old_value: "none", new_value: "Contributor" },
        SeedAudit { entity_type: "case", entity_id: "c-2", days_ago: 2, actor: "Dana Patel", field: "Docket", old_value: "FC-2026-0002", new_value: "FC-2026-0002-A" },
        SeedAudit { entity_type: "user", entity_id: "u-1", days_ago: 1, actor: "Maria Nguyen", field: "role", old_value: "Volunteer", new_value: "Admin" },
        SeedAudit { entity_type: "case", entity_id: "c-1", days_ago: 0, actor: "Maria Nguyen", field: "status", old_value: "Monitor", new_value: "Open" },
    ];
    // Oldest first so the newest change ends up with the highest `seq`.
    fixtures.sort_by_key(|f| std::cmp::Reverse(f.days_ago));

    for f in fixtures {
        let id = ids::next(pool, "cl").await?;
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
        .bind(f.entity_id)
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
