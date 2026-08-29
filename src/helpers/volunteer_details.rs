//! The volunteer's own details: skills, date of birth, contact, emergency
//! contact, and the optional Social Security Number.
//!
//! Pure functions, so one implementation serves both the browser form and the
//! server function. The SSN is write-only from the browser's side: the read
//! type [`VolunteerDetailsView`] carries a `has_ssn` flag instead.

use serde::{Deserialize, Serialize};

use crate::helpers::dates;

/// The earliest date of birth accepted. Anything before this is a typo rather
/// than a person.
const EARLIEST_BIRTH_YEAR: i32 = 1900;

/// How many digits a US phone number has, and an SSN.
const PHONE_DIGITS: usize = 10;
const SSN_DIGITS: usize = 9;

/// The consent the volunteer gives by filling the contact block in, shown
/// directly above those fields wherever they are edited.
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
    /// Trim everything, format both phones, and reduce the SSN to bare digits.
    /// Always run before [`Self::validate`] and before storing.
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

    /// Whether these details are complete and well-formed; the error is shown to
    /// the volunteer as-is. Age is not gated — paragraph 13 allows minors.
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

/// The read side of [`VolunteerDetails`]: identical but for the SSN, which is
/// reduced to a yes/no so the digits have no field to travel in.
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
    /// rows backfilled by migration and seed.
    pub fn is_present(&self) -> bool {
        !self.skills_focus.is_empty()
            || !self.date_of_birth.is_empty()
            || self.has_ssn
            || !self.emergency_first_name.is_empty()
            || !self.emergency_last_name.is_empty()
            || !self.emergency_phone.is_empty()
    }

    /// Seed an edit form from what is on file. The SSN is necessarily blank, so
    /// an empty box means "keep what is stored" rather than "clear it".
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
/// digits, so validation can quote back what was typed.
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

/// A stored SSN as `000-00-0000`. Only called on the audited reveal path.
pub fn format_ssn(value: &str) -> String {
    let digits = digits(value);
    if digits.len() != SSN_DIGITS {
        return digits;
    }
    format!("{}-{}-{}", &digits[0..3], &digits[3..5], &digits[5..9])
}

/// An ISO `YYYY-MM-DD` date as `MM-DD-YYYY`, the format the Foundation's form
/// uses. Anything unparseable is passed through.
pub fn format_dob(value: &str) -> String {
    dates::to_us(value)
}

/// A date of birth must be a real date, in the past, and not absurdly early.
/// "In the past" is checked against whichever clock is running this code.
fn validate_date_of_birth(value: &str) -> Result<(), String> {
    const MALFORMED: &str = "Enter your date of birth as a valid date.";
    if value.is_empty() {
        return Err("Please give your date of birth.".to_string());
    }
    let (year, _, _) = dates::parse_iso(value).ok_or(MALFORMED)?;
    if year < EARLIEST_BIRTH_YEAR {
        return Err(MALFORMED.to_string());
    }
    if value >= dates::today().as_str() {
        return Err("Your date of birth must be in the past.".to_string());
    }
    Ok(())
}
