//! Grant persistence (SSR only).
//!
//! Every read carries the grant's received total, computed in SQL from
//! non-voided funding rows rather than summed in the browser, so the rollup
//! cannot drift from the ledger.

use crate::server::db::{audit, ids, pool};
use crate::server_fns::grants::{
    Grant, GrantFilters, GrantStatus, GrantTotals, ReportingCadence, ValidatedGrant,
};
use crate::server_fns::pagination::Page;

#[derive(sqlx::FromRow)]
struct GrantRow {
    id: String,
    name: String,
    status: String,
    funder_organization_id: Option<String>,
    funder_name: Option<String>,
    program_officer_contact_id: Option<String>,
    program_officer_name: Option<String>,
    amount_requested_cents: Option<i64>,
    amount_awarded_cents: Option<i64>,
    application_date: Option<String>,
    decision_date: Option<String>,
    period_start: Option<String>,
    period_end: Option<String>,
    purpose: String,
    reporting_cadence: String,
    notes: String,
    received_cents: i64,
}

impl From<GrantRow> for Grant {
    fn from(row: GrantRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            status: GrantStatus::from_slug(&row.status).unwrap_or_default(),
            funder_organization_id: row.funder_organization_id.unwrap_or_default(),
            funder_name: row.funder_name.unwrap_or_default(),
            program_officer_contact_id: row.program_officer_contact_id.unwrap_or_default(),
            program_officer_name: row.program_officer_name.unwrap_or_default(),
            amount_requested_cents: row.amount_requested_cents,
            amount_awarded_cents: row.amount_awarded_cents,
            application_date: row.application_date.unwrap_or_default(),
            decision_date: row.decision_date.unwrap_or_default(),
            period_start: row.period_start.unwrap_or_default(),
            period_end: row.period_end.unwrap_or_default(),
            purpose: row.purpose,
            reporting_cadence: ReportingCadence::from_slug(&row.reporting_cadence)
                .unwrap_or_default(),
            notes: row.notes,
            received_cents: row.received_cents,
        }
    }
}

/// Dates are rendered as `YYYY-MM-DD` text in SQL so the domain type stays a
/// plain string, matching how the rest of the app carries display dates.
const SELECT_COLUMNS: &str = "g.id, g.name, g.status, g.funder_organization_id,
     o.name AS funder_name, g.program_officer_contact_id,
     btrim(coalesce(nullif(c.preferred_name, ''), c.first_name) || ' ' || c.last_name)
         AS program_officer_name,
     g.amount_requested_cents, g.amount_awarded_cents,
     to_char(g.application_date, 'YYYY-MM-DD') AS application_date,
     to_char(g.decision_date, 'YYYY-MM-DD') AS decision_date,
     to_char(g.period_start, 'YYYY-MM-DD') AS period_start,
     to_char(g.period_end, 'YYYY-MM-DD') AS period_end,
     g.purpose, g.reporting_cadence, g.notes,
     coalesce((SELECT sum(f.amount_cents) FROM funding f
               WHERE f.grant_id = g.id AND NOT f.voided), 0)::BIGINT AS received_cents";

const FROM_JOINS: &str = "FROM grants g
     LEFT JOIN organizations o ON o.id = g.funder_organization_id
     LEFT JOIN contacts c ON c.id = g.program_officer_contact_id";

pub async fn page(
    filters: &GrantFilters,
    offset: i64,
    limit: i64,
) -> Result<Page<Grant>, sqlx::Error> {
    let keyword = filters.keyword.trim();
    let status = filters.status.map(|s| s.slug()).unwrap_or_default();
    let funder = filters.funder_organization_id.trim();

    let where_sql = "WHERE ($1 = '' OR g.status = $1)
           AND ($2 = '' OR g.funder_organization_id = $2)
           AND ($3 = '' OR g.name ILIKE '%' || $3 || '%' OR o.name ILIKE '%' || $3 || '%')";

    let total: i64 = sqlx::query_scalar(&format!("SELECT count(*) {FROM_JOINS} {where_sql}"))
        .bind(status)
        .bind(funder)
        .bind(keyword)
        .fetch_one(pool())
        .await?;

    let rows = sqlx::query_as::<_, GrantRow>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOINS} {where_sql}
         ORDER BY g.seq DESC OFFSET $4 LIMIT $5"
    ))
    .bind(status)
    .bind(funder)
    .bind(keyword)
    .bind(offset.max(0))
    .bind(limit.clamp(1, 200))
    .fetch_all(pool())
    .await?;

    Ok(Page {
        items: rows.into_iter().map(Into::into).collect(),
        total,
    })
}

pub async fn get(id: &str) -> Result<Option<Grant>, sqlx::Error> {
    let row = sqlx::query_as::<_, GrantRow>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOINS} WHERE g.id = $1"
    ))
    .bind(id)
    .fetch_optional(pool())
    .await?;
    Ok(row.map(Into::into))
}

/// Portfolio totals for the grant list header.
///
/// `received` counts only funding recorded *against a grant*: subtracting
/// unrelated donations from the awarded total would make "outstanding" wrong.
/// General giving is visible in the ledger instead.
pub async fn totals() -> Result<GrantTotals, sqlx::Error> {
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
             coalesce(sum(g.amount_awarded_cents), 0)::BIGINT,
             coalesce((SELECT sum(f.amount_cents) FROM funding f
                       WHERE NOT f.voided AND f.grant_id IS NOT NULL), 0)::BIGINT,
             count(*) FILTER (WHERE g.status IN ('awarded', 'active', 'reporting')),
             count(*) FILTER (WHERE g.status IN ('prospect', 'applied'))
         FROM grants g",
    )
    .fetch_one(pool())
    .await?;
    Ok(GrantTotals {
        awarded_cents: row.0,
        received_cents: row.1,
        active_count: row.2,
        prospect_count: row.3,
    })
}

/// Active grants as `(id, name)` for the funding form's picker.
pub async fn options() -> Result<Vec<(String, String)>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, name FROM grants
         WHERE status NOT IN ('declined') ORDER BY lower(name) ASC",
    )
    .fetch_all(pool())
    .await?;
    Ok(rows)
}

pub async fn create(input: &ValidatedGrant, actor: &str) -> Result<String, sqlx::Error> {
    let id = ids::next(pool(), "gr").await?;
    let mut tx = pool().begin().await?;
    sqlx::query(
        "INSERT INTO grants
             (id, name, status, funder_organization_id, program_officer_contact_id,
              amount_requested_cents, amount_awarded_cents, application_date,
              decision_date, period_start, period_end, purpose, reporting_cadence, notes)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8::date, $9::date, $10::date, $11::date, $12, $13, $14)",
    )
    .bind(&id)
    .bind(&input.name)
    .bind(input.status.slug())
    .bind(&input.funder_organization_id)
    .bind(&input.program_officer_contact_id)
    .bind(input.amount_requested_cents)
    .bind(input.amount_awarded_cents)
    .bind(&input.application_date)
    .bind(&input.decision_date)
    .bind(&input.period_start)
    .bind(&input.period_end)
    .bind(&input.purpose)
    .bind(input.reporting_cadence.slug())
    .bind(&input.notes)
    .execute(&mut *tx)
    .await?;
    audit::record_in_transaction(
        &mut tx,
        audit::Entity::Grant,
        &id,
        actor,
        "grant",
        "",
        &format!("created ({})", input.status.label().to_lowercase()),
    )
    .await?;
    tx.commit().await?;
    Ok(id)
}

/// Update a grant, recording the status transition in the Change Log when it
/// actually moves.
pub async fn update(id: &str, input: &ValidatedGrant, actor: &str) -> Result<(), sqlx::Error> {
    let previous: Option<String> = sqlx::query_scalar("SELECT status FROM grants WHERE id = $1")
        .bind(id)
        .fetch_optional(pool())
        .await?;
    let previous = previous.ok_or(sqlx::Error::RowNotFound)?;

    let mut tx = pool().begin().await?;
    sqlx::query(
        "UPDATE grants
         SET name = $2, status = $3, funder_organization_id = $4,
             program_officer_contact_id = $5, amount_requested_cents = $6,
             amount_awarded_cents = $7, application_date = $8::date,
             decision_date = $9::date, period_start = $10::date, period_end = $11::date,
             purpose = $12, reporting_cadence = $13, notes = $14, updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(&input.name)
    .bind(input.status.slug())
    .bind(&input.funder_organization_id)
    .bind(&input.program_officer_contact_id)
    .bind(input.amount_requested_cents)
    .bind(input.amount_awarded_cents)
    .bind(&input.application_date)
    .bind(&input.decision_date)
    .bind(&input.period_start)
    .bind(&input.period_end)
    .bind(&input.purpose)
    .bind(input.reporting_cadence.slug())
    .bind(&input.notes)
    .execute(&mut *tx)
    .await?;

    let moved = previous != input.status.slug();
    let old_label = GrantStatus::from_slug(&previous)
        .map(|s| s.label())
        .unwrap_or("");
    audit::record_in_transaction(
        &mut tx,
        audit::Entity::Grant,
        id,
        actor,
        if moved { "status" } else { "grant" },
        if moved { old_label } else { "" },
        if moved {
            input.status.label()
        } else {
            "updated"
        },
    )
    .await?;
    tx.commit().await
}
