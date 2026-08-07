//! Case folder persistence (SSR only): the `case_folders` tree a case's files
//! are organized into.
//!
//! Two invariants live here and nowhere else, because every write to the tree
//! goes through this module:
//!
//! * A folder's `case_id` and `visibility` are copied from its parent, never
//!   supplied by a caller. The two roots are the only rows that decide an
//!   audience, and they are created with the case.
//! * A folder's *path* — the chain of names from its root down to it — is
//!   derived, not stored ([`path_of`]). It is what a file's blob name is built
//!   from, so it has exactly one definition.

use crate::helpers::new_case_folders;
use crate::helpers::visibility::Visibility;
use crate::server::db::{audit, ids, pool};
use crate::server_fns::case_folders::CaseFolder;

const COLUMNS: &str = "id, case_id, parent_id, name, visibility";

#[derive(sqlx::FromRow)]
struct FolderRow {
    id: String,
    case_id: String,
    parent_id: Option<String>,
    name: String,
    visibility: String,
}

impl FolderRow {
    fn into_folder(self) -> CaseFolder {
        CaseFolder {
            id: self.id,
            case_id: self.case_id,
            parent_id: self.parent_id,
            name: self.name,
            visibility: Visibility::from_slug(&self.visibility).unwrap_or_default(),
        }
    }
}

/// Every folder on a case that the caller may see, creation order.
pub async fn list(
    case_id: &str,
    include_volunteer_only: bool,
) -> Result<Vec<CaseFolder>, sqlx::Error> {
    let rows = sqlx::query_as::<_, FolderRow>(&format!(
        "SELECT {COLUMNS} FROM case_folders
         WHERE case_id = $1 AND ($2 OR visibility <> $3) ORDER BY seq ASC"
    ))
    .bind(case_id)
    .bind(include_volunteer_only)
    .bind(Visibility::VolunteerOnly.slug())
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(FolderRow::into_folder).collect())
}

/// A single folder by id, or `None` if it does not exist.
pub async fn get(folder_id: &str) -> Result<Option<CaseFolder>, sqlx::Error> {
    let row = sqlx::query_as::<_, FolderRow>(&format!(
        "SELECT {COLUMNS} FROM case_folders WHERE id = $1"
    ))
    .bind(folder_id)
    .fetch_optional(pool())
    .await?;
    Ok(row.map(FolderRow::into_folder))
}

/// How deep a folder sits: 1 for a standing top-level folder, 2 for a folder
/// inside one, and so on.
pub async fn depth_of(folder_id: &str) -> Result<usize, sqlx::Error> {
    let depth: Option<i64> = sqlx::query_scalar(
        "WITH RECURSIVE up AS (
             SELECT id, parent_id, 1 AS depth FROM case_folders WHERE id = $1
             UNION ALL
             SELECT f.id, f.parent_id, up.depth + 1
             FROM case_folders f JOIN up ON f.id = up.parent_id
         )
         SELECT max(depth)::bigint FROM up",
    )
    .bind(folder_id)
    .fetch_one(pool())
    .await?;
    Ok(depth.unwrap_or(0) as usize)
}

/// Whether a folder holds nothing at all — no files and no sub-folders. Deleting
/// a folder is only allowed once this is true.
pub async fn is_empty(folder_id: &str) -> Result<bool, sqlx::Error> {
    let occupied: Option<i32> = sqlx::query_scalar(
        "SELECT 1 WHERE EXISTS (SELECT 1 FROM evidence WHERE folder_id = $1)
                        OR EXISTS (SELECT 1 FROM case_folders WHERE parent_id = $1)",
    )
    .bind(folder_id)
    .fetch_optional(pool())
    .await?;
    Ok(occupied.is_none())
}

/// Whether `parent_id` already holds a folder with this name (case-insensitively).
pub async fn child_exists(parent_id: &str, name: &str) -> Result<bool, sqlx::Error> {
    let found: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM case_folders WHERE parent_id = $1 AND lower(name) = lower($2)",
    )
    .bind(parent_id)
    .bind(name)
    .fetch_optional(pool())
    .await?;
    Ok(found.is_some())
}

/// Create a sub-folder of `parent`, auditing it. Returns the new folder's id.
pub async fn create(parent: &CaseFolder, name: &str, actor: &str) -> Result<String, sqlx::Error> {
    let mut tx = pool().begin().await?;
    let folder = create_in(&mut tx, parent, name).await?;
    tx.commit().await?;
    audit::record(
        pool(),
        audit::Entity::Case,
        &parent.case_id,
        actor,
        "folder",
        "",
        name,
    )
    .await?;
    Ok(folder.id)
}

/// Create a sub-folder of `parent` inside an existing transaction, so a case can
/// be created together with the folders it starts out with.
pub async fn create_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    parent: &CaseFolder,
    name: &str,
) -> Result<CaseFolder, sqlx::Error> {
    let id = ids::next(&mut **tx, "f").await?;
    sqlx::query(
        "INSERT INTO case_folders (id, case_id, parent_id, name, visibility)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&id)
    .bind(&parent.case_id)
    .bind(&parent.id)
    .bind(name)
    .bind(parent.visibility.slug())
    .execute(&mut **tx)
    .await?;
    Ok(CaseFolder {
        id,
        case_id: parent.case_id.clone(),
        parent_id: Some(parent.id.clone()),
        name: name.to_string(),
        visibility: parent.visibility,
    })
}

/// The sub-folder of `parent` with this name, creating it if it is not there
/// yet. Used to build the standing folders a new case starts with.
pub async fn ensure_child_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    parent: &CaseFolder,
    name: &str,
) -> Result<CaseFolder, sqlx::Error> {
    let existing = sqlx::query_as::<_, FolderRow>(&format!(
        "SELECT {COLUMNS} FROM case_folders WHERE parent_id = $1 AND lower(name) = lower($2)"
    ))
    .bind(&parent.id)
    .bind(name)
    .fetch_optional(&mut **tx)
    .await?;
    match existing {
        Some(row) => Ok(row.into_folder()),
        None => create_in(tx, parent, name).await,
    }
}

/// Create the folder tree a new case starts with, from
/// [`NEW_CASE_FOLDERS`](crate::helpers::new_case_folders::NEW_CASE_FOLDERS).
/// Called in the case's own creation transaction so a case can never exist
/// without somewhere to put its files.
pub async fn create_for_new_case(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    case_id: &str,
) -> Result<(), sqlx::Error> {
    for spec in new_case_folders::NEW_CASE_FOLDERS {
        let id = ids::next(&mut **tx, "f").await?;
        sqlx::query(
            "INSERT INTO case_folders (id, case_id, parent_id, name, visibility)
             VALUES ($1, $2, NULL, $3, $4)",
        )
        .bind(&id)
        .bind(case_id)
        .bind(spec.name)
        .bind(spec.visibility.slug())
        .execute(&mut **tx)
        .await?;

        let parent = CaseFolder {
            id,
            case_id: case_id.to_string(),
            parent_id: None,
            name: spec.name.to_string(),
            visibility: spec.visibility,
        };
        for child in spec.children {
            create_in(tx, &parent, child).await?;
        }
    }
    Ok(())
}

/// A folder by its path from the top of a case's tree, e.g.
/// `["Intake", "Service Agreement"]`. `None` when any step is missing.
pub async fn find_by_path_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    case_id: &str,
    path: &[&str],
) -> Result<Option<CaseFolder>, sqlx::Error> {
    let mut found: Option<CaseFolder> = None;
    for name in path {
        let row = sqlx::query_as::<_, FolderRow>(&format!(
            "SELECT {COLUMNS} FROM case_folders
             WHERE case_id = $1 AND lower(name) = lower($2)
               AND parent_id IS NOT DISTINCT FROM $3"
        ))
        .bind(case_id)
        .bind(name)
        .bind(found.as_ref().map(|f| f.id.clone()))
        .fetch_optional(&mut **tx)
        .await?;
        match row {
            Some(row) => found = Some(row.into_folder()),
            None => return Ok(None),
        }
    }
    Ok(found)
}

/// Remove an empty folder, auditing it. Callers check emptiness first; the
/// database would otherwise cascade, which is exactly what must not happen.
pub async fn delete(folder_id: &str, actor: &str) -> Result<(), sqlx::Error> {
    let row: Option<(String, String)> =
        sqlx::query_as("DELETE FROM case_folders WHERE id = $1 RETURNING case_id, name")
            .bind(folder_id)
            .fetch_optional(pool())
            .await?;
    let Some((case_id, name)) = row else {
        return Ok(());
    };
    audit::record(
        pool(),
        audit::Entity::Case,
        &case_id,
        actor,
        "folder",
        &name,
        "",
    )
    .await
}
