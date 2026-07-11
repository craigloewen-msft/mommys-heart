//! One-time database seed. Ports the in-memory fixtures from [`crate::mockdata`]
//! into PostgreSQL, hashing the demo passwords so the existing "Demo autofill"
//! logins keep working. Runs only when the database is empty.

use crate::server::auth::hash_password;
use crate::server::db::{pool, users};
use crate::types::{ChangeLogEntry, User};

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

        insert_audit(&c.id, "case", &c.audit_log).await?;
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
