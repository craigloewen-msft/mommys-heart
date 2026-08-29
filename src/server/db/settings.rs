//! Persistence for per-user settings: reads and writes the shared
//! [`UserSettings`] aggregate (whose notifications part is
//! [`NotificationSettings`]), plus the recipient resolution used when sending
//! case emails. Database rows are mapped into the domain types here so the
//! shared `server_fns::settings` module stays free of any `sqlx` dependency.

use crate::server::db::pool;
use crate::server_fns::settings::{NotificationSettings, UserSettings};
use crate::server_fns::users::AccountRole;

/// A user who should receive a case notification: their email + display name and
/// the notification settings (the notifications part of their [`UserSettings`])
/// that decide whether we actually send.
#[derive(Clone, Debug)]
pub struct Recipient {
    pub email: String,
    pub name: String,
    pub settings: NotificationSettings,
}

/// A recipient query row: the three contact columns followed by the eight
/// notification flags (already `COALESCE`d to their defaults). sqlx reads tuple
/// rows positionally, so the SELECT just has to list the columns in this order.
type RecipientRow = (
    String,
    String,
    String,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
);

/// Build a [`Recipient`] from a recipient query row.
fn recipient_from_row(row: RecipientRow) -> Recipient {
    let (
        email,
        first_name,
        last_name,
        emails_enabled,
        new_message,
        case_data,
        note_added,
        evidence_changed,
        account_permissions_changed,
        admin_requests,
        admin_activity,
    ) = row;
    Recipient {
        email,
        name: format!("{first_name} {last_name}").trim().to_string(),
        settings: NotificationSettings {
            emails_enabled,
            new_message,
            case_data,
            note_added,
            evidence_changed,
            account_permissions_changed,
            admin_requests,
            admin_activity,
        },
    }
}

/// A single user's settings
pub async fn get_settings(user_id: &str) -> Result<UserSettings, sqlx::Error> {
    let row = sqlx::query_as::<_, (bool, bool, bool, bool, bool, bool, bool, bool)>(
        "SELECT notification_emails_enabled, notification_new_message, notification_case_data,
                notification_note_added, notification_evidence_changed,
                notification_account_permissions_changed, notification_admin_requests,
                notification_admin_activity
         FROM user_settings WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool())
    .await?;

    let notifications = row
        .map(
            |(
                emails_enabled,
                new_message,
                case_data,
                note_added,
                evidence_changed,
                account_permissions_changed,
                admin_requests,
                admin_activity,
            )| {
                NotificationSettings {
                    emails_enabled,
                    new_message,
                    case_data,
                    note_added,
                    evidence_changed,
                    account_permissions_changed,
                    admin_requests,
                    admin_activity,
                }
            },
        )
        .unwrap_or_default();
    Ok(UserSettings { notifications })
}

/// Insert or update a user's settings.
pub async fn upsert_settings(user_id: &str, settings: &UserSettings) -> Result<(), sqlx::Error> {
    let n = &settings.notifications;
    sqlx::query(
        "INSERT INTO user_settings
             (user_id, notification_emails_enabled, notification_new_message,
              notification_case_data, notification_note_added,
              notification_evidence_changed, notification_account_permissions_changed,
              notification_admin_requests, notification_admin_activity)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (user_id) DO UPDATE SET
             notification_emails_enabled   = EXCLUDED.notification_emails_enabled,
             notification_new_message      = EXCLUDED.notification_new_message,
             notification_case_data        = EXCLUDED.notification_case_data,
             notification_note_added       = EXCLUDED.notification_note_added,
             notification_evidence_changed = EXCLUDED.notification_evidence_changed,
             notification_account_permissions_changed = EXCLUDED.notification_account_permissions_changed,
             notification_admin_requests   = EXCLUDED.notification_admin_requests,
             notification_admin_activity   = EXCLUDED.notification_admin_activity",
    )
    .bind(user_id)
    .bind(n.emails_enabled)
    .bind(n.new_message)
    .bind(n.case_data)
    .bind(n.note_added)
    .bind(n.evidence_changed)
    .bind(n.account_permissions_changed)
    .bind(n.admin_requests)
    .bind(n.admin_activity)
    .execute(pool())
    .await?;
    Ok(())
}

/// The recipients for a case notification: every user assigned to the case with
/// the `view_case` capability, excluding `exclude_user_id` (the actor who made
/// the change) and anyone without an email address. Each recipient carries their
/// notification settings (defaulting to all-on when they have no saved row), so
/// the caller can filter by the relevant [`NotificationKind`].
pub async fn recipients_for_case(
    case_id: &str,
    exclude_user_id: &str,
    staff_only: bool,
) -> Result<Vec<Recipient>, sqlx::Error> {
    let rows = sqlx::query_as::<_, RecipientRow>(&format!(
        "SELECT u.email, u.first_name, u.last_name,
                COALESCE(s.notification_emails_enabled,   true),
                COALESCE(s.notification_new_message,      true),
                COALESCE(s.notification_case_data,        true),
                COALESCE(s.notification_note_added,       true),
                COALESCE(s.notification_evidence_changed, true),
                COALESCE(s.notification_account_permissions_changed, true),
                COALESCE(s.notification_admin_requests,   true),
                COALESCE(s.notification_admin_activity,   true)
         FROM case_assignments a
         JOIN users u ON u.id = a.user_id
         LEFT JOIN user_settings s ON s.user_id = u.id
         WHERE a.case_id = $1
           AND a.capability = 'view_case'
           AND a.user_id <> $2
           AND u.email <> ''
           AND (NOT $3 OR u.role <> '{client}')",
        client = AccountRole::Client.slug()
    ))
    .bind(case_id)
    .bind(exclude_user_id)
    .bind(staff_only)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(recipient_from_row).collect())
}

/// A single user's email + display name and their notification settings, for the
/// "assigned to a case" notification whose recipient is the assigned user
/// themselves. Returns `None` when the user is missing or has no email.
pub async fn recipient_for_user(user_id: &str) -> Result<Option<Recipient>, sqlx::Error> {
    let row = sqlx::query_as::<_, RecipientRow>(
        "SELECT u.email, u.first_name, u.last_name,
                COALESCE(s.notification_emails_enabled,   true),
                COALESCE(s.notification_new_message,      true),
                COALESCE(s.notification_case_data,        true),
                COALESCE(s.notification_note_added,       true),
                COALESCE(s.notification_evidence_changed, true),
                COALESCE(s.notification_account_permissions_changed, true),
                COALESCE(s.notification_admin_requests,   true),
                COALESCE(s.notification_admin_activity,   true)
         FROM users u
         LEFT JOIN user_settings s ON s.user_id = u.id
         WHERE u.id = $1 AND u.email <> ''",
    )
    .bind(user_id)
    .fetch_optional(pool())
    .await?;
    Ok(row.map(recipient_from_row))
}

/// Every site administrator with an email address and their notification
/// preferences, so callers can honor the admin-request category.
pub async fn recipients_for_site_admins() -> Result<Vec<Recipient>, sqlx::Error> {
    let rows = sqlx::query_as::<_, RecipientRow>(
        "SELECT u.email, u.first_name, u.last_name,
                COALESCE(s.notification_emails_enabled,   true),
                COALESCE(s.notification_new_message,      true),
                COALESCE(s.notification_case_data,        true),
                COALESCE(s.notification_note_added,       true),
                COALESCE(s.notification_evidence_changed, true),
                COALESCE(s.notification_account_permissions_changed, true),
                COALESCE(s.notification_admin_requests,   true),
                COALESCE(s.notification_admin_activity,   true)
         FROM users u
         LEFT JOIN user_settings s ON s.user_id = u.id
         WHERE u.role = $1 AND u.email <> ''
         ORDER BY u.id",
    )
    .bind(AccountRole::SiteAdmin.slug())
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(recipient_from_row).collect())
}

/// Every site or operations administrator with an email address and their
/// notification preferences.
pub async fn recipients_for_admins() -> Result<Vec<Recipient>, sqlx::Error> {
    let rows = sqlx::query_as::<_, RecipientRow>(
        "SELECT u.email, u.first_name, u.last_name,
                COALESCE(s.notification_emails_enabled,   true),
                COALESCE(s.notification_new_message,      true),
                COALESCE(s.notification_case_data,        true),
                COALESCE(s.notification_note_added,       true),
                COALESCE(s.notification_evidence_changed, true),
                COALESCE(s.notification_account_permissions_changed, true),
                COALESCE(s.notification_admin_requests,   true),
                COALESCE(s.notification_admin_activity,   true)
         FROM users u
         LEFT JOIN user_settings s ON s.user_id = u.id
         WHERE u.role IN ($1, $2) AND u.email <> ''
         ORDER BY u.id",
    )
    .bind(AccountRole::SiteAdmin.slug())
    .bind(AccountRole::OperationsAdmin.slug())
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(recipient_from_row).collect())
}
