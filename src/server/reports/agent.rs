//! The report-building agent (SSR only).
//!
//! The model is given the database's structure and one tool: run a read-only
//! query. It explores until it understands the data — checking how a status is
//! spelled, what a date column actually contains — and only then commits to the
//! query behind the report. That loop is the point: a single-shot "write me
//! SQL" prompt guesses at values it has never seen, and guesses wrongly.
//!
//! Each turn the model answers with one JSON object:
//!
//! - `{"action":"explore","sql":...,"note":...}` — run it, hand back the rows
//! - `{"action":"final","sql":...,"title":...,"summary":...,"chart":{...}}`
//! - `{"action":"error","message":...}` — the request cannot be answered
//!
//! Rejections are fed back rather than raised, so the model corrects its own
//! SQL against the real error the way a person would.

use serde::Deserialize;

use crate::server::config::AzureConfig;
use crate::server::rag::azure::{chat_json, ChatTurn};
use crate::server_fns::reports::{ChartKind, ChartSpec, ReportStep, ReportTable};

use super::{execute, sql_guard};

/// How many exploratory turns the agent gets before it must answer.
const MAX_TURNS: usize = 8;

/// Rows an exploratory query returns to the model. Enough to see the shape of
/// the data without spending the context window on it.
const EXPLORE_ROWS: usize = 20;

/// Cells are truncated to this before going back to the model, so one long
/// note body cannot crowd out the rest of the conversation.
const MAX_CELL_CHARS: usize = 200;

/// Output budget per turn. Generous because a reasoning deployment spends part
/// of it thinking before it writes anything.
const MAX_OUTPUT_TOKENS: u32 = 8_000;

const SYSTEM_PROMPT: &str = r#"You are the reporting analyst for Mommy's Heart, a case-management and CRM application backed by PostgreSQL. A staff member describes the report they want in plain language and you produce ONE read-only SQL query that answers it, plus a chart choice.

You work in a loop. Every reply is a single JSON object and nothing else.

To look at the data first:
{"action":"explore","sql":"SELECT ...","note":"what you are checking and why"}
The query is run read-only and you get back the column names and up to 20 rows.

When you are ready:
{"action":"final","sql":"SELECT ...","title":"Short report title","summary":"One or two sentences describing what the report shows and any caveat.","chart":{"kind":"bar","label_column":"month","value_columns":["total_cents"]}}

If the request cannot be answered from this database:
{"action":"error","message":"Explain plainly what is missing."}

Rules for every query you write:
- Read-only. SELECT or WITH only. No INSERT, UPDATE, DELETE, DDL, or SET. The connection is read-only and will reject anything else.
- One statement. No semicolons, no SQL comments, no bind parameters — inline literal values.
- Never select * from a table; name the columns you want.
- Always give computed columns a clear, human-readable alias (lower_snake_case).
- Order the result meaningfully and cap it with LIMIT unless the report is an aggregate that is naturally small.

What you need to know about this schema:
- IDs are TEXT, e.g. 'u-admin', 'c-1001'. Enum-like columns store lower_snake_case slugs as TEXT.
- Timestamps are TEXT, not a date type, written as 'YYYY-MM-DD HH:MM' or 'YYYY-MM-DD'. They sort correctly as text. To group by month use left(created_at, 7); to cast, use to_date(left(col,10),'YYYY-MM-DD') and guard against blank strings.
- Money is stored in integer minor units (cents) in *_cents columns. Divide by 100.0 for display and say so in the summary.
- Some rows are logically deleted or voided rather than removed (e.g. funding.voided). Exclude them unless the request asks otherwise.
- Do not guess how a status or slug is spelled. Run an explore query with a GROUP BY to find the real values first.

Choosing the chart:
- "bar" for a category compared against a measure, "line" for something over time, "pie" for parts of one whole (only with a handful of rows), "none" when the answer is a list rather than a shape.
- label_column must be one non-numeric column of your final result; value_columns must be one or more numeric columns of it. Use the aliases exactly as they appear in your final SELECT.
- Put the label column first and the value columns after it in the final SELECT.

Prefer to run at least one explore query before answering. Never invent a table or column that is not in the schema below."#;

/// What the agent settled on.
pub struct AgentOutcome {
    pub title: String,
    pub summary: String,
    pub sql: String,
    pub chart: ChartSpec,
    pub steps: Vec<ReportStep>,
}

#[derive(Deserialize)]
struct AgentReply {
    action: Option<String>,
    sql: Option<String>,
    note: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    message: Option<String>,
    chart: Option<ChartReply>,
}

#[derive(Deserialize)]
struct ChartReply {
    kind: Option<String>,
    label_column: Option<String>,
    #[serde(default)]
    value_columns: Vec<String>,
}

/// Run the loop until the agent commits to a final query or runs out of turns.
pub async fn run(
    cfg: &AzureConfig,
    request: &str,
    schema_prompt: &str,
) -> Result<AgentOutcome, String> {
    let system = format!(
        "{SYSTEM_PROMPT}\n\nDatabase schema (PostgreSQL, schema \"public\"):\n{schema_prompt}"
    );

    let mut turns = vec![
        ChatTurn::system(system),
        ChatTurn::user(format!("Report request: {request}")),
    ];
    let mut steps = Vec::new();

    for turn in 0..MAX_TURNS {
        let last_turn = turn == MAX_TURNS - 1;
        let raw = chat_json(cfg, &cfg.report_deployment, &turns, MAX_OUTPUT_TOKENS).await?;
        turns.push(ChatTurn::assistant(raw.clone()));

        let reply: AgentReply = serde_json::from_str(&raw)
            .map_err(|e| format!("the assistant's reply was not usable JSON: {e}"))?;

        match reply.action.as_deref().unwrap_or("").trim() {
            "final" => {
                let sql = reply.sql.unwrap_or_default();
                let validated = match sql_guard::validate(&sql) {
                    Ok(validated) => validated,
                    Err(message) => {
                        steps.push(ReportStep {
                            sql,
                            note: "Final query".into(),
                            row_count: 0,
                            error: message.clone(),
                        });
                        turns.push(ChatTurn::user(format!(
                            "That final query was rejected: {message}\nFix it and reply again."
                        )));
                        continue;
                    }
                };
                return Ok(AgentOutcome {
                    title: clean(reply.title.unwrap_or_default(), "Report", 120),
                    summary: clean(reply.summary.unwrap_or_default(), "", 600),
                    sql: validated,
                    chart: chart_from(reply.chart),
                    steps,
                });
            }
            "error" => {
                return Err(clean(
                    reply.message.unwrap_or_default(),
                    "That report cannot be built from this database.",
                    400,
                ));
            }
            "explore" => {
                let sql = reply.sql.unwrap_or_default();
                let note = clean(reply.note.unwrap_or_default(), "Exploring", 200);
                let (feedback, step) = explore(&sql, &note).await;
                steps.push(step);

                let nudge = if last_turn {
                    "\nThis was your last exploration. Reply now with the final report."
                } else {
                    ""
                };
                turns.push(ChatTurn::user(format!("{feedback}{nudge}")));
            }
            other => {
                turns.push(ChatTurn::user(format!(
                    "\"{other}\" is not a valid action. Reply with one JSON object using \"explore\", \"final\", or \"error\"."
                )));
            }
        }
    }

    Err("The assistant could not settle on a report. Try describing it more specifically.".into())
}

/// Validate and run one exploratory query, returning what the model is told
/// about it alongside the step recorded for the user.
async fn explore(sql: &str, note: &str) -> (String, ReportStep) {
    let validated = match sql_guard::validate(sql) {
        Ok(validated) => validated,
        Err(message) => {
            return (
                format!("That query was rejected: {message}"),
                ReportStep {
                    sql: sql.to_string(),
                    note: note.to_string(),
                    row_count: 0,
                    error: message,
                },
            );
        }
    };

    match execute::run(&validated, EXPLORE_ROWS).await {
        Ok(table) => (
            render_for_model(&table),
            ReportStep {
                sql: validated,
                note: note.to_string(),
                row_count: table.rows.len(),
                error: String::new(),
            },
        ),
        Err(message) => (
            format!("That query failed: {message}"),
            ReportStep {
                sql: validated,
                note: note.to_string(),
                row_count: 0,
                error: message,
            },
        ),
    }
}

/// A compact rendering of a result set for the model: the header, then one
/// pipe-separated line per row.
fn render_for_model(table: &ReportTable) -> String {
    if table.rows.is_empty() {
        return format!(
            "Query ran. Columns: {}. It returned no rows.",
            table
                .columns
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let mut out = format!(
        "Query ran and returned {} row(s){}.\n{}\n",
        table.rows.len(),
        if table.truncated { " (truncated)" } else { "" },
        table
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(" | ")
    );
    for row in &table.rows {
        out.push_str(
            &row.iter()
                .map(|cell| truncate(cell, MAX_CELL_CHARS))
                .collect::<Vec<_>>()
                .join(" | "),
        );
        out.push('\n');
    }
    out
}

fn chart_from(reply: Option<ChartReply>) -> ChartSpec {
    let Some(reply) = reply else {
        return ChartSpec::default();
    };
    ChartSpec {
        kind: ChartKind::from_slug(&reply.kind.unwrap_or_default()),
        label_column: reply.label_column.unwrap_or_default().trim().to_string(),
        value_columns: reply
            .value_columns
            .into_iter()
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .collect(),
    }
}

fn clean(value: String, fallback: &str, max: usize) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return fallback.to_string();
    }
    truncate(trimmed, max)
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let kept: String = value.chars().take(max).collect();
    format!("{kept}\u{2026}")
}
