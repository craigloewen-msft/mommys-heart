//! Who may see a piece of case information.
//!
//! Both case properties and case files carry a visibility, which is why it
//! lives here rather than inside either of them.

use serde::{Deserialize, Serialize};

/// The audience a case property or case file belongs to.
///
/// A case holds two audiences' worth of information side by side:
///
/// * [`Visibility::Shared`] — the client-facing record. Everyone who can view
///   the case sees it, the client the case is about included.
/// * [`Visibility::VolunteerOnly`] — the team's working record: the intake and
///   outtake paperwork, and anything else volunteers and admins keep on a case
///   but never show the client.
///
/// This is deliberately the *same* distinction the case chat already draws with
/// [`ChannelKind`], down to the stored string, so the app has one word for
/// "staff only" instead of a synonym per feature.
///
/// It is an **account-role** gate ([`sees_volunteer_only`]), orthogonal to the
/// per-case [`CaseCapability`] gate: capabilities decide whether you may touch a
/// case's properties and files *at all*, visibility decides *which* of them you
/// see. No capability grants a client account access to a volunteer-only row.
///
/// [`ChannelKind`]: crate::server_fns::channels::ChannelKind
/// [`CaseCapability`]: crate::server_fns::capabilities::CaseCapability
/// [`sees_volunteer_only`]: crate::server::permissions::sees_volunteer_only
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Visible to everyone who can view the case, clients included.
    #[default]
    Shared,
    /// Visible only to the volunteers and admins working the case.
    VolunteerOnly,
}

impl Visibility {
    /// Every visibility, in display order: the client-facing record first,
    /// followed by the team's own working record for staff who can see it.
    pub const ALL: [Visibility; 2] = [Visibility::Shared, Visibility::VolunteerOnly];

    /// The stored representation. Matches [`ChannelKind::slug`] for the
    /// equivalent audience on purpose.
    ///
    /// [`ChannelKind::slug`]: crate::server_fns::channels::ChannelKind::slug
    pub fn slug(self) -> &'static str {
        match self {
            Visibility::Shared => "shared",
            Visibility::VolunteerOnly => "volunteer_only",
        }
    }

    /// Heading used in the case view.
    pub fn label(self) -> &'static str {
        match self {
            Visibility::Shared => "Shared with client",
            Visibility::VolunteerOnly => "Volunteers and admins only",
        }
    }

    /// The name of the standing top-level file folder for this audience. Every
    /// case has exactly one folder per audience at the top of its file tree.
    pub fn folder_name(self) -> &'static str {
        match self {
            Visibility::Shared => "Shared with client",
            Visibility::VolunteerOnly => "Volunteer only",
        }
    }

    /// One-line explanation of who can see this, shown under the heading.
    pub fn description(self) -> &'static str {
        match self {
            Visibility::Shared => "Everyone with access to this case can see this info.",
            Visibility::VolunteerOnly => "Never shown to the client this case is about.",
        }
    }

    /// Parse a stored value. Unknown input is rejected rather than defaulted, so
    /// a typo can never silently widen an audience.
    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.slug() == s)
    }

    /// Whether this is hidden from client accounts.
    pub fn is_restricted(self) -> bool {
        matches!(self, Visibility::VolunteerOnly)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            Visibility::Shared => "bg-slate-500/15 text-slate-300 ring-1 ring-slate-500/30",
            Visibility::VolunteerOnly => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
        }
    }
}
