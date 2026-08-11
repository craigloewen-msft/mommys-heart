//! Contact persistence (SSR only): the people the foundation knows.
//!
//! The account link is the delicate part. `contacts.user_id` is nullable and
//! uniquely indexed, and reads join `users` for the email and role rather than
//! copying them: a contact must never become a second source of truth for who
//! someone is, or a second way to sign in.

use crate::server::db::{audit, ids, pool};
use crate::server_fns::contacts::{
    Contact, ContactFilters, ContactInput, ContactType, LinkableAccount,
};
use crate::server_fns::pagination::Page;
use crate::server_fns::users::AccountRole;

#[derive(sqlx::FromRow)]
struct ContactRow {
    id: String,
    first_name: String,
    last_name: String,
    preferred_name: String,
    email: String,
    phone: String,
    mobile: String,
    address: String,
    job_title: String,
    organization_id: Option<String>,
    organization_name: Option<String>,
    types: Vec<String>,
    source: String,
    description: String,
    do_not_contact: bool,
    archived: bool,
    user_id: Option<String>,
    linked_email: Option<String>,
    linked_role: Option<String>,
}

impl From<ContactRow> for Contact {
    fn from(row: ContactRow) -> Self {
        Self {
            id: row.id,
            first_name: row.first_name,
            last_name: row.last_name,
            preferred_name: row.preferred_name,
            email: row.email,
            phone: row.phone,
            mobile: row.mobile,
            address: row.address,
            job_title: row.job_title,
            organization_id: row.organization_id.unwrap_or_default(),
            organization_name: row.organization_name.unwrap_or_default(),
            // An unknown slug would mean the CHECK constraint was bypassed; drop
            // it rather than failing the whole read.
            types: row
                .types
                .iter()
                .filter_map(|slug| ContactType::from_slug(slug))
                .collect(),
            source: row.source,
            description: row.description,
            do_not_contact: row.do_not_contact,
            archived: row.archived,
            user_id: row.user_id.unwrap_or_default(),
            linked_email: row.linked_email.unwrap_or_default(),
            linked_role: row.linked_role.as_deref().and_then(AccountRole::from_slug),
        }
    }
}

/// The `SELECT` list every read shares. The organization name and the account's
/// email/role are joined in for display and stay owned by their own tables.
const SELECT_COLUMNS: &str = "c.id, c.first_name, c.last_name, c.preferred_name, c.email,
     c.phone, c.mobile, c.address, c.job_title, c.organization_id,
     o.name AS organization_name, c.types, c.source, c.description,
     c.do_not_contact, c.archived, c.user_id,
     u.email AS linked_email, u.role AS linked_role";

const FROM_JOINS: &str = "FROM contacts c
     LEFT JOIN organizations o ON o.id = c.organization_id
     LEFT JOIN users u ON u.id = c.user_id";

/// One page of the directory, filtered and searched in SQL so the browser never
/// receives rows it did not ask for.
pub async fn page(
    filters: &ContactFilters,
    offset: i64,
    limit: i64,
) -> Result<Page<Contact>, sqlx::Error> {
    let keyword = filters.keyword.trim();
    let contact_type = filters.contact_type.map(|t| t.slug()).unwrap_or_default();
    let organization_id = filters.organization_id.trim();

    let where_sql = "WHERE ($1 OR NOT c.archived)
           AND ($2 = '' OR $2 = ANY(c.types))
           AND ($3 = '' OR c.organization_id = $3)
           AND ($4 = '' OR c.first_name ILIKE '%' || $4 || '%'
                        OR c.last_name ILIKE '%' || $4 || '%'
                        OR c.preferred_name ILIKE '%' || $4 || '%'
                        OR c.email ILIKE '%' || $4 || '%'
                        OR c.phone ILIKE '%' || $4 || '%'
                        OR c.mobile ILIKE '%' || $4 || '%'
                        OR o.name ILIKE '%' || $4 || '%')";

    let total: i64 = sqlx::query_scalar(&format!("SELECT count(*) {FROM_JOINS} {where_sql}"))
        .bind(filters.include_archived)
        .bind(contact_type)
        .bind(organization_id)
        .bind(keyword)
        .fetch_one(pool())
        .await?;

    let rows = sqlx::query_as::<_, ContactRow>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOINS} {where_sql}
         ORDER BY c.archived ASC, lower(c.last_name) ASC, lower(c.first_name) ASC
         OFFSET $5 LIMIT $6"
    ))
    .bind(filters.include_archived)
    .bind(contact_type)
    .bind(organization_id)
    .bind(keyword)
    .bind(offset.max(0))
    .bind(limit.clamp(1, 200))
    .fetch_all(pool())
    .await?;

    Ok(Page {
        items: rows.into_iter().map(Into::into).collect(),
        total,
    })
}

pub async fn get(id: &str) -> Result<Option<Contact>, sqlx::Error> {
    let row = sqlx::query_as::<_, ContactRow>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOINS} WHERE c.id = $1"
    ))
    .bind(id)
    .fetch_optional(pool())
    .await?;
    Ok(row.map(Into::into))
}

/// The contact linked to an account, if there is one. Used by the person page to
/// go from a user to their CRM record.
pub async fn for_user(user_id: &str) -> Result<Option<Contact>, sqlx::Error> {
    let row = sqlx::query_as::<_, ContactRow>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOINS} WHERE c.user_id = $1"
    ))
    .bind(user_id)
    .fetch_optional(pool())
    .await?;
    Ok(row.map(Into::into))
}

/// Active contacts as `(id, label)` for pickers, newest surname order.
pub async fn active_options() -> Result<Vec<(String, String)>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.id,
                btrim(coalesce(nullif(c.preferred_name, ''), c.first_name) || ' ' || c.last_name)
                  || coalesce(' (' || nullif(o.name, '') || ')', '')
         FROM contacts c
         LEFT JOIN organizations o ON o.id = c.organization_id
         WHERE NOT c.archived
         ORDER BY lower(c.last_name) ASC, lower(c.first_name) ASC",
    )
    .fetch_all(pool())
    .await?;
    Ok(rows)
}

fn type_slugs(input: &ContactInput) -> Vec<String> {
    input.types.iter().map(|t| t.slug().to_string()).collect()
}

/// An empty organization id means "not filed under one", which is stored NULL.
fn optional(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

pub async fn create(input: &ContactInput, actor: &str) -> Result<String, sqlx::Error> {
    let id = ids::next(pool(), "ct").await?;
    let mut tx = pool().begin().await?;
    sqlx::query(
        "INSERT INTO contacts
             (id, first_name, last_name, preferred_name, email, phone, mobile,
              address, job_title, organization_id, types, source, description,
              do_not_contact)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
    )
    .bind(&id)
    .bind(&input.first_name)
    .bind(&input.last_name)
    .bind(&input.preferred_name)
    .bind(&input.email)
    .bind(&input.phone)
    .bind(&input.mobile)
    .bind(&input.address)
    .bind(&input.job_title)
    .bind(optional(&input.organization_id))
    .bind(type_slugs(input))
    .bind(&input.source)
    .bind(&input.description)
    .bind(input.do_not_contact)
    .execute(&mut *tx)
    .await?;
    audit::record_in_transaction(
        &mut tx,
        audit::Entity::Contact,
        &id,
        actor,
        "contact",
        "",
        "created",
    )
    .await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn update(id: &str, input: &ContactInput, actor: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    let updated = sqlx::query(
        "UPDATE contacts
         SET first_name = $2, last_name = $3, preferred_name = $4, email = $5,
             phone = $6, mobile = $7, address = $8, job_title = $9,
             organization_id = $10, types = $11, source = $12, description = $13,
             do_not_contact = $14, updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(&input.first_name)
    .bind(&input.last_name)
    .bind(&input.preferred_name)
    .bind(&input.email)
    .bind(&input.phone)
    .bind(&input.mobile)
    .bind(&input.address)
    .bind(&input.job_title)
    .bind(optional(&input.organization_id))
    .bind(type_slugs(input))
    .bind(&input.source)
    .bind(&input.description)
    .bind(input.do_not_contact)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    audit::record_in_transaction(
        &mut tx,
        audit::Entity::Contact,
        id,
        actor,
        "contact",
        "",
        "updated",
    )
    .await?;
    tx.commit().await
}

pub async fn set_archived(id: &str, archived: bool, actor: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    let updated =
        sqlx::query("UPDATE contacts SET archived = $2, updated_at = now() WHERE id = $1")
            .bind(id)
            .bind(archived)
            .execute(&mut *tx)
            .await?;
    if updated.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    audit::record_in_transaction(
        &mut tx,
        audit::Entity::Contact,
        id,
        actor,
        "contact",
        "",
        if archived { "archived" } else { "restored" },
    )
    .await?;
    tx.commit().await
}

/// Accounts that no contact points at yet, newest surname order.
pub async fn unlinked_accounts() -> Result<Vec<LinkableAccount>, sqlx::Error> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT u.id, btrim(u.first_name || ' ' || u.last_name), u.email
         FROM users u
         WHERE NOT EXISTS (SELECT 1 FROM contacts c WHERE c.user_id = u.id)
         ORDER BY lower(u.last_name) ASC, lower(u.first_name) ASC",
    )
    .fetch_all(pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, email)| LinkableAccount { id, name, email })
        .collect())
}

/// Link this contact to an account, or unlink it when `user_id` is `None`.
///
/// Only ever writes `contacts.user_id`: the account itself is untouched, so
/// unlinking leaves the person able to sign in exactly as before.
pub async fn set_account(id: &str, user_id: Option<&str>, actor: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    let updated = sqlx::query("UPDATE contacts SET user_id = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    if updated.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    audit::record_in_transaction(
        &mut tx,
        audit::Entity::Contact,
        id,
        actor,
        "account link",
        "",
        if user_id.is_some() {
            "linked"
        } else {
            "unlinked"
        },
    )
    .await?;
    tx.commit().await
}
