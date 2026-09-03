//! AI reports (SSR only): turn a plain-language request into a chart, a table
//! and a CSV.
//!
//! The pipeline is: read the database's structure ([`execute::schema`]), let the
//! agent explore it and commit to one `SELECT` ([`agent`]), run that query
//! read-only ([`execute`]), then reconcile the chart it asked for against the
//! columns the query actually produced.
//!
//! Everything the agent runs is gated twice — the text checks in [`sql_guard`]
//! and a PostgreSQL `READ ONLY` transaction — so no report, however the request
//! was phrased, can change or delete anything.

pub mod agent;
pub mod execute;
pub mod sql_guard;

use crate::server::config::AzureConfig;
use crate::server_fns::reports::{ChartKind, ChartSpec, Report, ReportTable, MAX_REPORT_ROWS};

/// Build a report for `request`. `actor_id` is logged so a report that reads
/// across the whole database is attributable.
pub async fn build(request: &str, actor_id: &str) -> Result<Report, String> {
    let cfg = AzureConfig::from_env();
    if !cfg.is_configured() {
        return Err(
            "AI reports need Azure OpenAI credentials, which are not configured on this server."
                .into(),
        );
    }

    let tables = execute::schema().await?;
    if tables.is_empty() {
        return Err("No reportable tables were found in the database.".into());
    }
    let schema_prompt = execute::schema_prompt(&tables);

    tracing::info!(actor = actor_id, "building AI report: {request}");

    let outcome = agent::run(&cfg, request, &schema_prompt).await?;
    let table = execute::run(&outcome.sql, MAX_REPORT_ROWS).await?;

    tracing::info!(
        actor = actor_id,
        rows = table.rows.len(),
        steps = outcome.steps.len(),
        "AI report ready: {}",
        outcome.sql
    );

    let chart = reconcile_chart(outcome.chart, &table);
    let csv = to_csv(&table);

    Ok(Report {
        title: outcome.title,
        summary: outcome.summary,
        sql: outcome.sql,
        table,
        chart,
        steps: outcome.steps,
        csv,
    })
}

/// Keep only the parts of the requested chart the result can actually support.
///
/// The model names columns from the query it *intended* to write; if the query
/// it finally sent disagrees, drawing the chart anyway would plot zeroes. A
/// missing label column falls back to the first non-numeric column, and a chart
/// with no usable values is downgraded to a table.
fn reconcile_chart(mut chart: ChartSpec, table: &ReportTable) -> ChartSpec {
    if chart.kind == ChartKind::None {
        return chart;
    }

    chart.value_columns.retain(|name| {
        table
            .column_index(name)
            .is_some_and(|index| table.columns[index].numeric)
    });

    if chart.value_columns.is_empty() {
        // Fall back to whatever numeric columns the result does have, so a
        // near-miss on a column name still draws.
        chart.value_columns = table
            .columns
            .iter()
            .filter(|c| c.numeric)
            .map(|c| c.name.clone())
            .collect();
    }

    if table.column_index(&chart.label_column).is_none() {
        chart.label_column = table
            .columns
            .iter()
            .find(|c| !c.numeric && !chart.value_columns.contains(&c.name))
            .or_else(|| table.columns.first())
            .map(|c| c.name.clone())
            .unwrap_or_default();
    }

    // A pie slices one series; more than one would be meaningless.
    if chart.kind == ChartKind::Pie {
        chart.value_columns.truncate(1);
    }

    if chart.value_columns.is_empty() || table.rows.is_empty() {
        chart.kind = ChartKind::None;
    }

    chart
}

/// Render the whole result as RFC 4180 CSV.
fn to_csv(table: &ReportTable) -> String {
    let mut out = String::new();
    out.push_str(
        &table
            .columns
            .iter()
            .map(|c| escape_csv(&c.name))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push('\n');
    for row in &table.rows {
        out.push_str(
            &row.iter()
                .map(|cell| escape_csv(cell))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }
    out
}

/// Quote a CSV field when it contains a delimiter, quote or newline.
///
/// A leading `=`, `+`, `-` or `@` is also quoted and prefixed with a single
/// quote: spreadsheets otherwise treat the cell as a formula, and this data
/// comes out of free-text fields.
fn escape_csv(value: &str) -> String {
    let risky_formula = value
        .chars()
        .next()
        .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@'));
    let needs_quotes = risky_formula
        || value.contains(',')
        || value.contains('"')
        || value.contains('\n')
        || value.contains('\r');

    if !needs_quotes {
        return value.to_string();
    }
    let escaped = value.replace('"', "\"\"");
    if risky_formula {
        format!("\"'{escaped}\"")
    } else {
        format!("\"{escaped}\"")
    }
}
