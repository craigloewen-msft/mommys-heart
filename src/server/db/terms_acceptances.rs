//! A record that a client accepted the Terms and Conditions: who, which version
//! of the wording, and when.
//!
//! This replaces the signed `.docx` that used to be filed as case evidence. The
//! version string is stored rather than referenced so that revising
//! [`crate::helpers::terms`] never changes what a past acceptance says.

use crate::server::db::{ids, pool};

/// A recorded acceptance, as shown to staff on a case.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct TermsAcceptance {
    pub terms_version: String,
    pub accepted_at: chrono::DateTime<chrono::Utc>,
}

/// Record an acceptance inside the caller's transaction, so that consent and the
/// case it was given for are committed together or not at all.
pub async fn insert_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
    case_id: Option<&str>,
    terms_version: &str,
) -> Result<(), sqlx::Error> {
    let id = ids::next(&mut **tx, "ta").await?;
    sqlx::query(
        "INSERT INTO terms_acceptances (id, user_id, case_id, terms_version)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(user_id)
    .bind(case_id)
    .bind(terms_version)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// The most recent acceptance recorded for `case_id`, if any.
pub async fn for_case(case_id: &str) -> Result<Option<TermsAcceptance>, sqlx::Error> {
    sqlx::query_as(
        "SELECT terms_version, accepted_at FROM terms_acceptances
         WHERE case_id = $1
         ORDER BY accepted_at DESC
         LIMIT 1",
    )
    .bind(case_id)
    .fetch_optional(pool())
    .await
}
