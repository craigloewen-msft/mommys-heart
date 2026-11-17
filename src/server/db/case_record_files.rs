use crate::server::db::pool;

#[derive(Clone, Debug, PartialEq)]
pub struct FiledRecord {
    pub version: i32,
    pub relative_path: String,
    pub file_name: String,
    pub web_url: String,
    pub filed_at: String,
}

pub async fn latest(kind: &str, record_id: &str) -> Result<Option<FiledRecord>, sqlx::Error> {
    let row: Option<(i32, String, String, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT version, relative_path, file_name, web_url, filed_at
         FROM case_record_files
         WHERE kind = $1 AND record_id = $2
         ORDER BY version DESC LIMIT 1",
    )
    .bind(kind)
    .bind(record_id)
    .fetch_optional(pool())
    .await?;
    Ok(row.map(|(version, relative_path, file_name, web_url, filed_at)| {
        use chrono::TimeZone;
        FiledRecord {
            version,
            relative_path,
            file_name,
            web_url,
            filed_at: chrono::Local
                .from_utc_datetime(&filed_at.naive_utc())
                .format("%Y-%m-%d %H:%M")
                .to_string(),
        }
    }))
}

pub async fn is_filed_path(case_id: &str, relative_path: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM case_record_files
         WHERE case_id = $1
           AND (lower(relative_path) = lower($2)
                OR left(lower(relative_path), length($2) + 1) = lower($2) || '/'))",
    )
    .bind(case_id)
    .bind(relative_path)
    .fetch_one(pool())
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn record(
    kind: &str,
    record_id: &str,
    version: i32,
    case_id: &str,
    relative_path: &str,
    file_name: &str,
    web_url: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO case_record_files
             (kind, record_id, version, case_id, relative_path, file_name, web_url)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (kind, record_id, version) DO UPDATE
             SET relative_path = EXCLUDED.relative_path,
                 file_name = EXCLUDED.file_name,
                 web_url = EXCLUDED.web_url,
                 filed_at = now()",
    )
    .bind(kind)
    .bind(record_id)
    .bind(version)
    .bind(case_id)
    .bind(relative_path)
    .bind(file_name)
    .bind(web_url)
    .execute(pool())
    .await?;
    Ok(())
}
