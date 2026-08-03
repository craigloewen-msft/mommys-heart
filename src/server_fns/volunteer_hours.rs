//! Volunteer-hour records shared by profile pages and the SSR persistence layer.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
const MAX_MINUTES_PER_ENTRY: i32 = 24 * 60;
#[cfg(feature = "ssr")]
const MAX_DESCRIPTION_CHARS: usize = 500;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolunteerHour {
    pub id: String,
    /// Service date in `YYYY-MM-DD` form.
    pub service_date: String,
    pub duration_minutes: i32,
    pub description: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VolunteerHours {
    pub entries: Vec<VolunteerHour>,
    pub total_minutes: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VolunteerHourInput {
    pub service_date: String,
    pub duration_minutes: i32,
    pub description: String,
}

#[cfg(feature = "ssr")]
fn validate_input(mut input: VolunteerHourInput) -> Result<VolunteerHourInput, ServerFnError> {
    input.service_date = input.service_date.trim().to_string();
    input.description = input.description.trim().to_string();

    let service_date = chrono::NaiveDate::parse_from_str(&input.service_date, "%Y-%m-%d")
        .map_err(|_| ServerFnError::new("Enter a valid service date."))?;
    if service_date > chrono::Local::now().date_naive() {
        return Err(ServerFnError::new(
            "Volunteer hours cannot be logged for a future date.",
        ));
    }
    if !(1..=MAX_MINUTES_PER_ENTRY).contains(&input.duration_minutes) {
        return Err(ServerFnError::new(
            "Hours must be greater than zero and no more than 24 for one date.",
        ));
    }
    if input.description.chars().count() > MAX_DESCRIPTION_CHARS {
        return Err(ServerFnError::new(
            "The description must be 500 characters or fewer.",
        ));
    }
    Ok(input)
}

/// Add time to the signed-in user's own profile.
#[server(prefix = "/api")]
pub async fn add_my_volunteer_hours(
    input: VolunteerHourInput,
) -> Result<VolunteerHours, ServerFnError> {
    use crate::server::db::volunteer_hours;
    use crate::server::permissions::{has_volunteer_access, require_user};

    let user = require_user().await?;
    if !has_volunteer_access(&user) {
        return Err(ServerFnError::new(
            "Volunteer hours are only available for volunteer accounts.",
        ));
    }
    let input = validate_input(input)?;
    volunteer_hours::create(&user.id, &input)
        .await
        .map_err(ServerFnError::new)?;
    volunteer_hours::list_for_user(&user.id)
        .await
        .map_err(ServerFnError::new)
}

/// Update one of the signed-in user's own entries.
#[server(prefix = "/api")]
pub async fn update_my_volunteer_hours(
    entry_id: String,
    input: VolunteerHourInput,
) -> Result<VolunteerHours, ServerFnError> {
    use crate::server::db::volunteer_hours;
    use crate::server::permissions::{has_volunteer_access, require_user};

    let user = require_user().await?;
    if !has_volunteer_access(&user) {
        return Err(ServerFnError::new(
            "Volunteer hours are only available for volunteer accounts.",
        ));
    }
    let input = validate_input(input)?;
    let updated = volunteer_hours::update(&user.id, entry_id.trim(), &input)
        .await
        .map_err(ServerFnError::new)?;
    if !updated {
        return Err(ServerFnError::new("Volunteer hour entry not found."));
    }
    volunteer_hours::list_for_user(&user.id)
        .await
        .map_err(ServerFnError::new)
}

/// Delete one of the signed-in user's own entries.
#[server(prefix = "/api")]
pub async fn delete_my_volunteer_hours(entry_id: String) -> Result<VolunteerHours, ServerFnError> {
    use crate::server::db::volunteer_hours;
    use crate::server::permissions::{has_volunteer_access, require_user};

    let user = require_user().await?;
    if !has_volunteer_access(&user) {
        return Err(ServerFnError::new(
            "Volunteer hours are only available for volunteer accounts.",
        ));
    }
    let deleted = volunteer_hours::delete(&user.id, entry_id.trim())
        .await
        .map_err(ServerFnError::new)?;
    if !deleted {
        return Err(ServerFnError::new("Volunteer hour entry not found."));
    }
    volunteer_hours::list_for_user(&user.id)
        .await
        .map_err(ServerFnError::new)
}
