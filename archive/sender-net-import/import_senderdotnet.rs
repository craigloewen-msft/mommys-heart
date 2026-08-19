//! One-time importer for the sender.net subscriber CSV export.
//!
//! Temporary migration tooling, not part of the application surface. Every CSV
//! column is preserved: the ones the CRM models become contact fields, the rest
//! become properties under a single "Sender.net import" section.

use std::collections::{HashMap, HashSet};

use crate::server::db::{contact_properties, contacts, organizations, pool};
use crate::server_fns::contact_properties::ContactProperty;
use crate::server_fns::contacts::{ContactInput, ContactType};
use crate::server_fns::organizations::{OrganizationInput, OrganizationKind};

/// The section every imported property is filed under.
const SECTION: &str = "Sender.net import";

/// Columns that become properties, in display order, paired with the label used
/// for them. `Created` is renamed because "Created" already means something else.
const PROPERTY_COLUMNS: &[(&str, &str)] = &[
    ("Groups", "Groups"),
    ("Type", "Type"),
    ("Tag", "Tag"),
    ("Invite", "Invite"),
    ("Email status", "Email status"),
    ("Sms status", "SMS status"),
    ("Transactional Email Status", "Transactional email status"),
    ("Transactional Sms Status", "Transactional SMS status"),
    ("Location", "Location"),
    ("Created", "Subscribed on"),
    ("Agreement Date", "Agreement date"),
    ("Secondary Email", "Secondary email"),
    ("Secondary Website", "Secondary website"),
    ("Company Website", "Company website"),
    ("Secondary Phone Number", "Secondary phone"),
    ("Fax", "Fax"),
    ("Office Number", "Office number"),
    ("Home", "Home"),
    ("Secondary address", "Secondary address"),
    ("Orders Count", "Orders count"),
    ("Total spent", "Total spent"),
    ("Last order number", "Last order number"),
    ("Currency", "Currency"),
];

/// Segment labels that map onto a real [`ContactType`]. Everything else in
/// Groups/Type/Tag is an outreach segment, preserved as a property only.
fn contact_type_for(label: &str) -> Option<ContactType> {
    match label {
        "donor" => Some(ContactType::Donor),
        "volunteer" => Some(ContactType::Volunteer),
        "board member" => Some(ContactType::BoardMember),
        "attorney" | "lawyer" => Some(ContactType::Attorney),
        "client" | "parent client" => Some(ContactType::Client),
        "vendor" => Some(ContactType::ServiceProvider),
        _ => None,
    }
}

/// Drop the invisible bidi overrides the phone columns carry.
fn strip_marks(value: &str) -> String {
    value
        .chars()
        .filter(|c| !matches!(c, '\u{202a}'..='\u{202e}' | '\u{200e}' | '\u{200f}' | '\u{feff}'))
        .collect()
}

/// Collapse whitespace, including the embedded newlines several address cells
/// carry, so a short-text field stays on one line.
fn tidy(value: &str) -> String {
    strip_marks(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Long free text keeps its internal line breaks; only the marks and the outer
/// whitespace go.
fn tidy_multiline(value: &str) -> String {
    strip_marks(value).trim().to_string()
}

/// A capitalized fallback surname from an email local part, for the rows that
/// carry no name at all.
fn name_from_email(email: &str) -> String {
    let local = email.split('@').next().unwrap_or(email);
    let cleaned: String = local
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let word = cleaned
        .split_whitespace()
        .next()
        .unwrap_or("Unknown")
        .to_string();
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Unknown".to_string(),
    }
}

/// What one CSV row becomes, before anything is written.
struct Planned {
    email: String,
    input: ContactInput,
    website: String,
    organization_name: String,
    properties: Vec<ContactProperty>,
    /// Set when the name had to be repaired to satisfy `contacts_named_check`.
    repair: Option<String>,
}

fn get<'a>(row: &'a HashMap<String, String>, column: &str) -> &'a str {
    row.get(column).map(String::as_str).unwrap_or_default()
}

fn property(key: &str, value: String) -> ContactProperty {
    ContactProperty {
        key: key.to_string(),
        value,
        section: SECTION.to_string(),
    }
}

/// Map one CSV row onto a contact, its organization, and its properties.
fn plan_row(row: &HashMap<String, String>) -> Planned {
    let email = tidy(get(row, "Email"));

    let mut first_name = tidy(get(row, "First name"));
    let mut last_name = tidy(get(row, "Last name"));
    let organization_name = tidy(get(row, "Company"));
    let mut repair = None;

    // `contacts_named_check` wants a surname or an organization. Follow the
    // directory's convention, where a lone word is the last name.
    if last_name.is_empty() && organization_name.is_empty() {
        if let Some((first, last)) = first_name.clone().rsplit_once(' ') {
            first_name = first.trim().to_string();
            last_name = last.trim().to_string();
            repair = Some(format!("split into '{first_name}' + '{last_name}'"));
        } else if !first_name.is_empty() {
            last_name = std::mem::take(&mut first_name);
            repair = Some(format!("moved '{last_name}' into the last name"));
        } else {
            last_name = name_from_email(&email);
            repair = Some(format!("derived '{last_name}' from the email address"));
        }
    }

    // The SMS-subscribed `Phone` becomes the mobile when a separate primary exists.
    let sender_phone = tidy(get(row, "Phone"));
    let primary_phone = tidy(get(row, "Primary Phone Number"));
    let (phone, mobile) = if primary_phone.is_empty() {
        (sender_phone, String::new())
    } else {
        (primary_phone, sender_phone)
    };

    // The mailing address wins the field; a differing street address is kept.
    let mailing = tidy(get(row, "Mailing Address"));
    let street = tidy(get(row, "Address"));
    let (address, spare_address) = if mailing.is_empty() {
        (street, String::new())
    } else if street.eq_ignore_ascii_case(&mailing) {
        (mailing, String::new())
    } else {
        (mailing, street)
    };

    // Types come from the union of the three segment columns. sender.net uses
    // both ',' and '.' as separators.
    let mut types: Vec<ContactType> = Vec::new();
    for column in ["Groups", "Type", "Tag"] {
        for part in get(row, column).split([',', '.']) {
            let lowered = tidy(part).to_lowercase();
            let label = lowered.strip_prefix("type:").unwrap_or(&lowered).trim();
            if let Some(contact_type) = contact_type_for(label) {
                if !types.contains(&contact_type) {
                    types.push(contact_type);
                }
            }
        }
    }
    if types.is_empty() {
        types.push(ContactType::Other);
    }

    // Only an explicit opt-out suppresses outreach. A bounce is a deliverability
    // failure, not a request to stop contacting.
    let email_status = tidy(get(row, "Email status")).to_lowercase();
    let do_not_contact = matches!(email_status.as_str(), "unsubscribed" | "spam_reported");

    let mut properties: Vec<ContactProperty> = PROPERTY_COLUMNS
        .iter()
        .filter_map(|(column, label)| {
            let value = tidy(get(row, column));
            (!value.is_empty()).then(|| property(label, value))
        })
        .collect();

    if !spare_address.is_empty() {
        properties.push(property("Address", spare_address));
    }

    // Keep the export's own full name only when it says more than first + last.
    let full_name = tidy(get(row, "Full Name"));
    if !full_name.is_empty() && full_name != format!("{first_name} {last_name}").trim() {
        properties.push(property("Full name", full_name));
    }

    Planned {
        email: email.clone(),
        input: ContactInput {
            first_name,
            last_name,
            preferred_name: String::new(),
            email,
            phone,
            mobile,
            address,
            job_title: tidy(get(row, "Title")),
            organization_id: String::new(),
            types,
            source: "sender.net".to_string(),
            description: tidy_multiline(get(row, "Notes")),
            do_not_contact,
        },
        website: tidy(get(row, "Website")),
        organization_name,
        properties,
        repair,
    }
}

/// Read the export, then create the organizations and contacts it describes.
///
/// Idempotent by email: a contact whose address already exists is skipped, so an
/// interrupted run can simply be repeated.
pub async fn run(path: &str, dry_run: bool, actor_email: &str) -> Result<(), String> {
    let (actor_user_id, actor) = resolve_actor(actor_email).await?;

    let mut reader =
        csv::Reader::from_path(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let headers = reader
        .headers()
        .map_err(|e| format!("cannot read the header row: {e}"))?
        .clone();

    let mut planned = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record = record.map_err(|e| format!("row {}: {e}", index + 2))?;
        // The file is UTF-8 with a BOM, which would otherwise corrupt the first
        // header name.
        let row: HashMap<String, String> = headers
            .iter()
            .zip(record.iter())
            .map(|(h, v)| (strip_marks(h).trim().to_string(), v.to_string()))
            .collect();
        let plan = plan_row(&row);
        if plan.email.is_empty() {
            tracing::warn!("row {} has no email address; skipped", index + 2);
            continue;
        }
        planned.push(plan);
    }

    let total = planned.len();
    let existing = existing_emails().await?;
    let (to_create, skipped): (Vec<_>, Vec<_>) = planned
        .into_iter()
        .partition(|p| !existing.contains(&p.email.to_lowercase()));

    // Distinct employers, matched case-insensitively like the directory does.
    let organization_names: Vec<String> = {
        let mut seen = HashSet::new();
        to_create
            .iter()
            .map(|p| p.organization_name.clone())
            .filter(|name| !name.is_empty())
            .filter(|name| seen.insert(name.to_lowercase()))
            .collect()
    };

    let flagged = to_create.iter().filter(|p| p.input.do_not_contact).count();
    let repairs: Vec<&Planned> = to_create.iter().filter(|p| p.repair.is_some()).collect();

    tracing::info!("read {total} rows from {path}");
    tracing::info!(
        "  {} to create, {} already present",
        to_create.len(),
        skipped.len()
    );
    tracing::info!("  {} organizations to resolve", organization_names.len());
    tracing::info!("  {flagged} flagged do-not-contact");
    tracing::info!("  {} names repaired:", repairs.len());
    for plan in &repairs {
        tracing::info!(
            "      {} - {}",
            plan.email,
            plan.repair.as_deref().unwrap_or_default()
        );
    }

    if dry_run {
        tracing::info!("dry run: nothing was written");
        return Ok(());
    }

    // Organizations first, so every contact can be filed on its way in.
    let mut organization_ids: HashMap<String, String> = HashMap::new();
    for name in organization_names {
        let id = resolve_organization(&name, &actor_user_id, &actor).await?;
        organization_ids.insert(name.to_lowercase(), id);
    }
    tracing::info!("{} organizations ready", organization_ids.len());

    let mut created = 0usize;
    let mut failed = 0usize;
    for mut plan in to_create {
        if let Some(id) = organization_ids.get(&plan.organization_name.to_lowercase()) {
            plan.input.organization_id = id.clone();
        }

        // Reuse the app's own validation so no database CHECK can be bypassed.
        let input = match plan.input.validate() {
            Ok(input) => input,
            Err(error) => {
                failed += 1;
                tracing::error!("{}: {error}", plan.email);
                continue;
            }
        };

        let contact_id = match contacts::create(&input, &actor_user_id, &actor).await {
            Ok(id) => id,
            Err(error) => {
                failed += 1;
                tracing::error!("{}: {error}", plan.email);
                continue;
            }
        };

        // `contacts::create` does not write `website` - the directory owns that
        // column - so set it the same way `contact_directory::save` does.
        if !plan.website.is_empty() {
            sqlx::query("UPDATE contacts SET website = $2, updated_at = now() WHERE id = $1")
                .bind(&contact_id)
                .bind(&plan.website)
                .execute(pool())
                .await
                .map_err(|e| format!("{}: setting the website: {e}", plan.email))?;
        }

        // Append the import rows after the code-owned defaults the create just
        // inserted, so those keep their ordinals.
        if !plan.properties.is_empty() {
            let mut properties = contact_properties::list(&contact_id)
                .await
                .map_err(|e| format!("{}: reading properties: {e}", plan.email))?;
            properties.extend(plan.properties);
            contact_properties::replace(&contact_id, properties, &actor_user_id, &actor)
                .await
                .map_err(|e| format!("{}: writing properties: {e}", plan.email))?;
        }

        created += 1;
        if created % 100 == 0 {
            tracing::info!("  {created} contacts created");
        }
    }

    tracing::info!(
        "done: {created} created, {} skipped, {failed} failed",
        skipped.len()
    );
    Ok(())
}

/// The importer writes as a real account, so the Change Log attributes it to a
/// person and `audit_log.actor_user_id` satisfies its foreign key.
async fn resolve_actor(email: &str) -> Result<(String, String), String> {
    let (id, first_name, last_name) = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, first_name, last_name FROM users WHERE lower(email) = lower($1)",
    )
    .bind(email)
    .fetch_optional(pool())
    .await
    .map_err(|e| format!("looking up the actor: {e}"))?
    .ok_or_else(|| format!("no account has the email {email}"))?;

    let name = format!("{first_name} {last_name}").trim().to_string();
    Ok((id, if name.is_empty() { email.to_string() } else { name }))
}

async fn existing_emails() -> Result<HashSet<String>, String> {
    let rows =
        sqlx::query_scalar::<_, String>("SELECT lower(email) FROM contacts WHERE btrim(email) <> ''")
            .fetch_all(pool())
            .await
            .map_err(|e| format!("reading the existing contacts: {e}"))?;
    Ok(rows.into_iter().collect())
}

/// Reuse an active organization with this name, or create one.
async fn resolve_organization(
    name: &str,
    actor_user_id: &str,
    actor: &str,
) -> Result<String, String> {
    if let Some(id) = sqlx::query_scalar::<_, String>(
        "SELECT id FROM organizations
         WHERE lower(btrim(name)) = lower(btrim($1)) AND NOT archived
         LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool())
    .await
    .map_err(|e| format!("looking up the organization {name}: {e}"))?
    {
        return Ok(id);
    }

    let input = OrganizationInput {
        name: name.to_string(),
        kind: OrganizationKind::Other,
        ..Default::default()
    }
    .validate()
    .map_err(|e| format!("organization {name}: {e}"))?;

    organizations::create(&input, actor_user_id, actor)
        .await
        .map_err(|e| format!("creating the organization {name}: {e}"))
}
