//! AI reports: describe a report in prose, get a chart, a table and a CSV.
//!
//! The caller types what they want. On the server an agent is handed the
//! database's *structure* (see [`crate::server::reports::schema`]) and a
//! read-only query tool, explores until it understands the data, then commits
//! to one final `SELECT`. Everything it runs — exploration and the final query
//! alike — goes through the read-only gate in
//! [`crate::server::reports::sql_guard`] and a `READ ONLY` transaction, so a
//! report can never change or drop anything.
//!
//! The types below are shared with the browser, so they stay free of any
//! server-only dependency.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::pagination::Page;

/// Most rows a report will return. Beyond this the table is truncated and says so.
pub const MAX_REPORT_ROWS: usize = 2_000;

/// Longest report request we will send to the model.
pub const MAX_REQUEST_CHARS: usize = 1_000;

/// One column of a report result.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReportColumn {
    pub name: String,
    /// Whether the underlying Postgres type is numeric, which is what makes a
    /// column eligible to be charted and right-aligned in the table.
    pub numeric: bool,
}

/// A rectangular result: every cell already rendered as text.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReportTable {
    pub columns: Vec<ReportColumn>,
    pub rows: Vec<Vec<String>>,
    /// Set when more rows matched than [`MAX_REPORT_ROWS`].
    pub truncated: bool,
}

impl ReportTable {
    /// Index of a column by name, case-insensitively.
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// A cell as a number, for charting. Blank and unparseable cells are zero.
    pub fn number_at(&self, row: usize, column: usize) -> f64 {
        self.rows
            .get(row)
            .and_then(|r| r.get(column))
            .map(|value| parse_number(value))
            .unwrap_or(0.0)
    }
}

/// Parse a cell into a chartable number, tolerating currency and thousands
/// separators the way a person would write them.
pub fn parse_number(value: &str) -> f64 {
    let cleaned: String = value
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == 'e' || *c == 'E')
        .collect();
    cleaned.parse::<f64>().unwrap_or(0.0)
}

/// Which chart the agent chose for the result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartKind {
    /// The result is a plain table; no chart is drawn.
    #[default]
    None,
    Bar,
    Line,
    Pie,
}

impl ChartKind {
    pub fn from_slug(slug: &str) -> Self {
        match slug.trim().to_ascii_lowercase().as_str() {
            "bar" | "column" => Self::Bar,
            "line" | "area" => Self::Line,
            "pie" | "doughnut" | "donut" => Self::Pie,
            _ => Self::None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "Table only",
            Self::Bar => "Bar chart",
            Self::Line => "Line chart",
            Self::Pie => "Pie chart",
        }
    }
}

/// How to draw the result: which column labels each point, and which columns
/// carry the values.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ChartSpec {
    pub kind: ChartKind,
    pub label_column: String,
    pub value_columns: Vec<String>,
}

impl ChartSpec {
    pub fn is_drawable(&self) -> bool {
        self.kind != ChartKind::None && !self.value_columns.is_empty()
    }
}

/// One exploratory query the agent ran on the way to the answer, kept so the
/// user can see how the report was arrived at.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReportStep {
    pub sql: String,
    pub note: String,
    pub row_count: usize,
    /// Set when the step was rejected or failed; the agent saw this too.
    pub error: String,
}

/// A finished report.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub title: String,
    pub summary: String,
    /// The final read-only `SELECT`, shown so the numbers can be checked.
    pub sql: String,
    pub table: ReportTable,
    pub chart: ChartSpec,
    pub steps: Vec<ReportStep>,
    /// The whole result as CSV, ready to download.
    pub csv: String,
}

/// A report someone kept: its final query and chart, so it can be run again
/// without the model.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SavedReport {
    pub id: String,
    pub title: String,
    /// The prose request that originally produced it, kept for context.
    pub request: String,
    pub summary: String,
    pub sql: String,
    pub chart: ChartSpec,
    pub author_name: String,
    pub created_at: String,
    /// Blank until the report has been re-run.
    pub last_run_at: String,
}

/// Longest title a saved report may carry.
pub const MAX_TITLE_CHARS: usize = 120;

/// Build a report from a plain-language request.
///
/// Restricted to operations administrators and above: a report can read across
/// the whole database, so no per-record permission can express it.
#[server(prefix = "/api")]
pub async fn build_report(request: String) -> Result<Report, ServerFnError> {
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;

    let request = request.trim().to_string();
    if request.is_empty() {
        return Err(ServerFnError::new("Describe the report you want."));
    }
    if request.chars().count() > MAX_REQUEST_CHARS {
        return Err(ServerFnError::new(format!(
            "Keep the request under {MAX_REQUEST_CHARS} characters."
        )));
    }

    crate::server::reports::build(&request, &user.id)
        .await
        .map_err(ServerFnError::new)
}

/// One page of saved reports, newest first, narrowed by an optional keyword.
#[server(prefix = "/api")]
pub async fn list_saved_reports(
    keyword: String,
    offset: i64,
    limit: i64,
) -> Result<Page<SavedReport>, ServerFnError> {
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;

    crate::server::db::reports::page(&keyword, offset, limit)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn load_saved_report(id: String) -> Result<Option<SavedReport>, ServerFnError> {
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;

    crate::server::db::reports::get(&id)
        .await
        .map_err(ServerFnError::new)
}

/// Keep a built report. Only a title is asked for; the query, chart and author
/// come from what was already produced.
#[server(prefix = "/api")]
pub async fn save_report(
    title: String,
    request: String,
    summary: String,
    sql: String,
    chart: ChartSpec,
) -> Result<String, ServerFnError> {
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;

    let title = title.trim();
    if title.is_empty() {
        return Err(ServerFnError::new("Give the report a title."));
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(ServerFnError::new(format!(
            "Keep the title under {MAX_TITLE_CHARS} characters."
        )));
    }

    // Re-check the query on the way in, so nothing but a read-only SELECT is
    // ever stored to be run later.
    let sql = crate::server::reports::sql_guard::validate(&sql).map_err(ServerFnError::new)?;

    crate::server::db::reports::create(
        title,
        request.trim(),
        summary.trim(),
        &sql,
        &chart,
        &user.id,
    )
    .await
    .map_err(ServerFnError::new)
}

/// Run a saved report again from its stored query. No model is involved.
#[server(prefix = "/api")]
pub async fn run_saved_report(id: String) -> Result<Report, ServerFnError> {
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;

    let saved = crate::server::db::reports::get(&id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("That report no longer exists."))?;

    // Validated again on the way out: a query the guard has since stopped
    // allowing must fail rather than run because it was stored earlier.
    let sql =
        crate::server::reports::sql_guard::validate(&saved.sql).map_err(ServerFnError::new)?;

    let report = crate::server::reports::run_query(&sql, saved.chart, &saved.title, &saved.summary)
        .await
        .map_err(ServerFnError::new)?;

    tracing::info!(actor = %user.id, report = %id, "re-ran saved report");
    let _ = crate::server::db::reports::touch_last_run(&id).await;

    Ok(report)
}

/// Delete a saved report. Any operations admin may: the library is shared.
#[server(prefix = "/api")]
pub async fn delete_saved_report(id: String) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;

    crate::server::db::reports::delete(&id)
        .await
        .map_err(ServerFnError::new)
}
