//! Sections: the free-text heading a case property or case file is grouped
//! under in the case view, such as "Intake".

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
