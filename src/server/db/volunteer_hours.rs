//! Persistence for self-reported volunteer service time (SSR only).

use crate::server::db::{ids, pool};
use crate::server_fns::volunteer_hours::{VolunteerHour, VolunteerHourInput, VolunteerHours};

#[derive(sqlx::FromRow)]
struct VolunteerHourRow {
    id: String,
    service_date: String,
    duration_minutes: i32,
    description: String,
}

impl From<VolunteerHourRow> for VolunteerHour {
    fn from(row: VolunteerHourRow) -> Self {
        Self {
            id: row.id,
            service_date: row.service_date,
            duration_minutes: row.duration_minutes,
            description: row.description,
        }
    }
}

/// Return all entries for one volunteer, newest service date first, plus the
/// all-time total in minutes.
pub async fn list_for_user(user_id: &str) -> Result<VolunteerHours, sqlx::Error> {
    let total_minutes = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(sum(duration_minutes), 0)::BIGINT
         FROM volunteer_hours
         WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool())
    .await?;

    let entries = sqlx::query_as::<_, VolunteerHourRow>(
        "SELECT id, to_char(service_date, 'YYYY-MM-DD') AS service_date,
                duration_minutes, description
         FROM volunteer_hours
         WHERE user_id = $1
         ORDER BY service_date DESC, seq DESC",
    )
    .bind(user_id)
    .fetch_all(pool())
    .await?
    .into_iter()
    .map(Into::into)
    .collect();

    Ok(VolunteerHours {
        entries,
        total_minutes,
    })
}

/// Create one entry owned by `user_id`.
pub async fn create(user_id: &str, input: &VolunteerHourInput) -> Result<(), sqlx::Error> {
    let id = ids::next(pool(), "vh").await?;
    sqlx::query(
        "INSERT INTO volunteer_hours
             (id, user_id, service_date, duration_minutes, description)
         VALUES ($1, $2, $3::date, $4, $5)",
    )
    .bind(id)
    .bind(user_id)
    .bind(&input.service_date)
    .bind(input.duration_minutes)
    .bind(&input.description)
    .execute(pool())
    .await?;
    Ok(())
}

/// Update one entry, scoped to its owner. Returns false when no owned row has
/// that id.
pub async fn update(
    user_id: &str,
    entry_id: &str,
    input: &VolunteerHourInput,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE volunteer_hours
         SET service_date = $3::date,
             duration_minutes = $4,
             description = $5,
             updated_at = now()
         WHERE id = $1 AND user_id = $2",
    )
    .bind(entry_id)
    .bind(user_id)
    .bind(&input.service_date)
    .bind(input.duration_minutes)
    .bind(&input.description)
    .execute(pool())
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Delete one entry, scoped to its owner. Returns false when no owned row has
/// that id.
pub async fn delete(user_id: &str, entry_id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM volunteer_hours WHERE id = $1 AND user_id = $2")
        .bind(entry_id)
        .bind(user_id)
        .execute(pool())
        .await?;
    Ok(result.rows_affected() == 1)
}
