//! Persistence and atomic decisions for operations-admin approval requests.

use std::fmt;

use crate::server::db::{audit, ids, pool, users};
use crate::server_fns::admin_requests::{AdminRequest, AdminRequestKind, AdminRequestStatus};
use crate::server_fns::capabilities::CaseCapability;
use crate::server_fns::pagination::Page;
use crate::server_fns::users::AccountRole;

const SELECT_REQUEST: &str = "SELECT r.id, r.kind, r.status, r.requested_by,
            trim(concat(rb.first_name, ' ', rb.last_name)) AS requested_by_name,
            r.target_user_id,
            trim(concat(tu.first_name, ' ', tu.last_name)) AS target_user_name,
            r.case_id, c.name AS case_name, r.previous_role AS current_role, r.requested_role,
            r.current_capabilities, r.requested_capabilities, r.request_note,
            to_char(r.created_at, 'YYYY-MM-DD HH24:MI') AS created_at,
            NULLIF(trim(concat(db.first_name, ' ', db.last_name)), '') AS decided_by_name,
            r.decision_note,
            CASE WHEN r.decided_at IS NULL THEN NULL
                 ELSE to_char(r.decided_at, 'YYYY-MM-DD HH24:MI') END AS decided_at
     FROM admin_requests r
     JOIN users rb ON rb.id = r.requested_by
     JOIN users tu ON tu.id = r.target_user_id
     LEFT JOIN cases c ON c.id = r.case_id
     LEFT JOIN users db ON db.id = r.decided_by";

#[derive(sqlx::FromRow)]
struct RequestRow {
    id: String,
    kind: String,
    status: String,
    requested_by: String,
    requested_by_name: String,
    target_user_id: String,
    target_user_name: String,
    case_id: Option<String>,
    case_name: Option<String>,
    current_role: Option<String>,
    requested_role: Option<String>,
    current_capabilities: Option<Vec<String>>,
    requested_capabilities: Option<Vec<String>>,
    request_note: String,
    created_at: String,
    decided_by_name: Option<String>,
    decision_note: String,
    decided_at: Option<String>,
}

impl TryFrom<RequestRow> for AdminRequest {
    type Error = Error;

    fn try_from(row: RequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            kind: match row.kind.as_str() {
                "role" => AdminRequestKind::Role,
                "case_capabilities" => AdminRequestKind::CaseCapabilities,
                other => {
                    return Err(Error::InvalidData(format!(
                        "unknown request kind {other:?}"
                    )))
                }
            },
            status: match row.status.as_str() {
                "pending" => AdminRequestStatus::Pending,
                "approved" => AdminRequestStatus::Approved,
                "denied" => AdminRequestStatus::Denied,
                other => {
                    return Err(Error::InvalidData(format!(
                        "unknown request status {other:?}"
                    )))
                }
            },
            requested_by_id: row.requested_by,
            requested_by_name: nonempty_name(row.requested_by_name, "Unknown requester"),
            target_user_id: row.target_user_id,
            target_user_name: nonempty_name(row.target_user_name, "Unknown user"),
            case_id: row.case_id,
            case_name: row.case_name,
            current_role: parse_role(row.current_role)?,
            requested_role: parse_role(row.requested_role)?,
            current_capabilities: parse_capabilities(row.current_capabilities)?,
            requested_capabilities: parse_capabilities(row.requested_capabilities)?,
            request_note: row.request_note,
            created_at: row.created_at,
            decided_by_name: row.decided_by_name,
            decision_note: row.decision_note,
            decided_at: row.decided_at,
        })
    }
}

pub struct DecisionOutcome {
    pub request: AdminRequest,
    pub was_unassigned: bool,
}

#[derive(Debug)]
pub enum Error {
    Database(sqlx::Error),
    NotFound,
    AlreadyResolved,
    DuplicatePending,
    NoChange,
    Stale,
    InvalidData(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => write!(formatter, "{error}"),
            Self::NotFound => write!(formatter, "Request or target not found."),
            Self::AlreadyResolved => write!(formatter, "This request has already been decided."),
            Self::DuplicatePending => write!(formatter, "A pending request already exists for this change."),
            Self::NoChange => write!(formatter, "The requested value already matches the current value."),
            Self::Stale => write!(formatter, "The target changed after this request was filed. Deny it and submit a fresh request."),
            Self::InvalidData(message) => formatter.write_str(message),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        if let sqlx::Error::Database(database) = &error {
            if database.constraint().is_some_and(|constraint| {
                constraint == "admin_requests_pending_role_target_idx"
                    || constraint == "admin_requests_pending_case_target_idx"
            }) {
                return Self::DuplicatePending;
            }
        }
        Self::Database(error)
    }
}

pub async fn list_active(
    requester_id: &str,
    is_site_admin: bool,
) -> Result<Vec<AdminRequest>, Error> {
    let sql = format!(
        "{SELECT_REQUEST}
         WHERE r.status = 'pending' AND ($1 OR r.requested_by = $2)
         ORDER BY r.seq DESC"
    );
    sqlx::query_as::<_, RequestRow>(&sql)
        .bind(is_site_admin)
        .bind(requester_id)
        .fetch_all(pool())
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect()
}

pub async fn history_page(
    requester_id: &str,
    is_site_admin: bool,
    offset: i64,
    limit: i64,
) -> Result<Page<AdminRequest>, Error> {
    let total = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_requests
         WHERE status <> 'pending' AND ($1 OR requested_by = $2)",
    )
    .bind(is_site_admin)
    .bind(requester_id)
    .fetch_one(pool())
    .await?;
    let sql = format!(
        "{SELECT_REQUEST}
         WHERE r.status <> 'pending' AND ($1 OR r.requested_by = $2)
         ORDER BY r.seq DESC
         LIMIT $3 OFFSET $4"
    );
    let items = sqlx::query_as::<_, RequestRow>(&sql)
        .bind(is_site_admin)
        .bind(requester_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool())
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Page { items, total })
}

pub async fn pending_count() -> Result<i64, Error> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*) FROM admin_requests WHERE status = 'pending'")
            .fetch_one(pool())
            .await?,
    )
}

pub async fn create_role(
    requester_id: &str,
    target_user_id: &str,
    requested_role: AccountRole,
    note: &str,
) -> Result<AdminRequest, Error> {
    let mut tx = pool().begin().await?;
    let current_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM users WHERE id = $1 FOR SHARE")
            .bind(target_user_id)
            .fetch_optional(&mut *tx)
            .await?;
    let current_role = current_role.ok_or(Error::NotFound)?;
    if current_role == requested_role.slug() {
        return Err(Error::NoChange);
    }
    let id = ids::next(&mut *tx, "ar").await?;
    sqlx::query(
        "INSERT INTO admin_requests
             (id, kind, requested_by, target_user_id, previous_role, requested_role, request_note)
         VALUES ($1, 'role', $2, $3, $4, $5, $6)",
    )
    .bind(&id)
    .bind(requester_id)
    .bind(target_user_id)
    .bind(current_role)
    .bind(requested_role.slug())
    .bind(note)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    get(&id).await
}

pub async fn create_case_capabilities(
    requester_id: &str,
    target_user_id: &str,
    changes: &[(String, Option<Vec<CaseCapability>>)],
    note: &str,
) -> Result<Vec<AdminRequest>, Error> {
    if changes.is_empty() {
        return Err(Error::InvalidData(
            "Choose at least one case-permission change.".to_string(),
        ));
    }
    let mut tx = pool().begin().await?;
    let target_exists: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM users WHERE id = $1 FOR SHARE")
            .bind(target_user_id)
            .fetch_optional(&mut *tx)
            .await?;
    if target_exists.is_none() {
        return Err(Error::NotFound);
    }

    let mut request_ids = Vec::with_capacity(changes.len());
    for (case_id, requested_capabilities) in changes {
        let case_exists: Option<i32> =
            sqlx::query_scalar("SELECT 1 FROM cases WHERE id = $1 FOR SHARE")
                .bind(case_id)
                .fetch_optional(&mut *tx)
                .await?;
        if case_exists.is_none() {
            return Err(Error::NotFound);
        }
        let current: Option<Vec<String>> = sqlx::query_scalar(
            "SELECT array_agg(capability ORDER BY capability)
             FROM case_assignments WHERE user_id = $1 AND case_id = $2",
        )
        .bind(target_user_id)
        .bind(case_id)
        .fetch_one(&mut *tx)
        .await?;
        let requested = capability_slugs(requested_capabilities.as_deref());
        if current == requested {
            return Err(Error::NoChange);
        }
        let id = ids::next(&mut *tx, "ar").await?;
        sqlx::query(
            "INSERT INTO admin_requests
                 (id, kind, requested_by, target_user_id, case_id,
                  current_capabilities, requested_capabilities, request_note)
             VALUES ($1, 'case_capabilities', $2, $3, $4, $5, $6, $7)",
        )
        .bind(&id)
        .bind(requester_id)
        .bind(target_user_id)
        .bind(case_id)
        .bind(current)
        .bind(requested)
        .bind(note)
        .execute(&mut *tx)
        .await?;
        request_ids.push(id);
    }

    let sql = format!("{SELECT_REQUEST} WHERE r.id = $1");
    let mut requests = Vec::with_capacity(request_ids.len());
    for id in request_ids {
        let request = sqlx::query_as::<_, RequestRow>(&sql)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?
            .try_into()?;
        requests.push(request);
    }
    tx.commit().await?;
    Ok(requests)
}

pub async fn decide(
    request_id: &str,
    approve: bool,
    actor_id: &str,
    actor_name: &str,
    note: &str,
) -> Result<DecisionOutcome, Error> {
    let mut tx = pool().begin().await?;
    let target_id: Option<String> =
        sqlx::query_scalar("SELECT target_user_id FROM admin_requests WHERE id = $1")
            .bind(request_id)
            .fetch_optional(&mut *tx)
            .await?;
    let target_id = target_id.ok_or(Error::NotFound)?;
    let target_exists: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM users WHERE id = $1 FOR UPDATE")
            .bind(&target_id)
            .fetch_optional(&mut *tx)
            .await?;
    if target_exists.is_none() {
        return Err(Error::NotFound);
    }
    let locked: Option<(
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<Vec<String>>,
        Option<Vec<String>>,
    )> = sqlx::query_as(
        "SELECT kind, status, target_user_id, case_id, previous_role, requested_role,
                current_capabilities, requested_capabilities
         FROM admin_requests WHERE id = $1 FOR UPDATE",
    )
    .bind(request_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((
        kind,
        status,
        locked_target_id,
        case_id,
        current_role,
        requested_role,
        current_caps,
        requested_caps,
    )) = locked
    else {
        return Err(Error::NotFound);
    };
    if status != "pending" {
        return Err(Error::AlreadyResolved);
    }
    if locked_target_id != target_id {
        return Err(Error::Stale);
    }

    let was_unassigned = current_caps.is_none();
    if approve {
        match kind.as_str() {
            "role" => {
                let actual: Option<String> =
                    sqlx::query_scalar("SELECT role FROM users WHERE id = $1 FOR UPDATE")
                        .bind(&target_id)
                        .fetch_optional(&mut *tx)
                        .await?;
                if actual != current_role {
                    return Err(Error::Stale);
                }
                let requested_role = requested_role.ok_or_else(|| {
                    Error::InvalidData("Role request has no requested role.".into())
                })?;
                // Route through the one function allowed to change a role, so the
                // subtype records follow it. It writes the audit entry itself.
                let role = AccountRole::from_slug(&requested_role).ok_or_else(|| {
                    Error::InvalidData(format!("unknown role {requested_role:?}"))
                })?;
                users::set_role_in(&mut tx, &target_id, role, actor_name).await?;
            }
            "case_capabilities" => {
                let case_id = case_id
                    .as_deref()
                    .ok_or_else(|| Error::InvalidData("Case request has no case.".into()))?;
                let actual: Option<Vec<String>> = sqlx::query_scalar(
                    "SELECT array_agg(capability ORDER BY capability)
                     FROM case_assignments WHERE user_id = $1 AND case_id = $2",
                )
                .bind(&target_id)
                .bind(case_id)
                .fetch_one(&mut *tx)
                .await?;
                if actual != current_caps {
                    return Err(Error::Stale);
                }
                sqlx::query("DELETE FROM case_assignments WHERE user_id = $1 AND case_id = $2")
                    .bind(&target_id)
                    .bind(case_id)
                    .execute(&mut *tx)
                    .await?;
                if let Some(capabilities) = &requested_caps {
                    for capability in capabilities {
                        sqlx::query(
                            "INSERT INTO case_assignments (user_id, case_id, capability)
                             VALUES ($1, $2, $3)",
                        )
                        .bind(&target_id)
                        .bind(case_id)
                        .bind(capability)
                        .execute(&mut *tx)
                        .await?;
                    }
                }
                audit::record_in_transaction(
                    &mut tx,
                    audit::Entity::User,
                    &target_id,
                    actor_name,
                    &format!("case:{case_id}"),
                    &capability_text(current_caps.as_deref()),
                    &capability_text(requested_caps.as_deref()),
                )
                .await?;
            }
            other => {
                return Err(Error::InvalidData(format!(
                    "unknown request kind {other:?}"
                )))
            }
        }
    }

    let decision = if approve { "approved" } else { "denied" };
    sqlx::query(
        "UPDATE admin_requests
         SET status = $2, decided_by = $3, decision_note = $4, decided_at = now()
         WHERE id = $1",
    )
    .bind(request_id)
    .bind(decision)
    .bind(actor_id)
    .bind(note)
    .execute(&mut *tx)
    .await?;

    let sql = format!("{SELECT_REQUEST} WHERE r.id = $1");
    let request = sqlx::query_as::<_, RequestRow>(&sql)
        .bind(request_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?
        .try_into()?;
    tx.commit().await?;

    Ok(DecisionOutcome {
        request,
        was_unassigned,
    })
}

async fn get(id: &str) -> Result<AdminRequest, Error> {
    let sql = format!("{SELECT_REQUEST} WHERE r.id = $1");
    let row = sqlx::query_as::<_, RequestRow>(&sql)
        .bind(id)
        .fetch_optional(pool())
        .await?
        .ok_or(Error::NotFound)?;
    row.try_into()
}

// These are admin_request specific helper functions which is why they can be present in this file

fn parse_role(value: Option<String>) -> Result<Option<AccountRole>, Error> {
    value
        .map(|value| {
            AccountRole::from_slug(&value)
                .ok_or_else(|| Error::InvalidData(format!("unknown account role {value:?}")))
        })
        .transpose()
}

fn parse_capabilities(values: Option<Vec<String>>) -> Result<Option<Vec<CaseCapability>>, Error> {
    values
        .map(|values| {
            values
                .into_iter()
                .map(|value| {
                    CaseCapability::from_slug(&value).ok_or_else(|| {
                        Error::InvalidData(format!("unknown case capability {value:?}"))
                    })
                })
                .collect()
        })
        .transpose()
}

fn capability_slugs(capabilities: Option<&[CaseCapability]>) -> Option<Vec<String>> {
    capabilities.map(|capabilities| {
        let mut slugs = capabilities
            .iter()
            .map(|capability| capability.slug().to_string())
            .collect::<Vec<_>>();
        slugs.sort();
        slugs.dedup();
        slugs
    })
}

fn capability_text(capabilities: Option<&[String]>) -> String {
    capabilities
        .map(|values| values.join(", "))
        .unwrap_or_else(|| "not assigned".to_string())
}

fn nonempty_name(name: String, fallback: &str) -> String {
    if name.is_empty() {
        fallback.to_string()
    } else {
        name
    }
}
