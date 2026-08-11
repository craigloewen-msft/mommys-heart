//! Case-contact persistence (SSR only): who is involved in a case.
//!
//! Audit entries here are `volunteer_only` and content-free — the role and the
//! identifiers, never the note text — so the case Change Log cannot become a
//! back door to client-facing detail (REQ-AUD-002).

use crate::helpers::visibility::Visibility;
use crate::server::db::{audit, ids, pool};
use crate::server_fns::case_contacts::{CaseContact, CaseContactRole};

#[derive(sqlx::FromRow)]
struct CaseContactRow {
    id: String,
    case_id: String,
    contact_id: String,
    contact_name: String,
    organization_name: Option<String>,
    email: String,
    phone: String,
    role: String,
    note: String,
    is_primary: bool,
    contact_archived: bool,
}

impl From<CaseContactRow> for CaseContact {
    fn from(row: CaseContactRow) -> Self {
        Self {
            id: row.id,
            case_id: row.case_id,
            contact_id: row.contact_id,
            contact_name: row.contact_name,
            organization_name: row.organization_name.unwrap_or_default(),
            email: row.email,
            phone: row.phone,
            role: CaseContactRole::from_slug(&row.role).unwrap_or_default(),
            note: row.note,
            is_primary: row.is_primary,
            contact_archived: row.contact_archived,
        }
    }
}

/// Everyone linked to a case, primary first then most recently added.
pub async fn list(case_id: &str) -> Result<Vec<CaseContact>, sqlx::Error> {
    let rows = sqlx::query_as::<_, CaseContactRow>(
        "SELECT cc.id, cc.case_id, cc.contact_id,
                btrim(coalesce(nullif(c.preferred_name, ''), c.first_name) || ' ' || c.last_name)
                    AS contact_name,
                o.name AS organization_name,
                c.email, c.phone, cc.role, cc.note, cc.is_primary,
                c.archived AS contact_archived
         FROM case_contacts cc
         JOIN contacts c ON c.id = cc.contact_id
         LEFT JOIN organizations o ON o.id = c.organization_id
         WHERE cc.case_id = $1
         ORDER BY cc.is_primary DESC, cc.seq DESC",
    )
    .bind(case_id)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// The case a link belongs to, so callers can authorize against the stored row
/// rather than a case id supplied by the browser.
pub async fn case_id(id: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT case_id FROM case_contacts WHERE id = $1")
        .bind(id)
        .fetch_optional(pool())
        .await
}

/// Link a contact to a case. Promoting a new primary demotes the old one in the
/// same transaction, so the one-primary-per-case index is never violated.
pub async fn add(
    case_id: &str,
    contact_id: &str,
    role: CaseContactRole,
    note: &str,
    is_primary: bool,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let id = ids::next(pool(), "cc").await?;
    let mut tx = pool().begin().await?;
    if is_primary {
        sqlx::query(
            "UPDATE case_contacts SET is_primary = false WHERE case_id = $1 AND is_primary",
        )
        .bind(case_id)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO case_contacts (id, case_id, contact_id, role, note, is_primary, added_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id)
    .bind(case_id)
    .bind(contact_id)
    .bind(role.slug())
    .bind(note)
    .bind(is_primary)
    .bind(actor)
    .execute(&mut *tx)
    .await?;
    // Content-free: the role, never the note.
    audit::record_in_transaction_with_visibility(
        &mut tx,
        audit::Entity::Case,
        case_id,
        actor,
        "case contact",
        "",
        &format!("added ({})", role.label().to_lowercase()),
        Visibility::VolunteerOnly,
    )
    .await?;
    tx.commit().await
}

pub async fn remove(id: &str, case_id: &str, actor: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    let deleted = sqlx::query("DELETE FROM case_contacts WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    audit::record_in_transaction_with_visibility(
        &mut tx,
        audit::Entity::Case,
        case_id,
        actor,
        "case contact",
        "",
        "removed",
        Visibility::VolunteerOnly,
    )
    .await?;
    tx.commit().await
}

/// How many cases a contact is linked to. Used to explain why a contact cannot
/// be deleted.
pub async fn count_for_contact(contact_id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT count(*) FROM case_contacts WHERE contact_id = $1")
        .bind(contact_id)
        .fetch_one(pool())
        .await
}
