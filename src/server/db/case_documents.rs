//! Case document persistence (SSR only): where a case's SharePoint folder is,
//! and which sharing invitations the app has issued on it.
//!
//! Deliberately small. The library itself holds the files, the folders and the
//! timestamps; the only things worth keeping here are the two that cannot be
//! recomputed from it:
//!
//! * the case's folder id, so the app finds the folder again without searching
//!   the library by name; and
//! * the permission ids the app created, so a revoke withdraws exactly what a
//!   grant issued rather than guessing from the recipient's address.

use crate::server::db::pool;
use crate::server::sharepoint::DocumentRole;

/// Where a case's documents live.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CaseFolderRef {
    /// The library's id for the case folder. Empty until provisioning succeeds.
    pub item_id: String,
    /// Browser link to that folder.
    pub web_url: String,
}

impl CaseFolderRef {
    /// Whether the case's folder has actually been created yet.
    pub fn is_ready(&self) -> bool {
        !self.item_id.is_empty()
    }
}

/// One invitation the app issued, as recorded here.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordedGrant {
    pub user_id: String,
    pub folder_name: String,
    pub permission_id: String,
    pub role: String,
    pub email: String,
}

/// Who should be able to reach a case's documents, and how.
///
/// Read from the same `case_assignments` rows the rest of authorization uses, so
/// the library's access can never disagree with the app's own: a user appears
/// here exactly when they hold `view_evidence` on the case.
#[derive(Clone, Debug, PartialEq)]
pub struct DesiredGrant {
    pub user_id: String,
    pub email: String,
    pub role: DocumentRole,
    /// Whether this user may see volunteer-only folders.
    pub sees_volunteer_only: bool,
}

/// The case's folder, or an empty reference when it has not been provisioned.
pub async fn folder_ref(case_id: &str) -> Result<CaseFolderRef, sqlx::Error> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT drive_item_id, documents_web_url FROM cases WHERE id = $1")
            .bind(case_id)
            .fetch_optional(pool())
            .await?;
    Ok(row
        .map(|(item_id, web_url)| CaseFolderRef { item_id, web_url })
        .unwrap_or_default())
}

/// Record where a case's folder ended up, after provisioning created it.
pub async fn set_folder_ref(
    case_id: &str,
    item_id: &str,
    web_url: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE cases SET drive_item_id = $2, documents_web_url = $3 WHERE id = $1")
        .bind(case_id)
        .bind(item_id)
        .bind(web_url)
        .execute(pool())
        .await?;
    Ok(())
}

/// Every case that still has no documents folder.
///
/// Used by the startup reconciler to finish provisioning that a Graph outage
/// left undone. Ordered by the numeric part of the id, so the oldest cases are
/// caught up first; `cases` has no sequence column to order by.
pub async fn unprovisioned_case_ids(limit: i64) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT id FROM cases WHERE drive_item_id = ''
         ORDER BY CASE WHEN split_part(id, '-', 2) ~ '^[0-9]+$'
                       THEN split_part(id, '-', 2)::bigint ELSE 0 END
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool())
    .await
}

/// Who should hold access to a case's documents right now.
///
/// `view_evidence` is the capability that decides presence; holding
/// `upload_evidence` as well is what turns a read grant into a write one.
/// Deactivated accounts and accounts without an email are excluded — there is
/// nobody to invite.
pub async fn desired_grants(case_id: &str) -> Result<Vec<DesiredGrant>, sqlx::Error> {
    use crate::server_fns::users::AccountRole;

    let rows: Vec<(String, String, bool, String)> = sqlx::query_as(
        "SELECT u.id, u.email,
                bool_or(a.capability = 'upload_evidence') AS may_upload,
                u.role
         FROM case_assignments a
         JOIN users u ON u.id = a.user_id
         WHERE a.case_id = $1
           AND u.email <> ''
           AND u.role <> 'deactivated'
           AND EXISTS (
               SELECT 1 FROM case_assignments v
               WHERE v.user_id = a.user_id AND v.case_id = a.case_id
                 AND v.capability = 'view_evidence'
           )
         GROUP BY u.id, u.email, u.role
         ORDER BY u.id",
    )
    .bind(case_id)
    .fetch_all(pool())
    .await?;

    Ok(rows
        .into_iter()
        .map(|(user_id, email, may_upload, role)| {
            let role_parsed = AccountRole::from_slug(&role).unwrap_or(AccountRole::Client);
            DesiredGrant {
                user_id,
                email,
                role: if may_upload {
                    DocumentRole::Write
                } else {
                    DocumentRole::Read
                },
                // The same account-role gate the app applies to volunteer-only
                // case information, so the library agrees with the case page.
                sees_volunteer_only: role_parsed.has_volunteer_privileges(),
            }
        })
        .collect())
}

/// Every invitation the app has recorded for a case.
pub async fn recorded_grants(case_id: &str) -> Result<Vec<RecordedGrant>, sqlx::Error> {
    let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT user_id, folder_name, permission_id, role, email
         FROM case_document_permissions WHERE case_id = $1",
    )
    .bind(case_id)
    .fetch_all(pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(user_id, folder_name, permission_id, role, email)| RecordedGrant {
                user_id,
                folder_name,
                permission_id,
                role,
                email,
            },
        )
        .collect())
}

/// Remember an invitation that was just issued.
pub async fn record_grant(
    case_id: &str,
    user_id: &str,
    folder_name: &str,
    permission_id: &str,
    role: DocumentRole,
    email: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO case_document_permissions
             (case_id, user_id, folder_name, permission_id, role, email)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (case_id, user_id, folder_name) DO UPDATE
             SET permission_id = EXCLUDED.permission_id,
                 role = EXCLUDED.role,
                 email = EXCLUDED.email,
                 granted_at = now()",
    )
    .bind(case_id)
    .bind(user_id)
    .bind(folder_name)
    .bind(permission_id)
    .bind(role.slug())
    .bind(email)
    .execute(pool())
    .await?;
    Ok(())
}

/// Forget an invitation that has been withdrawn.
pub async fn forget_grant(
    case_id: &str,
    user_id: &str,
    folder_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM case_document_permissions
         WHERE case_id = $1 AND user_id = $2 AND folder_name = $3",
    )
    .bind(case_id)
    .bind(user_id)
    .bind(folder_name)
    .execute(pool())
    .await?;
    Ok(())
}

/// Every case a user currently holds a document grant on.
///
/// Used when an account is deactivated: the account's assignments may stay
/// exactly as they were, so the cases to re-sync have to be read from what was
/// actually granted.
pub async fn case_ids_granted_to(user_id: &str) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT DISTINCT case_id FROM case_document_permissions WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool())
    .await
}
