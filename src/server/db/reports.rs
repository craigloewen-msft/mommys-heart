//! Saved report persistence (SSR only).
//!
//! A saved report is the final `SELECT` plus the chart spec that made sense of
//! it, so re-running one never needs the model again. The author's name is
//! joined in on read, since the list is about who kept what.

use crate::server::db::{ids, pool};
use crate::server_fns::pagination::Page;
use crate::server_fns::reports::{ChartSpec, SavedReport};

#[derive(sqlx::FromRow)]
struct SavedReportRow {
    id: String,
    title: String,
    request: String,
    summary: String,
    sql_text: String,
    chart: serde_json::Value,
    author_name: Option<String>,
    created_at: String,
    last_run_at: Option<String>,
}

impl From<SavedReportRow> for SavedReport {
    fn from(row: SavedReportRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            request: row.request,
            summary: row.summary,
            sql: row.sql_text,
            chart: serde_json::from_value(row.chart).unwrap_or_default(),
            author_name: row.author_name.unwrap_or_default(),
            created_at: row.created_at,
            last_run_at: row.last_run_at.unwrap_or_default(),
        }
    }
}

const SELECT_COLUMNS: &str = "r.id, r.title, r.request, r.summary, r.sql_text, r.chart,
     btrim(coalesce(u.first_name, '') || ' ' || coalesce(u.last_name, '')) AS author_name,
     to_char(r.created_at, 'YYYY-MM-DD HH24:MI') AS created_at,
     to_char(r.last_run_at, 'YYYY-MM-DD HH24:MI') AS last_run_at";

const FROM_JOINS: &str = "FROM saved_reports r LEFT JOIN users u ON u.id = r.created_by";

/// One page of saved reports, newest first, optionally narrowed by keyword.
pub async fn page(
    keyword: &str,
    offset: i64,
    limit: i64,
) -> Result<Page<SavedReport>, sqlx::Error> {
    let keyword = keyword.trim();
    let where_sql = "WHERE ($1 = '' OR r.title ILIKE '%' || $1 || '%'
               OR r.request ILIKE '%' || $1 || '%'
               OR u.first_name ILIKE '%' || $1 || '%'
               OR u.last_name ILIKE '%' || $1 || '%')";

    let total: i64 = sqlx::query_scalar(&format!("SELECT count(*) {FROM_JOINS} {where_sql}"))
        .bind(keyword)
        .fetch_one(pool())
        .await?;

    let rows = sqlx::query_as::<_, SavedReportRow>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOINS} {where_sql}
         ORDER BY r.created_at DESC, r.id DESC OFFSET $2 LIMIT $3"
    ))
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

pub async fn get(id: &str) -> Result<Option<SavedReport>, sqlx::Error> {
    let row = sqlx::query_as::<_, SavedReportRow>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOINS} WHERE r.id = $1"
    ))
    .bind(id)
    .fetch_optional(pool())
    .await?;
    Ok(row.map(Into::into))
}

pub async fn create(
    title: &str,
    request: &str,
    summary: &str,
    sql: &str,
    chart: &ChartSpec,
    author_id: &str,
) -> Result<String, sqlx::Error> {
    let id = ids::opaque("rpt");
    let chart_json = serde_json::to_value(chart).unwrap_or_default();
    sqlx::query(
        "INSERT INTO saved_reports (id, title, request, summary, sql_text, chart, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id)
    .bind(title)
    .bind(request)
    .bind(summary)
    .bind(sql)
    .bind(chart_json)
    .bind(author_id)
    .execute(pool())
    .await?;
    Ok(id)
}

/// Record that the report was just run, so the list can say when it last was.
pub async fn touch_last_run(id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE saved_reports SET last_run_at = now() WHERE id = $1")
        .bind(id)
        .execute(pool())
        .await?;
    Ok(())
}

pub async fn delete(id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM saved_reports WHERE id = $1")
        .bind(id)
        .execute(pool())
        .await?;
    Ok(())
}
