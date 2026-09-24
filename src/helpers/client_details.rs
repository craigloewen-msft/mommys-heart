//! What a prospective client signs the Service Agreement with: their electronic
//! signature, contact details, emergency contact, the services they are asking
//! for, and the optional block a parent or guardian fills in for a minor.
//!
//! The client-side twin of [`volunteer_details`](crate::helpers::volunteer_details):
//! pure functions, so one implementation serves both the browser form and the
//! server function that stores it.

use serde::{Deserialize, Serialize};

use crate::helpers::dates;
use crate::helpers::volunteer_details::format_phone;

/// The earliest date of birth accepted. Anything before this is a typo rather
/// than a person.
const EARLIEST_BIRTH_YEAR: i32 = 1900;

/// The services a recipient may ask for, in the order they are shown.
pub const SERVICES_REQUESTED: &[&str] = &[
    "Legal Services",
    "Mental Health Services",
    "Immigration Paperwork Assistance",
    "Economic Empowerment",
    "Supervised Visitation and Safe Exchange",
];

/// The signature and contact block captured with the Service Agreement. Every
/// field is a string because this is form input; [`Self::normalized`] is what
/// turns it into storable values.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientAgreementDetails {
    /// The recipient's full legal name, as printed on the agreement.
    pub legal_name: String,
    /// The full legal name typed into the electronic-signature field.
    pub signature_name: String,
    pub email: String,
    /// ISO `YYYY-MM-DD`, as an `<input type="date">` produces.
    pub date_of_birth: String,
    pub phone: String,
    pub emergency_name: String,
    pub emergency_relationship: String,
    pub emergency_phone: String,
    /// The ticked services, in the order of [`SERVICES_REQUESTED`].
    #[serde(default)]
    pub services: Vec<String>,
    /// Whether the electronic consent box was clicked.
    #[serde(default)]
    pub electronic_consent: bool,
    // The minor block. Every field here is optional: it is filled in only when a
    // parent or legal guardian is signing on a minor's behalf.
    #[serde(default)]
    pub minor_name: String,
    #[serde(default)]
    pub guardian_name: String,
    #[serde(default)]
    pub guardian_relationship: String,
    #[serde(default)]
    pub guardian_signature: String,
    #[serde(default)]
    pub guardian_email: String,
}

impl ClientAgreementDetails {
    /// Trim everything, format both phones, and drop services we do not offer.
    /// Always run before [`Self::validate`] and before storing.
    pub fn normalized(&self) -> Self {
        Self {
            legal_name: self.legal_name.trim().to_string(),
            signature_name: self.signature_name.trim().to_string(),
            email: self.email.trim().to_lowercase(),
            date_of_birth: self.date_of_birth.trim().to_string(),
            phone: normalize_phone(&self.phone),
            emergency_name: self.emergency_name.trim().to_string(),
            emergency_relationship: self.emergency_relationship.trim().to_string(),
            emergency_phone: normalize_phone(&self.emergency_phone),
            services: SERVICES_REQUESTED
                .iter()
                .filter(|service| self.services.iter().any(|chosen| chosen == *service))
                .map(|service| (*service).to_string())
                .collect(),
            electronic_consent: self.electronic_consent,
            minor_name: self.minor_name.trim().to_string(),
            guardian_name: self.guardian_name.trim().to_string(),
            guardian_relationship: self.guardian_relationship.trim().to_string(),
            guardian_signature: self.guardian_signature.trim().to_string(),
            guardian_email: self.guardian_email.trim().to_string(),
        }
    }

    /// Whether the block is complete and well-formed. The error is shown to the
    /// recipient as-is. The minor block is never required — someone signing for
    /// themselves leaves all of it blank.
    pub fn validate(&self) -> Result<(), String> {
        if !self.electronic_consent {
            return Err(
                "Click the acknowledgment box to adopt your typed name as your electronic signature."
                    .to_string(),
            );
        }
        if self.legal_name.trim().is_empty() {
            return Err("Please give the recipient's full legal name.".to_string());
        }
        if self.signature_name.trim().is_empty() {
            return Err("Type your full legal name as your electronic signature.".to_string());
        }
        if !is_email(&self.email) {
            return Err("Enter a valid email address.".to_string());
        }
        validate_date_of_birth(self.date_of_birth.trim())?;
        if format_phone(&self.phone).is_none() {
            return Err(
                "Enter your phone number as 10 digits, for example (555) 123-4567.".to_string(),
            );
        }
        if self.emergency_name.trim().is_empty() {
            return Err("Please give your emergency contact's name.".to_string());
        }
        if self.emergency_relationship.trim().is_empty() {
            return Err("Please give your emergency contact's relationship to you.".to_string());
        }
        if format_phone(&self.emergency_phone).is_none() {
            return Err(
                "Enter your emergency contact's phone number as 10 digits, for example (555) 123-4567."
                    .to_string(),
            );
        }
        if self.services.is_empty() {
            return Err("Please choose at least one service you are asking for.".to_string());
        }
        // Not required, but an address typed into it still has to be one.
        if !self.guardian_email.trim().is_empty() && !is_email(&self.guardian_email) {
            return Err("Enter a valid parent or legal guardian email address.".to_string());
        }
        Ok(())
    }

    /// The ticked services as one comma-separated string, for display and for
    /// the case property staff read them from.
    pub fn services_line(&self) -> String {
        self.services.join(", ")
    }
}

/// A crude but sufficient email check: one `@` with something either side and a
/// dot in the domain.
fn is_email(value: &str) -> bool {
    let value = value.trim();
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

/// A phone number as `(000) 000-0000`, or the trimmed input when it is not ten
/// digits, so validation can quote back what was typed.
fn normalize_phone(value: &str) -> String {
    format_phone(value).unwrap_or_else(|| value.trim().to_string())
}

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
