//! Funding persistence (SSR only): the money ledger.
//!
//! There is no delete path here at all — a trigger refuses `DELETE` — so the
//! only correction is [`void`], which leaves the row visible and audited while
//! excluding it from every rollup.

use crate::server::db::{audit, ids, pool};
use crate::server_fns::funding::{FundingFilters, FundingKind, FundingRecord, ValidatedFunding};
use crate::server_fns::pagination::Page;

#[derive(sqlx::FromRow)]
struct FundingRow {
    id: String,
    kind: String,
    amount_cents: i64,
    received_on: String,
    grant_id: Option<String>,
    grant_name: Option<String>,
    source_organization_id: Option<String>,
    source_organization_name: Option<String>,
    source_contact_id: Option<String>,
    source_contact_name: Option<String>,
    reference: String,
    notes: String,
    voided: bool,
    void_reason: String,
    voided_by: String,
    voided_at: String,
    recorded_by: String,
    created_at: String,
}

impl From<FundingRow> for FundingRecord {
    fn from(row: FundingRow) -> Self {
        Self {
            id: row.id,
            kind: FundingKind::from_slug(&row.kind).unwrap_or_default(),
            amount_cents: row.amount_cents,
            received_on: row.received_on,
            grant_id: row.grant_id.unwrap_or_default(),
            grant_name: row.grant_name.unwrap_or_default(),
            source_organization_id: row.source_organization_id.unwrap_or_default(),
            source_organization_name: row.source_organization_name.unwrap_or_default(),
            source_contact_id: row.source_contact_id.unwrap_or_default(),
            source_contact_name: row.source_contact_name.unwrap_or_default(),
            reference: row.reference,
            notes: row.notes,
            voided: row.voided,
            void_reason: row.void_reason,
            voided_by: row.voided_by,
            voided_at: row.voided_at,
            recorded_by: row.recorded_by,
            created_at: row.created_at,
        }
    }
}

const SELECT_COLUMNS: &str = "f.id, f.kind, f.amount_cents,
     to_char(f.received_on, 'YYYY-MM-DD') AS received_on,
     f.grant_id, g.name AS grant_name,
     f.source_organization_id, o.name AS source_organization_name,
     f.source_contact_id,
     btrim(coalesce(nullif(c.preferred_name, ''), c.first_name) || ' ' || c.last_name)
         AS source_contact_name,
     f.reference, f.notes, f.voided, f.void_reason, f.voided_by,
     coalesce(to_char(f.voided_at, 'YYYY-MM-DD HH24:MI'), '') AS voided_at,
     f.recorded_by,
     to_char(f.created_at, 'YYYY-MM-DD HH24:MI') AS created_at";

const FROM_JOINS: &str = "FROM funding f
     LEFT JOIN grants g ON g.id = f.grant_id
     LEFT JOIN organizations o ON o.id = f.source_organization_id
     LEFT JOIN contacts c ON c.id = f.source_contact_id";

/// One page of the ledger. Voided rows are hidden unless asked for, so the
/// default view is the money that actually counts.
pub async fn page(
    filters: &FundingFilters,
    offset: i64,
    limit: i64,
) -> Result<Page<FundingRecord>, sqlx::Error> {
    let kind = filters.kind.map(|k| k.slug()).unwrap_or_default();
    let grant_id = filters.grant_id.trim();

    let where_sql = "WHERE ($1 OR NOT f.voided)
           AND ($2 = '' OR f.kind = $2)
           AND ($3 = '' OR f.grant_id = $3)";

    let total: i64 = sqlx::query_scalar(&format!("SELECT count(*) {FROM_JOINS} {where_sql}"))
        .bind(filters.include_voided)
        .bind(kind)
        .bind(grant_id)
        .fetch_one(pool())
        .await?;

    let rows = sqlx::query_as::<_, FundingRow>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOINS} {where_sql}
         ORDER BY f.received_on DESC, f.seq DESC OFFSET $4 LIMIT $5"
    ))
    .bind(filters.include_voided)
    .bind(kind)
    .bind(grant_id)
    .bind(offset.max(0))
    .bind(limit.clamp(1, 200))
    .fetch_all(pool())
    .await?;

    Ok(Page {
        items: rows.into_iter().map(Into::into).collect(),
        total,
    })
}

pub async fn create(
    input: &ValidatedFunding,
    actor_user_id: &str,
    actor: &str,
) -> Result<String, sqlx::Error> {
    let id = ids::next(pool(), "fn").await?;
    let mut tx = pool().begin().await?;
    sqlx::query(
        "INSERT INTO funding
             (id, kind, amount_cents, received_on, grant_id, source_organization_id,
              source_contact_id, reference, notes, recorded_by_user_id, recorded_by)
         VALUES ($1, $2, $3, $4::date, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(&id)
    .bind(input.kind.slug())
    .bind(input.amount_cents)
    .bind(&input.received_on)
    .bind(&input.grant_id)
    .bind(&input.source_organization_id)
    .bind(&input.source_contact_id)
    .bind(&input.reference)
    .bind(&input.notes)
    .bind(actor_user_id)
    .bind(actor)
    .execute(&mut *tx)
    .await?;

    // REQ-CRM-044/048: every receipt has its own stable audit scope, including
    // standalone donations; grant-linked receipts are mirrored on the grant.
    audit::record_in_transaction_by(
        &mut tx,
        audit::Entity::Funding,
        &id,
        actor_user_id,
        actor,
        "funding",
        "",
        "recorded",
    )
    .await?;
    if let Some(grant_id) = &input.grant_id {
        audit::record_in_transaction_by(
            &mut tx,
            audit::Entity::Grant,
            grant_id,
            actor_user_id,
            actor,
            "funding",
            "",
            &format!("recorded {id}"),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(id)
}

/// Void a record with a reason. Refuses to re-void, so an existing reason and
/// its timestamp cannot be overwritten.
pub async fn void(
    id: &str,
    reason: &str,
    actor_user_id: &str,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    let grant_id: Option<Option<String>> =
        sqlx::query_scalar("SELECT grant_id FROM funding WHERE id = $1 AND NOT voided FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some(grant_id) = grant_id else {
        return Err(sqlx::Error::RowNotFound);
    };

    sqlx::query(
        "UPDATE funding
         SET voided = true, void_reason = $2, voided_by_user_id = $3,
             voided_by = $4, voided_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(reason)
    .bind(actor_user_id)
    .bind(actor)
    .execute(&mut *tx)
    .await?;

    audit::record_in_transaction_by(
        &mut tx,
        audit::Entity::Funding,
        id,
        actor_user_id,
        actor,
        "funding",
        "recorded",
        "voided",
    )
    .await?;
    if let Some(grant_id) = grant_id {
        audit::record_in_transaction_by(
            &mut tx,
            audit::Entity::Grant,
            &grant_id,
            actor_user_id,
            actor,
            "funding",
            id,
            "voided",
        )
        .await?;
    }
    tx.commit().await
}

/// The non-voided funding recorded against one grant, newest first.
pub async fn for_grant(grant_id: &str) -> Result<Vec<FundingRecord>, sqlx::Error> {
    let rows = sqlx::query_as::<_, FundingRow>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOINS}
         WHERE f.grant_id = $1 ORDER BY f.received_on DESC, f.seq DESC"
    ))
    .bind(grant_id)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}
