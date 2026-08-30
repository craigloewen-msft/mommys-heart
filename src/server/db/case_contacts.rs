//! Case-contact persistence: who is involved in a case.
//!
//! Relationship audit identifies both endpoints and the role, never the note.

use crate::helpers::visibility::Visibility;
use crate::server::db::{audit, ids, pool};
use crate::server_fns::case_contacts::{CaseContact, CaseContactRole, ContactCaseLink};
use crate::server_fns::cases::CaseStatus;

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

#[derive(sqlx::FromRow)]
struct ContactCaseRow {
    id: String,
    case_id: String,
    case_name: String,
    case_status: String,
    role: String,
    note: String,
    is_primary: bool,
    can_edit: bool,
}

impl From<ContactCaseRow> for ContactCaseLink {
    fn from(row: ContactCaseRow) -> Self {
        Self {
            id: row.id,
            case_id: row.case_id,
            case_name: row.case_name,
            case_status: CaseStatus::from_slug(&row.case_status).unwrap_or(CaseStatus::Open),
            role: CaseContactRole::from_slug(&row.role).unwrap_or_default(),
            note: row.note,
            is_primary: row.is_primary,
            can_edit: row.can_edit,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StoredLink {
    contact_id: String,
    role: String,
    contact_archived: bool,
}

/// Everyone linked to a case, primary first then most recently added.
pub async fn list(case_id: &str) -> Result<Vec<CaseContact>, sqlx::Error> {
    let rows = sqlx::query_as::<_, CaseContactRow>(
        "SELECT cc.id, cc.case_id, cc.contact_id,
                coalesce(nullif(btrim(coalesce(nullif(c.preferred_name, ''), c.first_name)
                    || ' ' || c.last_name), ''), nullif(o.name, ''), c.id) AS contact_name,
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

/// Every case link for a person, including whether this viewer may mutate it.
/// A site admin holds every capability on every case, so they see and may edit
/// every link; everyone else needs the stored capability on that case.
///
/// `can_edit` excludes the frozen statuses, mirroring `CaseStatus::accepts_changes`
/// — `require_cap` refuses every write to a declined *or* withdrawn case, so
/// offering the control on one would only produce an error when used. The link
/// itself stays listed: that the case existed is part of the person's history.
pub async fn list_for_contact(
    contact_id: &str,
    user_id: &str,
) -> Result<Vec<ContactCaseLink>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ContactCaseRow>(
        "SELECT cc.id, cc.case_id, ca.name AS case_name, ca.status AS case_status,
                cc.role, cc.note, cc.is_primary,
                (ca.status NOT IN ('declined', 'withdrawn') AND (
                    EXISTS (
                        SELECT 1 FROM case_assignments assignment
                        WHERE assignment.case_id = ca.id
                          AND assignment.user_id = $2
                          AND assignment.capability = 'edit_case'
                    )
                    OR EXISTS (
                        SELECT 1 FROM users viewer
                        WHERE viewer.id = $2 AND viewer.role = 'site_admin'
                    )
                )) AS can_edit
         FROM case_contacts cc
         JOIN cases ca ON ca.id = cc.case_id
         WHERE cc.contact_id = $1
           AND (
               EXISTS (
                   SELECT 1 FROM case_assignments visible
                   WHERE visible.case_id = ca.id
                     AND visible.user_id = $2
                     AND visible.capability = 'view_case'
               )
               OR EXISTS (
                   SELECT 1 FROM users viewer
                   WHERE viewer.id = $2 AND viewer.role = 'site_admin'
               )
           )
         ORDER BY ca.id, cc.is_primary DESC, cc.seq DESC",
    )
    .bind(contact_id)
    .bind(user_id)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// The case a link belongs to, resolved from stored data for authorization.
pub async fn case_id(id: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT case_id FROM case_contacts WHERE id = $1")
        .bind(id)
        .fetch_optional(pool())
        .await
}

async fn require_active_contact_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    contact_id: &str,
) -> Result<(), sqlx::Error> {
    let archived: Option<bool> =
        sqlx::query_scalar("SELECT archived FROM contacts WHERE id = $1 FOR UPDATE")
            .bind(contact_id)
            .fetch_optional(&mut **tx)
            .await?;
    match archived {
        Some(false) => Ok(()),
        Some(true) => Err(sqlx::Error::Protocol("archived contact".into())),
        None => Err(sqlx::Error::Protocol(
            "That person no longer exists.".into(),
        )),
    }
}

async fn audit_link_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    case_id: &str,
    contact_id: &str,
    role: CaseContactRole,
    actor: &str,
    action: &str,
) -> Result<(), sqlx::Error> {
    let description = format!(
        "{} {} as {}",
        action,
        contact_id,
        role.label().to_lowercase()
    );
    audit::record_in_transaction_with_visibility(
        tx,
        audit::Entity::Case,
        case_id,
        actor,
        "case contact",
        "",
        &description,
        Visibility::VolunteerOnly,
    )
    .await?;
    // Keep the contact-scoped log free of case identifiers; case visibility is
    // governed by case capabilities and the detailed entry already lives there.
    audit::record_in_transaction(
        tx,
        audit::Entity::Contact,
        contact_id,
        actor,
        "case relationship",
        "",
        action,
    )
    .await
}

/// Insert a link in the caller's transaction.
pub async fn add_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    case_id: &str,
    contact_id: &str,
    role: CaseContactRole,
    note: &str,
    is_primary: bool,
    actor: &str,
) -> Result<String, sqlx::Error> {
    require_active_contact_in(tx, contact_id).await?;
    if is_primary {
        sqlx::query(
            "UPDATE case_contacts SET is_primary = false WHERE case_id = $1 AND is_primary",
        )
        .bind(case_id)
        .execute(&mut **tx)
        .await?;
    }
    let id = ids::next(&mut **tx, "cc").await?;
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
    .execute(&mut **tx)
    .await?;
    audit_link_in(tx, case_id, contact_id, role, actor, "added").await?;
    Ok(id)
}

pub async fn add(
    case_id: &str,
    contact_id: &str,
    role: CaseContactRole,
    note: &str,
    is_primary: bool,
    actor_user_id: &str,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_user_id).await?;
    add_in(&mut tx, case_id, contact_id, role, note, is_primary, actor).await?;
    tx.commit().await
}

pub async fn update(
    id: &str,
    case_id: &str,
    role: CaseContactRole,
    note: &str,
    is_primary: bool,
    actor_user_id: &str,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_user_id).await?;
    let stored = sqlx::query_as::<_, StoredLink>(
        "SELECT cc.contact_id, cc.role, c.archived AS contact_archived
         FROM case_contacts cc JOIN contacts c ON c.id = cc.contact_id
         WHERE cc.id = $1 AND cc.case_id = $2 FOR UPDATE OF cc",
    )
    .bind(id)
    .bind(case_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(sqlx::Error::RowNotFound)?;
    if stored.contact_archived {
        return Err(sqlx::Error::Protocol(
            "Restore this person before editing their case link.".into(),
        ));
    }
    if is_primary {
        sqlx::query(
            "UPDATE case_contacts SET is_primary = false
             WHERE case_id = $1 AND id <> $2 AND is_primary",
        )
        .bind(case_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("UPDATE case_contacts SET role = $2, note = $3, is_primary = $4 WHERE id = $1")
        .bind(id)
        .bind(role.slug())
        .bind(note)
        .bind(is_primary)
        .execute(&mut *tx)
        .await?;
    let old_role = CaseContactRole::from_slug(&stored.role).unwrap_or_default();
    audit_link_in(
        &mut tx,
        case_id,
        &stored.contact_id,
        role,
        actor,
        if old_role == role {
            "updated"
        } else {
            "changed"
        },
    )
    .await?;
    tx.commit().await
}

pub async fn remove(
    id: &str,
    case_id: &str,
    actor_user_id: &str,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_user_id).await?;
    let stored = sqlx::query_as::<_, StoredLink>(
        "SELECT cc.contact_id, cc.role, c.archived AS contact_archived
         FROM case_contacts cc JOIN contacts c ON c.id = cc.contact_id
         WHERE cc.id = $1 AND cc.case_id = $2 FOR UPDATE OF cc",
    )
    .bind(id)
    .bind(case_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(sqlx::Error::RowNotFound)?;
    sqlx::query("DELETE FROM case_contacts WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    audit_link_in(
        &mut tx,
        case_id,
        &stored.contact_id,
        CaseContactRole::from_slug(&stored.role).unwrap_or_default(),
        actor,
        "removed",
    )
    .await?;
    tx.commit().await
}

pub async fn count_for_contact(contact_id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT count(*) FROM case_contacts WHERE contact_id = $1")
        .bind(contact_id)
        .fetch_one(pool())
        .await
}
