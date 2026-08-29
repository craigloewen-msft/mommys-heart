//! Date primitives shared by the forms that take a date: parsing the ISO
//! `YYYY-MM-DD` an `<input type="date">` produces, rendering it the way the
//! Foundation's paper forms do, and reading today's date.
//!
//! Pure functions with no feature-specific meaning, so both the volunteer
//! details form and the case intake questionnaire use them rather than one
//! importing the other's private helpers.

/// Split an ISO `YYYY-MM-DD` date, checking the parts are in range. The day is
/// not checked against the month's real length; the date input cannot produce one.
pub fn parse_iso(value: &str) -> Option<(i32, u32, u32)> {
    let mut parts = value.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

/// An ISO `YYYY-MM-DD` date as `MM-DD-YYYY`, the format the Foundation's forms
/// use. Anything unparseable is passed through.
pub fn to_us(value: &str) -> String {
    match parse_iso(value.trim()) {
        Some((year, month, day)) => format!("{month:02}-{day:02}-{year:04}"),
        None => value.trim().to_string(),
    }
}

/// Today as ISO `YYYY-MM-DD`. [`crate::state::today`] is a browser clock that
/// stubs to 1970 without `hydrate`, so the server reads its own.
pub fn today() -> String {
    #[cfg(feature = "ssr")]
    {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }
    #[cfg(not(feature = "ssr"))]
    {
        crate::state::today()
    }
}
