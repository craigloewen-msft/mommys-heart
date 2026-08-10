//! The volunteer's own details: skills, date of birth, contact, emergency
//! contact, and the optional Social Security Number.
//!
//! Pure data and pure functions, so the one implementation serves both the
//! application form in the browser and the server function that stores it —
//! the same arrangement as [`crate::helpers::case_intake`]. Validating in both
//! places from one source means the form's message and the server's refusal can
//! never drift apart.
//!
//! # The Social Security Number
//!
//! [`VolunteerDetails::ssn`] is the *only* place the digits appear in a type
//! that travels. It is write-only from the browser's point of view: nothing
//! sends it back, and the read side ([`VolunteerDetailsView`]) carries a
//! `has_ssn` flag instead. Reading the number is a separate, site-admin-only,
//! audited server function.

use serde::{Deserialize, Serialize};

/// The earliest date of birth accepted. Anything before this is a typo rather
/// than a person.
const EARLIEST_BIRTH_YEAR: i32 = 1900;

/// How many digits a US phone number has, and an SSN.
const PHONE_DIGITS: usize = 10;
const SSN_DIGITS: usize = 9;

/// The consent the volunteer gives by filling the contact block in. Shown
/// directly above those fields wherever they are edited, so it is never
/// separated from what it governs.
pub const BACKGROUND_CHECK_CONSENT: &str = "By providing the information below, Volunteer consents to the Foundation performing a background check.";

/// What a volunteer submits about themselves. Every field is a string because
/// this is form input; [`VolunteerDetails::normalized`] is what turns it into
/// storable values.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VolunteerDetails {
    pub skills_focus: String,
    /// ISO `YYYY-MM-DD`, as an `<input type="date">` produces. Displayed as
    /// `MM-DD-YYYY` by [`format_dob`].
    pub date_of_birth: String,
    /// Optional. Digits only after normalization; empty means "not provided".
    pub ssn: String,
    pub phone: String,
    pub emergency_first_name: String,
    pub emergency_last_name: String,
    pub emergency_relationship: String,
    pub emergency_phone: String,
}

impl VolunteerDetails {
    /// Trim everything, format both phone numbers consistently, and reduce the
    /// SSN to bare digits. Always run before [`Self::validate`] and before
    /// storing, so what is checked is what is written.
    pub fn normalized(&self) -> Self {
        Self {
            skills_focus: self.skills_focus.trim().to_string(),
            date_of_birth: self.date_of_birth.trim().to_string(),
            ssn: digits(&self.ssn),
            phone: normalize_phone(&self.phone),
            emergency_first_name: self.emergency_first_name.trim().to_string(),
            emergency_last_name: self.emergency_last_name.trim().to_string(),
            emergency_relationship: self.emergency_relationship.trim().to_string(),
            emergency_phone: normalize_phone(&self.emergency_phone),
        }
    }

    /// Whether these details are complete and well-formed. The error is written
    /// to be shown to the volunteer as-is.
    ///
    /// Age is deliberately not gated: paragraph 13 of the agreement
    /// contemplates a minor volunteering with a guardian's permission.
    pub fn validate(&self) -> Result<(), String> {
        if self.skills_focus.trim().is_empty() {
            return Err("Please describe your skills and area of focus.".to_string());
        }
        validate_date_of_birth(self.date_of_birth.trim())?;
        if !self.ssn.is_empty() && digits(&self.ssn).len() != SSN_DIGITS {
            return Err(
                "A Social Security Number has 9 digits. Leave it blank to skip it.".to_string(),
            );
        }
        if format_phone(&self.phone).is_none() {
            return Err(
                "Enter your phone number as 10 digits, for example (555) 123-4567.".to_string(),
            );
        }
        if self.emergency_first_name.trim().is_empty() || self.emergency_last_name.trim().is_empty()
        {
            return Err("Please give your emergency contact's first and last name.".to_string());
        }
        if format_phone(&self.emergency_phone).is_none() {
            return Err(
                "Enter your emergency contact's phone number as 10 digits, for example (555) 123-4567."
                    .to_string(),
            );
        }
        Ok(())
    }
}

/// The read side of [`VolunteerDetails`]: what a browser is allowed to see.
///
/// Structurally identical but for the SSN, which is reduced to a yes/no. There
/// is deliberately no field here the digits could travel in, not even a masked
/// suffix — the only way to see the number is the audited reveal.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VolunteerDetailsView {
    pub skills_focus: String,
    /// ISO `YYYY-MM-DD`, or empty for a volunteer who predates this form.
    pub date_of_birth: String,
    /// Whether a Social Security Number is on file. Never the number itself.
    pub has_ssn: bool,
    pub phone: String,
    pub emergency_first_name: String,
    pub emergency_last_name: String,
    pub emergency_relationship: String,
    pub emergency_phone: String,
}

impl VolunteerDetailsView {
    /// Whether this volunteer has filled the details form in at all. False for
    /// the rows backfilled by migration and seed, which predate it.
    pub fn is_present(&self) -> bool {
        !self.skills_focus.is_empty()
            || !self.date_of_birth.is_empty()
            || self.has_ssn
            || !self.emergency_first_name.is_empty()
            || !self.emergency_last_name.is_empty()
            || !self.emergency_phone.is_empty()
    }

    /// Seed an edit form from what is on file. The SSN is necessarily blank:
    /// the browser was never sent it, so an empty box means "keep what is
    /// stored" rather than "clear it".
    pub fn to_edit(&self) -> VolunteerDetails {
        VolunteerDetails {
            skills_focus: self.skills_focus.clone(),
            date_of_birth: self.date_of_birth.clone(),
            ssn: String::new(),
            phone: self.phone.clone(),
            emergency_first_name: self.emergency_first_name.clone(),
            emergency_last_name: self.emergency_last_name.clone(),
            emergency_relationship: self.emergency_relationship.clone(),
            emergency_phone: self.emergency_phone.clone(),
        }
    }

    /// The emergency contact's full name, or empty when none is recorded.
    pub fn emergency_full_name(&self) -> String {
        format!("{} {}", self.emergency_first_name, self.emergency_last_name)
            .trim()
            .to_string()
    }
}

/// Just the digits of `value`.
fn digits(value: &str) -> String {
    value.chars().filter(char::is_ascii_digit).collect()
}

/// A phone number as `(000) 000-0000`, or the trimmed input when it is not ten
/// digits — normalization must not destroy what the person typed, so validation
/// can quote it back at them.
fn normalize_phone(value: &str) -> String {
    format_phone(value).unwrap_or_else(|| value.trim().to_string())
}

/// `value` as `(000) 000-0000`, or `None` unless it holds exactly ten digits.
pub fn format_phone(value: &str) -> Option<String> {
    let digits = digits(value);
    if digits.len() != PHONE_DIGITS {
        return None;
    }
    Some(format!(
        "({}) {}-{}",
        &digits[0..3],
        &digits[3..6],
        &digits[6..10]
    ))
}

/// A stored SSN as `000-00-0000`. Only ever called on the reveal path, where an
/// administrator has explicitly asked for the number and the ask was audited.
pub fn format_ssn(value: &str) -> String {
    let digits = digits(value);
    if digits.len() != SSN_DIGITS {
        return digits;
    }
    format!("{}-{}-{}", &digits[0..3], &digits[3..5], &digits[5..9])
}

/// An ISO `YYYY-MM-DD` date as `MM-DD-YYYY`, the format the Foundation's form
/// uses. Anything unparseable is passed through rather than hidden.
pub fn format_dob(value: &str) -> String {
    match parse_iso_date(value.trim()) {
        Some((year, month, day)) => format!("{month:02}-{day:02}-{year:04}"),
        None => value.trim().to_string(),
    }
}

/// Split an ISO `YYYY-MM-DD` date, checking the parts are in range. Does not
/// check the day against the month's real length; the date input the form uses
/// cannot produce a 31st of February.
fn parse_iso_date(value: &str) -> Option<(i32, u32, u32)> {
    let mut parts = value.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

/// A date of birth must be a real date, in the past, and not absurdly early.
///
/// "In the past" is checked against the caller's clock in the browser and the
/// server's clock on the server. That is deliberate: a date of birth that is
/// tomorrow in one timezone and today in another is not a case worth failing a
/// submission over.
fn validate_date_of_birth(value: &str) -> Result<(), String> {
    const MALFORMED: &str = "Enter your date of birth as a valid date.";
    if value.is_empty() {
        return Err("Please give your date of birth.".to_string());
    }
    let (year, _, _) = parse_iso_date(value).ok_or(MALFORMED)?;
    if year < EARLIEST_BIRTH_YEAR {
        return Err(MALFORMED.to_string());
    }
    if value >= today().as_str() {
        return Err("Your date of birth must be in the past.".to_string());
    }
    Ok(())
}

/// Today as ISO `YYYY-MM-DD`.
///
/// [`crate::state::today`] is a browser clock and stubs to 1970 when there is no
/// `hydrate` feature, so the server must read its own clock instead — sharing
/// the browser's would reject every real date of birth server-side.
fn today() -> String {
    #[cfg(feature = "ssr")]
    {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }
    #[cfg(not(feature = "ssr"))]
    {
        crate::state::today()
    }
}
