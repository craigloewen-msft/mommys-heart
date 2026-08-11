//! Shared vocabulary for the CRM modules: contacts, organizations, case
//! contacts, grants, and funding.
//!
//! These records are organization-wide rather than per-case, so the per-case
//! [`CaseCapability`](crate::server_fns::capabilities::CaseCapability) model does
//! not apply to most of them. Authorization is by account role instead, and the
//! one rule they all share is [`require_staff`]: a client account can never see
//! any of it.

/// Field length caps, applied on the server so the browser cannot post past them.
pub const MAX_NAME: usize = 120;
pub const MAX_SHORT_TEXT: usize = 200;
pub const MAX_LONG_TEXT: usize = 2_000;

/// Define an enum whose variants each carry a stable database slug and a display
/// label, with `ALL`, `slug`, `label`, and `from_slug`.
///
/// The slugs are the same strings the migration's `CHECK` constraints list, so
/// the database and the Rust types cannot drift apart silently.
macro_rules! coded_enum {
    ($name:ident { $($variant:ident => ($slug:literal, $label:literal)),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn slug(self) -> &'static str {
                match self {
                    $($name::$variant => $slug),+
                }
            }

            pub fn label(self) -> &'static str {
                match self {
                    $($name::$variant => $label),+
                }
            }

            pub fn from_slug(s: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|v| v.slug() == s)
            }
        }
    };
}

pub(crate) use coded_enum;

/// Reject a client account. CRM records are internal: clients see no contact,
/// organization, grant, or funding data, and no evidence that any of it exists.
#[cfg(feature = "ssr")]
pub fn require_staff(
    user: &crate::server_fns::users::User,
) -> Result<(), leptos::prelude::ServerFnError> {
    if user.role.has_volunteer_privileges() {
        Ok(())
    } else {
        Err(leptos::prelude::ServerFnError::new(
            "This area is only available to staff accounts.",
        ))
    }
}

/// Format integer minor units as a currency string, e.g. `250000` -> `$2,500.00`.
///
/// Money is carried as `i64` cents everywhere; this is the only place it becomes
/// text, and it never becomes a float.
pub fn format_cents(cents: i64) -> String {
    let negative = cents < 0;
    let abs = cents.unsigned_abs();
    let dollars = abs / 100;
    let remainder = abs % 100;

    // Group the dollar part in threes from the right.
    let digits = dollars.to_string();
    let mut grouped = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }

    let sign = if negative { "-" } else { "" };
    format!("{sign}${grouped}.{remainder:02}")
}

/// Parse a typed amount such as `2,500` or `$2,500.00` into integer minor units.
///
/// Rejects more than two decimal places rather than rounding, so a mistyped
/// amount is corrected by a human instead of silently losing a fraction of a cent.
pub fn parse_cents(input: &str) -> Result<i64, String> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',' && *c != '$')
        .collect();
    if cleaned.is_empty() {
        return Err("Enter an amount.".into());
    }

    let (dollars, cents) = match cleaned.split_once('.') {
        Some((d, c)) => {
            if c.len() > 2 {
                return Err("An amount may have at most two decimal places.".into());
            }
            // "5.5" means 50 cents, not 5.
            let padded = format!("{c:0<2}");
            (if d.is_empty() { "0" } else { d }, padded)
        }
        None => (cleaned.as_str(), "00".to_string()),
    };

    let dollars: i64 = dollars
        .parse()
        .map_err(|_| "Enter a valid amount.".to_string())?;
    let cents: i64 = cents
        .parse()
        .map_err(|_| "Enter a valid amount.".to_string())?;
    if dollars < 0 {
        return Err("An amount cannot be negative.".into());
    }
    dollars
        .checked_mul(100)
        .and_then(|d| d.checked_add(cents))
        .ok_or_else(|| "That amount is too large.".to_string())
}

/// Validate a `YYYY-MM-DD` date, returning it trimmed. An empty string is
/// allowed and means "not set"; callers decide whether that is acceptable.
///
/// Hand-rolled rather than using `chrono`, because this runs in the browser too
/// and `chrono` is an SSR-only dependency. It is a calendar check, not a parse:
/// it rejects month 13 and 31 February, including in non-leap years.
pub fn clean_date(input: &str, label: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let invalid = || format!("Enter a valid {label} as YYYY-MM-DD.");

    let parts: Vec<&str> = trimmed.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return Err(invalid());
    };
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return Err(invalid());
    }
    let year: u32 = year.parse().map_err(|_| invalid())?;
    let month: u32 = month.parse().map_err(|_| invalid())?;
    let day: u32 = day.parse().map_err(|_| invalid())?;

    if !(1..=12).contains(&month) {
        return Err(invalid());
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if leap => 29,
        _ => 28,
    };
    if !(1..=days_in_month).contains(&day) {
        return Err(invalid());
    }
    Ok(trimmed.to_string())
}

/// Trim a field and reject it if it is longer than `max` characters.
pub fn clean_text(input: &str, label: &str, max: usize) -> Result<String, String> {
    let trimmed = input.trim().to_string();
    if trimmed.chars().count() > max {
        return Err(format!("The {label} must be {max} characters or fewer."));
    }
    Ok(trimmed)
}
