//! The read-only gate every generated query passes through (SSR only).
//!
//! Two independent defences, because one is not enough for SQL a model wrote:
//!
//! 1. *This module* — the statement must be a single `SELECT`/`WITH`, with no
//!    comments, no statement separator, no writing keyword, and no reference to
//!    a table or column withheld from reports.
//! 2. The database — [`super::execute`] runs it inside a `READ ONLY`
//!    transaction that is rolled back, so PostgreSQL itself refuses any write
//!    even if something slipped past the text checks.
//!
//! Scanning happens on the statement with string literals and quoted
//! identifiers blanked out, so `WHERE status = 'deleted'` is not mistaken for a
//! `DELETE` and a literal cannot smuggle a keyword past the check.

/// Tables reports may not read at all: credentials, sessions and the one-time
/// codes behind them. Nothing in here is reportable, and all of it is dangerous
/// to surface.
const DENIED_TABLES: &[&str] = &[
    "sessions",
    "mfa_challenges",
    "trusted_devices",
    "password_reset_tokens",
    "auth_throttle",
    "pending_registrations",
    "message_send_throttle",
    "_sqlx_migrations",
    "pg_authid",
    "pg_shadow",
    "pg_user_mappings",
];

/// Column names withheld wherever they appear, so `SELECT *` over a permitted
/// table cannot leak a secret either.
const DENIED_COLUMNS: &[&str] = &[
    "password_hash",
    "password",
    "token_hash",
    "session_token",
    "secret",
    "access_key",
    "api_key",
];

/// Keywords that write, change structure, or reach outside the database.
/// `into` is here because `SELECT ... INTO` creates a table.
const DENIED_KEYWORDS: &[&str] = &[
    "insert",
    "update",
    "delete",
    "drop",
    "alter",
    "create",
    "truncate",
    "grant",
    "revoke",
    "merge",
    "call",
    "do",
    "copy",
    "vacuum",
    "reindex",
    "cluster",
    "refresh",
    "lock",
    "set",
    "reset",
    "begin",
    "start",
    "commit",
    "rollback",
    "savepoint",
    "prepare",
    "execute",
    "deallocate",
    "listen",
    "notify",
    "unlisten",
    "discard",
    "into",
    "nextval",
    "setval",
    "dblink",
    "pg_sleep",
    "pg_read_file",
    "pg_read_binary_file",
    "pg_ls_dir",
    "lo_import",
    "lo_export",
    "pg_terminate_backend",
];

/// Check a model-written statement and return it normalised (trimmed, with any
/// trailing semicolon removed) so it can be wrapped as a subquery.
pub fn validate(sql: &str) -> Result<String, String> {
    let trimmed = sql.trim().trim_end_matches(';').trim().to_string();

    if trimmed.is_empty() {
        return Err("The query was empty.".into());
    }
    if trimmed.len() > 20_000 {
        return Err("The query is too long.".into());
    }

    // Comments are the usual way to hide a second statement, and a report has
    // no need of them.
    if trimmed.contains("--") || trimmed.contains("/*") {
        return Err("Comments are not allowed in a report query.".into());
    }

    let Scrubbed {
        sql: scrubbed,
        quoted_identifiers,
    } = blank_literals(&trimmed)?;

    if scrubbed.contains(';') {
        return Err("Only one statement is allowed; remove the semicolons.".into());
    }

    let lowered = scrubbed.to_ascii_lowercase();
    let first = lowered
        .split(|c: char| !c.is_ascii_alphabetic())
        .find(|word| !word.is_empty())
        .unwrap_or("");
    if first != "select" && first != "with" {
        return Err("A report query must start with SELECT or WITH.".into());
    }

    for keyword in DENIED_KEYWORDS {
        if contains_word(&lowered, keyword) {
            return Err(format!(
                "`{}` is not allowed: reports are read-only.",
                keyword.to_uppercase()
            ));
        }
    }
    // Quoted identifiers are excluded from keyword scanning above (an alias may
    // legitimately read "Grant into detail"), but a quoted identifier can still
    // name a withheld table, so it is matched here in full.
    for table in DENIED_TABLES {
        if contains_word(&lowered, table) || names(&quoted_identifiers, table) {
            return Err(format!(
                "The `{table}` table holds credentials and is not available to reports."
            ));
        }
    }
    for column in DENIED_COLUMNS {
        if contains_word(&lowered, column) || names(&quoted_identifiers, column) {
            return Err(format!(
                "The `{column}` column is not available to reports."
            ));
        }
    }

    Ok(trimmed)
}

/// Whether a table is withheld from reports. Used when describing the schema so
/// the agent is never shown what it may not query.
pub fn table_is_denied(table: &str) -> bool {
    DENIED_TABLES
        .iter()
        .any(|denied| denied.eq_ignore_ascii_case(table))
}

/// Whether a column is withheld from reports.
pub fn column_is_denied(column: &str) -> bool {
    DENIED_COLUMNS
        .iter()
        .any(|denied| denied.eq_ignore_ascii_case(column))
}

/// Whether any quoted identifier is exactly `name`.
fn names(quoted_identifiers: &[String], name: &str) -> bool {
    quoted_identifiers
        .iter()
        .any(|identifier| identifier.eq_ignore_ascii_case(name))
}

/// A statement with its literals removed, plus the quoted identifiers that were
/// removed along the way.
struct Scrubbed {
    sql: String,
    quoted_identifiers: Vec<String>,
}

/// Replace the contents of every `'string'`, `"identifier"` and `$tag$body$tag$`
/// with spaces, so keyword scanning only ever sees SQL code.
///
/// Quoted identifiers are collected rather than discarded: they must not be
/// scanned for keywords (an alias may read `"Grant into detail"`), but
/// `FROM "sessions"` still has to be caught by the table check.
fn blank_literals(sql: &str) -> Result<Scrubbed, String> {
    let chars: Vec<char> = sql.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut quoted_identifiers = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '\'' | '"' => {
                let quote = chars[i];
                let mut inner = String::new();
                out.push(' ');
                i += 1;
                loop {
                    if i >= chars.len() {
                        return Err("The query has an unterminated quote.".into());
                    }
                    if chars[i] == quote {
                        // A doubled quote is an escaped quote, not the end.
                        if chars.get(i + 1) == Some(&quote) {
                            inner.push(quote);
                            out.push_str("  ");
                            i += 2;
                            continue;
                        }
                        out.push(' ');
                        i += 1;
                        break;
                    }
                    inner.push(chars[i]);
                    out.push(' ');
                    i += 1;
                }
                if quote == '"' {
                    quoted_identifiers.push(inner);
                }
            }
            '$' => {
                let tag_end = chars[i + 1..]
                    .iter()
                    .position(|c| *c == '$')
                    .map(|o| i + 1 + o);
                let tag: Option<String> = tag_end.and_then(|end| {
                    let tag: String = chars[i..=end].iter().collect();
                    let inner = &tag[1..tag.len() - 1];
                    inner
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_')
                        .then_some(tag)
                });
                let Some(tag) = tag else {
                    // A bare `$` (a positional parameter, say) is ordinary text.
                    out.push('$');
                    i += 1;
                    continue;
                };
                let tag_len = tag.chars().count();
                let rest: String = chars[i + tag_len..].iter().collect();
                let Some(close) = rest.find(&tag) else {
                    return Err("The query has an unterminated dollar-quoted string.".into());
                };
                let consumed = tag_len + rest[..close].chars().count() + tag_len;
                out.push_str(&" ".repeat(consumed));
                i += consumed;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }

    Ok(Scrubbed {
        sql: out,
        quoted_identifiers,
    })
}

/// Whole-word match, so `deleted_at` does not trip the `delete` check and
/// `offset` does not trip `set`.
fn contains_word(haystack: &str, word: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut from = 0;
    while let Some(found) = haystack[from..].find(word) {
        let start = from + found;
        let end = start + word.len();
        let before_ok = start == 0 || !is_ident_char(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_ident_char(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
        if from >= haystack.len() {
            break;
        }
    }
    false
}

fn is_ident_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
