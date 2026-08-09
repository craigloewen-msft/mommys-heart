//! The Volunteer Agreement a prospective volunteer accepts before they can apply,
//! and the version string that records *which* wording they accepted.
//!
//! # PLACEHOLDER WORDING
//!
//! The text below is **placeholder prose written by a developer, not reviewed
//! legal language**. It describes the volunteer relationship as this application
//! models it, so the flow can be built and tested end to end, but it must be
//! replaced with wording the Foundation has actually approved before any real
//! volunteer is asked to accept it. When it is replaced, bump
//! [`VOLUNTEER_AGREEMENT_VERSION`] in the same edit.
//!
//! Structurally this mirrors [`crate::helpers::terms`]: the text lives in code
//! because it applies org-wide and changes by deploy, and acceptances store the
//! version string rather than a reference so revising the wording never changes
//! what a past volunteer agreed to.

use crate::helpers::terms::TermsSection;

/// The version of the volunteer agreement currently in force. Stored verbatim on
/// every acceptance; bump it whenever [`VOLUNTEER_AGREEMENT_SECTIONS`] changes in
/// substance.
pub const VOLUNTEER_AGREEMENT_VERSION: &str = "placeholder-2026-01-01";

/// The agreement, in display order.
pub const VOLUNTEER_AGREEMENT_SECTIONS: &[TermsSection] = &[
    TermsSection {
        heading: "",
        paragraphs: &[
            "Thank you for offering your time to Mommy's Heart, Inc. (the \"Foundation\"). Volunteers are how the Foundation carries out its mission of protecting and defending the fundamental rights of fit parents to direct the care and custody of their children.",
            "This Volunteer Agreement (the \"Agreement\") is entered into between the Foundation and you (\"Volunteer\" or \"you\"), and takes effect on the date you accept it below. In volunteering with the Foundation, you understand and agree as follows:",
        ],
    },
    TermsSection {
        heading: "1. Volunteer service is unpaid and is not employment",
        paragraphs: &[
            "You are offering your services voluntarily and without expectation of payment, benefits, or other compensation. This Agreement does not create an employment relationship, a partnership, or an agency relationship between you and the Foundation, and nothing in it should be read as a promise of work, hours, or continued involvement.",
        ],
    },
    TermsSection {
        heading: "2. Confidentiality",
        paragraphs: &[
            "In the course of volunteering you will learn confidential and sensitive information about Recipients of Services, including information about their families, their children, their finances, and ongoing legal proceedings. You agree to keep all such information strictly confidential, to access it only as needed to perform your volunteer work, and to not disclose it to anyone outside the Foundation without the express permission of the Foundation or the Recipient of Services.",
            "This obligation continues after you stop volunteering with the Foundation.",
        ],
    },
    TermsSection {
        heading: "3. You are not providing professional services on the Foundation's behalf",
        paragraphs: &[
            "The Foundation is not a law firm and does not provide legal, medical, or mental-health advice. If you are a licensed professional, any professional services you choose to provide are provided by you directly and in your own professional capacity, subject to your own professional obligations and judgment, and not on behalf of the Foundation. The Foundation does not supervise, direct, or take responsibility for that work.",
        ],
    },
    TermsSection {
        heading: "4. Conduct",
        paragraphs: &[
            "You agree to treat Recipients of Services, staff, and other volunteers with respect, to follow the Foundation's policies and any reasonable direction given to you in the course of your volunteer work, and to promptly disclose any conflict of interest that arises between your volunteer role and your other commitments.",
        ],
    },
    TermsSection {
        heading: "5. Either party may end the arrangement at any time",
        paragraphs: &[
            "You may stop volunteering at any time and for any reason, and the Foundation may end your volunteer role at any time and for any reason. Your confidentiality obligations under Section 2 survive that ending.",
        ],
    },
    TermsSection {
        heading: "6. Approval",
        paragraphs: &[
            "Accepting this Agreement submits an application to volunteer. It does not by itself make you a volunteer: an administrator reviews each application, and you will be notified by email whether it was approved or declined.",
        ],
    },
];

/// The sentence shown immediately above the acceptance control. Kept separate
/// from [`VOLUNTEER_AGREEMENT_SECTIONS`] because it is what the checkbox attests
/// to, not part of the agreement it attests about.
pub const VOLUNTEER_ATTESTATION: &str = "You acknowledge and certify that you have read and reviewed, and now and hereby agree to, all of the above-stated paragraphs, and that you are submitting an application to volunteer with the Foundation.";

/// Whether `version` is an agreement version this build knows how to honour. Only
/// the current wording may be accepted — an older tab holding a stale version
/// must re-read the agreement rather than consent to text it was never shown.
pub fn is_current(version: &str) -> bool {
    version == VOLUNTEER_AGREEMENT_VERSION
}
