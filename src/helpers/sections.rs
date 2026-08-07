//! Sections: the free-text heading a case property is grouped under in the case
//! view, such as "Intake".

pub const INTAKE: &str = "Intake";
pub const OUTTAKE: &str = "Outtake";

/// The heading used for rows whose section is empty.
pub const DEFAULT_LABEL: &str = "General";

/// The display label for a (possibly empty) section name.
pub fn label(section: &str) -> &str {
    if section.trim().is_empty() {
        DEFAULT_LABEL
    } else {
        section
    }
}
