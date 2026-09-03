//! Running a validated report query, read-only (SSR only).
//!
//! Every statement runs inside a transaction that is `SET TRANSACTION READ
//! ONLY` before anything else and rolled back afterwards, with a statement
//! timeout so a runaway query cannot hold a connection. PostgreSQL enforces the
//! read-only part itself, which is what makes this safe rather than merely
//! careful: the text checks in [`super::sql_guard`] are the outer layer, this is
//! the one that cannot be talked around.
//!
//! Results come back as text. The query's shape is *described* first (a
//! prepare, not an execution), then re-issued with every column cast to `text`,
//! which sidesteps decoding types sqlx has no Rust mapping for — `numeric`,
//! which `sum()` returns, being the one that matters most here.

use sqlx::{Column, Executor, Row, TypeInfo};

use crate::server::db::pool;
use crate::server_fns::reports::{ReportColumn, ReportTable};

/// How long any single report query may run.
const STATEMENT_TIMEOUT_MS: i32 = 15_000;

/// Postgres type names that make a column chartable.
const NUMERIC_TYPES: &[&str] = &[
    "INT2", "INT4", "INT8", "FLOAT4", "FLOAT8", "NUMERIC", "MONEY", "OID",
];

/// Run `sql` read-only and return at most `limit` rows.
///
/// The table's `truncated` flag is set by fetching one row more than asked for
/// and dropping it, so "there is more" is a fact rather than a guess.
pub async fn run(sql: &str, limit: usize) -> Result<ReportTable, String> {
    let mut tx = pool()
        .begin()
        .await
        .map_err(|e| format!("could not start a read-only transaction: {e}"))?;

    // First statement in the transaction, so nothing can have written already.
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("could not make the transaction read-only: {e}"))?;
    sqlx::query(&format!(
        "SET LOCAL statement_timeout = {STATEMENT_TIMEOUT_MS}"
    ))
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("could not apply the query timeout: {e}"))?;

    let result = collect(&mut tx, sql, limit).await;

    // Nothing should have changed, but rolling back says so rather than
    // trusting it.
    let _ = tx.rollback().await;
    result
}

async fn collect(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    sql: &str,
    limit: usize,
) -> Result<ReportTable, String> {
    // Wrapping as a subquery is load-bearing: it forces the statement to be a
    // single query expression, so a data-modifying CTE — legal only at the top
    // level — cannot appear at all.
    let wrapped = format!("SELECT * FROM (\n{sql}\n) AS mh_report");

    let described = tx
        .describe(&wrapped)
        .await
        .map_err(|e| format!("the query could not be prepared: {}", trim_db_error(&e)))?;

    let parameter_count = match described.parameters() {
        Some(sqlx::Either::Left(types)) => types.len(),
        Some(sqlx::Either::Right(count)) => count,
        None => 0,
    };
    if parameter_count > 0 {
        return Err("A report query cannot take bind parameters.".into());
    }

    let mut columns = Vec::new();
    let mut seen = Vec::<String>::new();
    for column in described.columns() {
        let type_name = column.type_info().name().to_ascii_uppercase();
        let mut name = column.name().to_string();
        if name.is_empty() {
            name = format!("column_{}", seen.len() + 1);
        }
        // Two columns of the same name would collide in the CSV and in the
        // chart's column lookup.
        if seen.iter().any(|existing| existing == &name) {
            name = format!("{name}_{}", seen.len() + 1);
        }
        if super::sql_guard::column_is_denied(column.name()) {
            return Err(format!(
                "The `{}` column is not available to reports; select named columns instead of *.",
                column.name()
            ));
        }
        seen.push(name.clone());
        columns.push(ReportColumn {
            name,
            numeric: NUMERIC_TYPES.contains(&type_name.as_str()),
        });
    }

    if columns.is_empty() {
        return Err("The query returned no columns.".into());
    }

    // Cast everything to text so decoding never depends on a sqlx type mapping.
    let projection = described
        .columns()
        .iter()
        .enumerate()
        .map(|(index, column)| {
            format!(
                "mh_report.{}::text AS {}",
                quote_ident(column.name()),
                quote_ident(&columns[index].name)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let text_query = format!(
        "SELECT {projection} FROM (\n{sql}\n) AS mh_report LIMIT {}",
        limit + 1
    );

    let mut rows = sqlx::query(&text_query)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| format!("the query failed: {}", trim_db_error(&e)))?;

    let truncated = rows.len() > limit;
    rows.truncate(limit);

    let rows = rows
        .into_iter()
        .map(|row| {
            (0..columns.len())
                .map(|index| {
                    row.try_get::<Option<String>, _>(index)
                        .ok()
                        .flatten()
                        .unwrap_or_default()
                })
                .collect()
        })
        .collect();

    Ok(ReportTable {
        columns,
        rows,
        truncated,
    })
}

/// One table's structure as the agent sees it.
pub struct TableSchema {
    pub name: String,
    /// `(column, type)` pairs, in declaration order.
    pub columns: Vec<(String, String)>,
}

/// Read the shape of every reportable table in the `public` schema.
///
/// Withheld tables and columns are dropped here rather than filtered later, so
/// the agent is never even told they exist.
pub async fn schema() -> Result<Vec<TableSchema>, String> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT c.table_name, c.column_name, c.data_type
           FROM information_schema.columns c
           JOIN information_schema.tables t
             ON t.table_schema = c.table_schema AND t.table_name = c.table_name
          WHERE c.table_schema = 'public' AND t.table_type = 'BASE TABLE'
          ORDER BY c.table_name, c.ordinal_position",
    )
    .fetch_all(pool())
    .await
    .map_err(|e| format!("could not read the database structure: {e}"))?;

    let mut tables: Vec<TableSchema> = Vec::new();
    for (table, column, data_type) in rows {
        if super::sql_guard::table_is_denied(&table) || super::sql_guard::column_is_denied(&column)
        {
            continue;
        }
        match tables.last_mut() {
            Some(last) if last.name == table => last.columns.push((column, data_type)),
            _ => tables.push(TableSchema {
                name: table,
                columns: vec![(column, data_type)],
            }),
        }
    }

    Ok(tables)
}

/// Render the schema as the compact listing handed to the model.
pub fn schema_prompt(tables: &[TableSchema]) -> String {
    let mut out = String::new();
    for table in tables {
        out.push_str(&table.name);
        out.push('(');
        out.push_str(
            &table
                .columns
                .iter()
                .map(|(name, kind)| format!("{name} {}", short_type(kind)))
                .collect::<Vec<_>>()
                .join(", "),
        );
        out.push_str(")\n");
    }
    out
}

fn short_type(data_type: &str) -> &str {
    match data_type {
        "character varying" => "varchar",
        "timestamp with time zone" => "timestamptz",
        "timestamp without time zone" => "timestamp",
        "double precision" => "float8",
        other => other,
    }
}

/// Quote an identifier for interpolation, doubling any embedded quote.
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Database errors carry a lot of noise; the agent only needs the message.
fn trim_db_error(error: &sqlx::Error) -> String {
    match error {
        sqlx::Error::Database(db) => db.message().to_string(),
        other => other.to_string(),
    }
}
