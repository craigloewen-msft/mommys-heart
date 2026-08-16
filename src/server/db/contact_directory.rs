//! Persistence for the outreach contact directory (SSR only): the reusable
//! classification taxonomy, keyword search, and the communication log.
//!
//! The person record is not duplicated here. A directory row *is* a
//! [`crate::server::db::contacts`] row, read through a flattened projection and
//! written back through that module, so audit entries, default properties, and
//! the account link keep working exactly as they do in Admin.

use std::collections::HashMap;

use crate::server::db::{contacts, ids, organizations, pool};
use crate::server_fns::contact_directory::{
    CommunicationKind, Contact, ContactCategory, ContactCommunication, ContactDetails, ContactInput,
};
use crate::server_fns::contacts::{ContactInput as PersonInput, ContactType};
use crate::server_fns::organizations::{OrganizationInput, OrganizationKind};

const STAMP: &str = "%Y-%m-%d %H:%M";

/// Marks the people this directory created, so their origin is visible in Admin.
const DIRECTORY_SOURCE: &str = "Contact directory";

/// The directory's flat projection of a person: the account owns identity for
/// linked contacts, so those columns are read from `users` just as the Admin
/// directory reads them.
const DIRECTORY_COLUMNS: &str = "c.id,
     coalesce(
         nullif(
             btrim(
                 coalesce(nullif(c.preferred_name, ''),
                          CASE WHEN u.id IS NULL THEN c.first_name ELSE u.first_name END)
                 || ' '
                 || CASE WHEN u.id IS NULL THEN c.last_name ELSE u.last_name END
             ),
             ''
         ),
         o.name,
         ''
     ) AS full_name,
     c.job_title AS title,
     coalesce(o.name, '') AS organization,
     CASE WHEN u.id IS NULL THEN c.email ELSE u.email END AS email,
     CASE WHEN u.id IS NULL THEN c.phone ELSE u.phone END AS phone,
     CASE WHEN u.id IS NULL THEN c.address ELSE u.home_address END AS address,
     c.website, c.updated_at,
     category.id AS category_id, category.name AS category_name,
     category.parent_id, coalesce(parent.name, '') AS parent_name";

const DIRECTORY_JOINS: &str = "FROM contacts c
     LEFT JOIN organizations o ON o.id = c.organization_id
     LEFT JOIN users u ON u.id = c.user_id
     LEFT JOIN contact_category_assignments assignment ON assignment.contact_id = c.id
     LEFT JOIN contact_categories category ON category.id = assignment.category_id
     LEFT JOIN contact_categories parent ON parent.id = category.parent_id";

#[derive(sqlx::FromRow)]
struct CategoryRow {
    id: String,
    name: String,
    parent_id: Option<String>,
    parent_name: String,
}

impl From<CategoryRow> for ContactCategory {
    fn from(row: CategoryRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            parent_id: row.parent_id,
            parent_name: row.parent_name,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ContactRow {
    id: String,
    full_name: String,
    title: String,
    organization: String,
    email: String,
    phone: String,
    address: String,
    website: String,
    updated_at: chrono::DateTime<chrono::Utc>,
    category_id: Option<String>,
    category_name: Option<String>,
    parent_id: Option<String>,
    parent_name: Option<String>,
}

fn stamp(at: chrono::DateTime<chrono::Utc>) -> String {
    at.with_timezone(&chrono::Local).format(STAMP).to_string()
}

fn fold_contacts(rows: Vec<ContactRow>) -> Vec<Contact> {
    let mut contacts = Vec::<Contact>::new();
    let mut indexes = HashMap::<String, usize>::new();
    for row in rows {
        let index = if let Some(index) = indexes.get(&row.id) {
            *index
        } else {
            let index = contacts.len();
            indexes.insert(row.id.clone(), index);
            contacts.push(Contact {
                id: row.id.clone(),
                full_name: row.full_name,
                title: row.title,
                organization: row.organization,
                email: row.email,
                phone: row.phone,
                address: row.address,
                website: row.website,
                categories: Vec::new(),
                updated_at: stamp(row.updated_at),
            });
            index
        };
        if let (Some(id), Some(name)) = (row.category_id, row.category_name) {
            contacts[index].categories.push(ContactCategory {
                id,
                name,
                parent_id: row.parent_id,
                parent_name: row.parent_name.unwrap_or_default(),
            });
        }
    }
    contacts
}

pub async fn list_categories() -> Result<Vec<ContactCategory>, sqlx::Error> {
    sqlx::query_as::<_, CategoryRow>(
        "SELECT category.id, category.name, category.parent_id,
                COALESCE(parent.name, '') AS parent_name
         FROM contact_categories category
         LEFT JOIN contact_categories parent ON parent.id = category.parent_id
         ORDER BY COALESCE(parent.name, category.name), category.parent_id NULLS FIRST,
                  category.name",
    )
    .fetch_all(pool())
    .await
    .map(|rows| rows.into_iter().map(Into::into).collect())
}

pub async fn create_category(name: &str, parent_id: Option<&str>) -> Result<(), sqlx::Error> {
    let id = ids::opaque("cc");
    sqlx::query(
        "INSERT INTO contact_categories (id, name, parent_id)
         VALUES ($1, $2, $3)",
    )
    .bind(id)
    .bind(name)
    .bind(parent_id)
    .execute(pool())
    .await?;
    Ok(())
}

pub async fn category_is_root(category_id: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM contact_categories
             WHERE id = $1 AND parent_id IS NULL
         )",
    )
    .bind(category_id)
    .fetch_one(pool())
    .await
}

/// Archived people are left out: the directory exists to be acted on, and an
/// archived record is deliberately absent from every other picker too.
pub async fn search(query: &str, category_ids: &[String]) -> Result<Vec<Contact>, sqlx::Error> {
    let pattern = format!("%{query}%");
    let rows = sqlx::query_as::<_, ContactRow>(&format!(
        "SELECT {DIRECTORY_COLUMNS}
         {DIRECTORY_JOINS}
         WHERE NOT c.archived
         AND (
             $1 = '' OR c.first_name ILIKE $2 OR c.last_name ILIKE $2
             OR c.preferred_name ILIKE $2 OR u.first_name ILIKE $2 OR u.last_name ILIKE $2
             OR c.job_title ILIKE $2 OR o.name ILIKE $2
             OR c.email ILIKE $2 OR u.email ILIKE $2 OR c.phone ILIKE $2
             OR c.mobile ILIKE $2 OR c.address ILIKE $2 OR c.website ILIKE $2
         )
         AND (
             cardinality($3::text[]) = 0 OR c.id IN (
                 SELECT selected.contact_id
                 FROM contact_category_assignments selected
                 WHERE selected.category_id = ANY($3::text[])
                 GROUP BY selected.contact_id
                 HAVING count(DISTINCT selected.category_id) = cardinality($3::text[])
             )
         )
         ORDER BY lower(c.last_name), lower(c.first_name), lower(coalesce(o.name, '')),
                  category.name"
    ))
    .bind(query)
    .bind(pattern)
    .bind(category_ids)
    .fetch_all(pool())
    .await?;
    Ok(fold_contacts(rows))
}

pub async fn get(contact_id: &str) -> Result<Option<ContactDetails>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ContactRow>(&format!(
        "SELECT {DIRECTORY_COLUMNS}
         {DIRECTORY_JOINS}
         WHERE c.id = $1
         ORDER BY category.name"
    ))
    .bind(contact_id)
    .fetch_all(pool())
    .await?;
    let Some(contact) = fold_contacts(rows).into_iter().next() else {
        return Ok(None);
    };

    #[derive(sqlx::FromRow)]
    struct CommunicationRow {
        id: String,
        kind: String,
        body: String,
        author_name: String,
        occurred_at: chrono::DateTime<chrono::Utc>,
    }

    let communications = sqlx::query_as::<_, CommunicationRow>(
        "SELECT id, kind, body, author_name, occurred_at
         FROM contact_communications
         WHERE contact_id = $1
         ORDER BY occurred_at DESC, created_at DESC",
    )
    .bind(contact_id)
    .fetch_all(pool())
    .await?
    .into_iter()
    .filter_map(|row| {
        Some(ContactCommunication {
            id: row.id,
            kind: CommunicationKind::from_slug(&row.kind)?,
            body: row.body,
            author_name: row.author_name,
            occurred_at: stamp(row.occurred_at),
        })
    })
    .collect();
    Ok(Some(ContactDetails {
        contact,
        communications,
    }))
}

/// Split a directory-entered name into the stored first/last pair. A single word
/// becomes the last name, which is what `contacts_named_check` requires.
fn split_name(full_name: &str) -> (String, String) {
    let trimmed = full_name.trim();
    match trimmed.rsplit_once(char::is_whitespace) {
        Some((first, last)) => (first.trim().to_string(), last.trim().to_string()),
        None => (String::new(), trimmed.to_string()),
    }
}

/// Map the directory's free-text employer onto an organization record, creating
/// one when the name is new. Archived organizations are reused only if nothing
/// active matches, which would fail validation later with a readable message.
async fn resolve_organization(
    name: &str,
    actor_user_id: &str,
    actor: &str,
) -> Result<String, sqlx::Error> {
    let name = name.trim();
    if name.is_empty() {
        return Ok(String::new());
    }
    if let Some(id) = sqlx::query_scalar::<_, String>(
        "SELECT id FROM organizations
         WHERE lower(btrim(name)) = lower(btrim($1)) AND NOT archived
         LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool())
    .await?
    {
        return Ok(id);
    }
    organizations::create(
        &OrganizationInput {
            name: name.to_string(),
            kind: OrganizationKind::Other,
            ..Default::default()
        },
        actor_user_id,
        actor,
    )
    .await
}

/// Create or update a person from the directory form, then rewrite their
/// classifications. Identity fields are written through
/// [`crate::server::db::contacts`]; the fields the directory does not collect
/// keep whatever Admin last set.
pub async fn save(
    contact_id: Option<&str>,
    input: &ContactInput,
    actor_user_id: &str,
    actor: &str,
) -> Result<String, sqlx::Error> {
    let (first_name, last_name) = split_name(&input.full_name);
    let organization_id = resolve_organization(&input.organization, actor_user_id, actor).await?;

    let id = match contact_id {
        Some(id) => {
            let existing = contacts::get(id)
                .await?
                .ok_or(sqlx::Error::RowNotFound)?;
            // A linked person's name, email, phone and address are owned by their
            // account, so editing them here would be silently discarded.
            if existing.has_account() {
                return Err(sqlx::Error::Protocol(
                    "This person signs in with an account; edit them from Admin instead.".into(),
                ));
            }
            let person = PersonInput {
                first_name,
                last_name,
                email: input.email.clone(),
                phone: input.phone.clone(),
                address: input.address.clone(),
                job_title: input.title.clone(),
                organization_id,
                preferred_name: existing.preferred_name,
                mobile: existing.mobile,
                types: existing.types,
                source: existing.source,
                description: existing.description,
                do_not_contact: existing.do_not_contact,
            };
            let person = person.validate().map_err(sqlx::Error::Protocol)?;
            contacts::update(id, &person, actor_user_id, actor).await?;
            id.to_string()
        }
        None => {
            let person = PersonInput {
                first_name,
                last_name,
                email: input.email.clone(),
                phone: input.phone.clone(),
                address: input.address.clone(),
                job_title: input.title.clone(),
                organization_id,
                types: vec![ContactType::Other],
                source: DIRECTORY_SOURCE.to_string(),
                ..Default::default()
            };
            let person = person.validate().map_err(sqlx::Error::Protocol)?;
            contacts::create(&person, actor_user_id, actor).await?
        }
    };

    let mut tx = pool().begin().await?;
    sqlx::query("UPDATE contacts SET website = $2, updated_at = now() WHERE id = $1")
        .bind(&id)
        .bind(&input.website)
        .execute(&mut *tx)
        .await?;

    let valid_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM contact_categories WHERE id = ANY($1::text[])")
            .bind(&input.category_ids)
            .fetch_one(&mut *tx)
            .await?;
    if valid_count != input.category_ids.len() as i64 {
        return Err(sqlx::Error::Protocol(
            "One or more selected contact categories no longer exist.".to_string(),
        ));
    }

    sqlx::query("DELETE FROM contact_category_assignments WHERE contact_id = $1")
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    // Assign each selected classification and its parent. A contact tagged
    // "Family law" is therefore also found by the broader "Legal" filter.
    sqlx::query(
        "INSERT INTO contact_category_assignments (contact_id, category_id)
         SELECT $1, category_id
         FROM (
             SELECT id AS category_id
             FROM contact_categories
             WHERE id = ANY($2::text[])
             UNION
             SELECT parent_id
             FROM contact_categories
             WHERE id = ANY($2::text[]) AND parent_id IS NOT NULL
         ) selected",
    )
    .bind(&id)
    .bind(&input.category_ids)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn add_communication(
    contact_id: &str,
    kind: CommunicationKind,
    body: &str,
    author_id: &str,
    author_name: &str,
) -> Result<(), sqlx::Error> {
    let id = ids::opaque("comm");
    let result = sqlx::query(
        "INSERT INTO contact_communications
             (id, contact_id, kind, body, author_id, author_name)
         SELECT $1, id, $3, $4, $5, $6 FROM contacts WHERE id = $2",
    )
    .bind(id)
    .bind(contact_id)
    .bind(kind.slug())
    .bind(body)
    .bind(author_id)
    .bind(author_name)
    .execute(pool())
    .await?;
    if result.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}
