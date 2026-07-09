//! Pure aggregation layer for the analytics / outcome-tracking dashboards.
//!
//! Everything here is a plain function over the in-memory demo data
//! (`Vec<Case>` / `Vec<Volunteer>`), so it compiles for both the server and the
//! browser and has no dependency on reactive state. The `/analytics` page feeds
//! it the current `AppState` snapshots and renders the results.

use std::collections::BTreeMap;

use crate::types::{Case, CaseOutcome, CaseStatus, MatterType, Volunteer};

/// A single labelled measurement used to drive bar/donut/list charts.
#[derive(Clone, Debug, PartialEq)]
pub struct MetricSlice {
    pub label: String,
    pub value: f64,
    /// Optional Tailwind colour classes for donut/legend rendering.
    pub color: Option<&'static str>,
}

impl MetricSlice {
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
        }
    }

    pub fn with_color(mut self, color: &'static str) -> Self {
        self.color = Some(color);
        self
    }
}

// ---------------------------------------------------------------------------
// Date helpers (ISO `YYYY-MM-DD`, no external date crate).
// ---------------------------------------------------------------------------

/// Parse an ISO `YYYY-MM-DD` string into (year, month, day). Returns `None` for
/// non-date placeholders like "just now".
fn parse_ymd(s: &str) -> Option<(i64, i64, i64)> {
    let mut parts = s.split('-');
    let y = parts.next()?.trim().parse().ok()?;
    let m = parts.next()?.trim().parse().ok()?;
    let d = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((y, m, d))
}

/// Approximate day count for a date since year 0, good enough to diff two dates
/// (uses a 30.44-day month average — fine for reporting-scale day differences).
fn approx_days((y, m, d): (i64, i64, i64)) -> f64 {
    y as f64 * 365.25 + (m as f64 - 1.0) * 30.44 + d as f64
}

/// Whole-day difference between two ISO dates, or `None` if either is unparsable.
fn days_between(start: &str, end: &str) -> Option<f64> {
    let a = approx_days(parse_ymd(start)?);
    let b = approx_days(parse_ymd(end)?);
    Some((b - a).max(0.0))
}

/// `YYYY-MM` key for grouping by month, from an ISO date.
fn year_month(s: &str) -> Option<String> {
    let (y, m, _) = parse_ymd(s)?;
    Some(format!("{y:04}-{m:02}"))
}

// ---------------------------------------------------------------------------
// Program overview / client & service metrics.
// ---------------------------------------------------------------------------

/// Headline client & service metrics for the program-overview cards.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProgramOverview {
    pub clients_served: usize,
    pub open_matters: usize,
    pub closed_matters: usize,
    pub referrals_made: usize,
    pub services_provided: usize,
    /// 0.0–100.0 completion rate of follow-ups.
    pub follow_up_completion_rate: f64,
    /// Average days from intake to resolution across resolved cases.
    pub avg_days_to_resolution: Option<f64>,
}

pub fn program_overview(cases: &[Case]) -> ProgramOverview {
    let clients_served = cases.len();
    let closed_matters = cases
        .iter()
        .filter(|c| c.status == CaseStatus::Closed)
        .count();
    let open_matters = clients_served - closed_matters;

    let referrals_made = cases.iter().map(|c| c.referrals.len()).sum();
    let services_provided = cases.iter().map(|c| c.services.len()).sum();

    let (fu_total, fu_done) = cases.iter().fold((0usize, 0usize), |(t, d), c| {
        (
            t + c.follow_ups.len(),
            d + c.follow_ups.iter().filter(|f| f.completed).count(),
        )
    });
    let follow_up_completion_rate = if fu_total == 0 {
        0.0
    } else {
        fu_done as f64 / fu_total as f64 * 100.0
    };

    let durations: Vec<f64> = cases
        .iter()
        .filter_map(|c| {
            let end = c.resolved_date.as_deref()?;
            days_between(&c.intake_date, end)
        })
        .collect();
    let avg_days_to_resolution = if durations.is_empty() {
        None
    } else {
        Some(durations.iter().sum::<f64>() / durations.len() as f64)
    };

    ProgramOverview {
        clients_served,
        open_matters,
        closed_matters,
        referrals_made,
        services_provided,
        follow_up_completion_rate,
        avg_days_to_resolution,
    }
}

// ---------------------------------------------------------------------------
// Programmatic (matter-type) metrics.
// ---------------------------------------------------------------------------

/// Case counts by matter type, ordered by the canonical `MatterType::ALL` order,
/// dropping categories with no cases.
pub fn matters_by_type(cases: &[Case]) -> Vec<MetricSlice> {
    MatterType::ALL
        .into_iter()
        .filter_map(|mt| {
            let count = cases.iter().filter(|c| c.matter_type == mt).count();
            (count > 0).then(|| MetricSlice::new(mt.label(), count as f64))
        })
        .collect()
}

/// Case counts by lifecycle status.
pub fn cases_by_status(cases: &[Case]) -> Vec<MetricSlice> {
    CaseStatus::ALL
        .into_iter()
        .map(|st| {
            let count = cases.iter().filter(|c| c.status == st).count();
            MetricSlice::new(st.label(), count as f64).with_color(status_color(st))
        })
        .collect()
}

/// Case counts by resolution outcome.
pub fn cases_by_outcome(cases: &[Case]) -> Vec<MetricSlice> {
    CaseOutcome::ALL
        .into_iter()
        .map(|oc| {
            let count = cases.iter().filter(|c| c.outcome == oc).count();
            MetricSlice::new(oc.label(), count as f64).with_color(outcome_color(oc))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Trends over time.
// ---------------------------------------------------------------------------

/// Matters opened per month, chronologically. Non-date placeholders (e.g. cases
/// created live in the demo with "just now") are ignored.
pub fn matters_opened_by_month(cases: &[Case]) -> Vec<MetricSlice> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for c in cases {
        if let Some(ym) = year_month(&c.opened_at) {
            *counts.entry(ym).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .map(|(ym, n)| MetricSlice::new(ym, n as f64))
        .collect()
}

// ---------------------------------------------------------------------------
// Volunteer metrics.
// ---------------------------------------------------------------------------

/// Per-volunteer operational figures for the volunteer metrics table/charts.
#[derive(Clone, Debug, PartialEq)]
pub struct VolunteerMetric {
    pub name: String,
    pub hours_logged: f64,
    pub open_case_load: usize,
    pub total_case_load: usize,
    pub client_contacts: u32,
    pub weekly_availability_hours: f64,
    pub trainings_completed: usize,
}

/// Compute per-volunteer metrics, joining volunteers to their assigned cases.
pub fn volunteer_metrics(volunteers: &[Volunteer], cases: &[Case]) -> Vec<VolunteerMetric> {
    volunteers
        .iter()
        .map(|v| {
            let assigned: Vec<&Case> = cases
                .iter()
                .filter(|c| c.assigned_volunteer_ids.iter().any(|id| id == &v.id))
                .collect();
            let open_case_load = assigned
                .iter()
                .filter(|c| c.status != CaseStatus::Closed)
                .count();
            VolunteerMetric {
                name: v.name.clone(),
                hours_logged: v.hours_logged,
                open_case_load,
                total_case_load: assigned.len(),
                client_contacts: v.client_contacts,
                weekly_availability_hours: v.weekly_availability_hours,
                trainings_completed: v.trainings.iter().filter(|t| t.is_completed()).count(),
            }
        })
        .collect()
}

/// Volunteer hours as chart slices (one bar per volunteer).
pub fn volunteer_hours(volunteers: &[Volunteer]) -> Vec<MetricSlice> {
    volunteers
        .iter()
        .filter(|v| v.hours_logged > 0.0)
        .map(|v| MetricSlice::new(v.name.clone(), v.hours_logged))
        .collect()
}

/// Case workload distribution: open cases assigned per volunteer.
pub fn workload_distribution(volunteers: &[Volunteer], cases: &[Case]) -> Vec<MetricSlice> {
    volunteer_metrics(volunteers, cases)
        .into_iter()
        .map(|m| MetricSlice::new(m.name, m.open_case_load as f64))
        .collect()
}

/// Training-participation rate: share of volunteers with ≥1 completed training.
pub fn training_participation_rate(volunteers: &[Volunteer]) -> f64 {
    if volunteers.is_empty() {
        return 0.0;
    }
    let participated = volunteers
        .iter()
        .filter(|v| v.trainings.iter().any(|t| t.is_completed()))
        .count();
    participated as f64 / volunteers.len() as f64 * 100.0
}

// ---------------------------------------------------------------------------
// Colour helpers (Tailwind classes shared by donut segments + legends).
// ---------------------------------------------------------------------------

fn status_color(status: CaseStatus) -> &'static str {
    match status {
        CaseStatus::Open => "text-sky-400",
        CaseStatus::InProgress => "text-violet-400",
        CaseStatus::OnHold => "text-amber-400",
        CaseStatus::Closed => "text-slate-400",
    }
}

fn outcome_color(outcome: CaseOutcome) -> &'static str {
    match outcome {
        CaseOutcome::Resolved => "text-emerald-400",
        CaseOutcome::Ongoing => "text-sky-400",
        CaseOutcome::ReferredOut => "text-violet-400",
        CaseOutcome::Withdrawn => "text-slate-400",
        CaseOutcome::Unresolved => "text-rose-400",
    }
}
