//! Where a case note's filed document is (SSR only).
//!
//! Deliberately tiny, for the same reason
//! [`case_documents`](super::case_documents) is: the SharePoint library holds
//! the document, and the only things worth keeping here are the ones that
//! cannot be recomputed from it — which file in the case's `Case Notes` folder
//! belongs to which note, and how much of the note that file already contains.

use crate::server::db::pool;

/// A note's filed document, as recorded here.
#[derive(Clone, Debug, PartialEq)]
pub struct NoteDocumentRef {
    pub note_id: String,
    pub case_id: String,
    /// File name inside the case's `Case Notes` folder.
    pub file_name: String,
    pub web_url: String,
    /// How many addenda the filed document includes.
    pub addenda_included: i32,
    /// When it was filed, pre-formatted in local time like every other stamp
    /// the app displays.
    pub filed_at: String,
}

impl NoteDocumentRef {
    /// The path this document is addressed by everywhere else in the app:
    /// relative to the case folder, which is what the documents surface takes.
    pub fn relative_path(&self) -> String {
        format!(
            "{}/{}",
            crate::server::sharepoint::CASE_NOTES_FOLDER,
            self.file_name
        )
    }
}

#[derive(sqlx::FromRow)]
struct DocumentRow {
    note_id: String,
    case_id: String,
    file_name: String,
    web_url: String,
    addenda_included: i32,
    filed_at: chrono::DateTime<chrono::Utc>,
}

impl From<DocumentRow> for NoteDocumentRef {
    fn from(row: DocumentRow) -> Self {
        use chrono::TimeZone;

        NoteDocumentRef {
            note_id: row.note_id,
            case_id: row.case_id,
            file_name: row.file_name,
            web_url: row.web_url,
            addenda_included: row.addenda_included,
            // Formatted here rather than in SQL so it reads in the server's
            // local time, matching `now_stamp` and every other stamp the app
            // displays. Postgres would render it in the session's zone (UTC),
            // which would show a note filed at 16:53 as 20:53.
            filed_at: chrono::Local
                .from_utc_datetime(&row.filed_at.naive_utc())
                .format("%Y-%m-%d %H:%M")
                .to_string(),
        }
    }
}

const SELECT: &str = "SELECT note_id, case_id, file_name, web_url, addenda_included, filed_at
     FROM case_note_documents";

/// The filed document for one note, or `None` when filing has not succeeded yet.
pub async fn get(note_id: &str) -> Result<Option<NoteDocumentRef>, sqlx::Error> {
    let row = sqlx::query_as::<_, DocumentRow>(&format!("{SELECT} WHERE note_id = $1"))
        .bind(note_id)
        .fetch_optional(pool())
        .await?;
    Ok(row.map(Into::into))
}

/// Whether a case-relative path is a filed note record.
///
/// This is what stops the document surface deleting the file the note record
/// lives in: the library is the source of truth for it, so it may not be
/// removed by hand there any more than a note may be deleted here.
pub async fn is_filed_record(case_id: &str, file_name: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM case_note_documents
         WHERE case_id = $1 AND file_name = $2)",
    )
    .bind(case_id)
    .bind(file_name)
    .fetch_one(pool())
    .await
}

/// Remember a document that was just written to the library.
pub async fn record(
    note_id: &str,
    case_id: &str,
    file_name: &str,
    web_url: &str,
    addenda_included: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO case_note_documents
             (note_id, case_id, file_name, web_url, addenda_included)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (note_id) DO UPDATE
             SET file_name = EXCLUDED.file_name,
                 web_url = EXCLUDED.web_url,
                 addenda_included = EXCLUDED.addenda_included,
                 filed_at = now()",
    )
    .bind(note_id)
    .bind(case_id)
    .bind(file_name)
    .bind(web_url)
    .bind(addenda_included)
    .execute(pool())
    .await?;
    Ok(())
}

/// Notes whose filed document is missing or out of date, oldest first.
///
/// "Out of date" means the note has gained addenda since it was filed. Both
/// cases are the same repair, which is why they are one query: the startup
/// backfill re-files whatever this returns.
///
/// Only finalized and legacy notes are filed. A draft is private to its author
/// until it is finalized, and a discarded one has no content left to file.
pub async fn needing_filing(limit: i64) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT n.id
         FROM case_notes n
         LEFT JOIN case_note_documents d ON d.note_id = n.id
         WHERE n.state IN ('finalized', 'legacy')
           AND (d.note_id IS NULL
                OR d.addenda_included <>
                   (SELECT count(*) FROM case_note_addenda a WHERE a.note_id = n.id))
         ORDER BY n.seq
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool())
    .await
}
