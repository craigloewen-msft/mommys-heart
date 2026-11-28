use crate::server::db::pool;

#[derive(Clone, Debug, PartialEq)]
pub struct FiledRecord {
    pub version: i32,
    pub relative_path: String,
    pub file_name: String,
    pub filed_at: String,
}

pub async fn find(
    kind: &str,
    record_id: &str,
    version: i32,
) -> Result<Option<FiledRecord>, sqlx::Error> {
    let row: Option<(i32, String, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT version, relative_path, file_name, filed_at
         FROM case_record_files
         WHERE kind = $1 AND record_id = $2 AND version = $3",
    )
    .bind(kind)
    .bind(record_id)
    .bind(version)
    .fetch_optional(pool())
    .await?;
    Ok(row.map(
        |(version, relative_path, file_name, filed_at)| FiledRecord {
            version,
            relative_path,
            file_name,
            filed_at: crate::server::db::local_stamp(filed_at),
        },
    ))
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

pub async fn record(
    kind: &str,
    record_id: &str,
    version: i32,
    case_id: &str,
    relative_path: &str,
    file_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO case_record_files
             (kind, record_id, version, case_id, relative_path, file_name)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (kind, record_id, version) DO UPDATE
             SET relative_path = EXCLUDED.relative_path,
                 file_name = EXCLUDED.file_name,
                 filed_at = now()",
    )
    .bind(kind)
    .bind(record_id)
    .bind(version)
    .bind(case_id)
    .bind(relative_path)
    .bind(file_name)
    .execute(pool())
    .await?;
    Ok(())
}
