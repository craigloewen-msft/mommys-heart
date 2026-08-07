//! The Terms and Conditions a prospective client accepts before they can start a
//! case, and the version string that records *which* wording they accepted.
//!
//! Like [`new_case_folders`](crate::helpers::new_case_folders), this text lives
//! in code on purpose: it applies org-wide, changes by deploy rather than at
//! runtime, and there is exactly one live version at a time. Revising the terms
//! is a two-part edit here — change the prose *and* bump [`TERMS_VERSION`] — so
//! that acceptances already recorded keep pointing at the wording they were given
//! rather than silently inheriting the new one.

/// The version of the terms currently in force. Stored verbatim on every
/// acceptance; bump it whenever [`TERMS_SECTIONS`] changes in substance.
pub const TERMS_VERSION: &str = "2026-01-01";

/// One numbered section of the terms: an optional heading and its paragraphs.
pub struct TermsSection {
    /// Shown above the paragraphs. Empty for the opening preamble, which reads
    /// as continuous prose rather than a titled clause.
    pub heading: &'static str,
    pub paragraphs: &'static [&'static str],
}

/// The terms, in display order.
pub const TERMS_SECTIONS: &[TermsSection] = &[
    TermsSection {
        heading: "",
        paragraphs: &[
            "Thank you for engaging Mommy's Heart, Inc. (the \"Foundation\") to assist you with issues relating to your divorce proceedings and associated parental custody matters. We look forward to working with you in furthering the Foundation's mission of protecting and defending the fundamental rights of fit parents to direct the care and custody of their children.",
            "These Terms and Conditions (the \"Agreement\") are entered into between Mommy's Heart, Inc., with the mailing address of 930 Fifth Avenue, Suite 4H, New York, NY 10021, and you (\"Recipient of Services\", \"Recipient\", or \"you\"; together with the Foundation, the \"Parties\"), and take effect on the date you accept them below.",
            "In working with Mommy's Heart, you understand and agree as follows:",
        ],
    },
    TermsSection {
        heading: "1. The Foundation connects you with Volunteers",
        paragraphs: &[
            "The Foundation is a nonprofit organization that seeks to assist parents seeking to direct the care and custody of their children by connecting them with professionals (collectively, the \"Volunteers\") who can provide assistance. While the Volunteers are associated with the Foundation, you will have a direct relationship with the Volunteers, and the Foundation is not responsible for any advice the Volunteers provide or work they perform on your behalf. Any questions regarding those services should be directed to the Volunteers.",
        ],
    },
    TermsSection {
        heading: "2. No guaranteed outcomes",
        paragraphs: &[
            "I acknowledge I am entering into this agreement and have elected to support the Foundation's mission on a voluntary basis. The Foundation cannot guarantee any legal, health related, or other outcomes and itself is not a law firm or other organization that provides legal, mental wellbeing, or other advice directly to Recipients of Services.",
        ],
    },
    TermsSection {
        heading: "3. Release and indemnification",
        paragraphs: &[
            "The Foundation (including its employees, management, board members, and other volunteers) is not responsible for the actions of its Volunteers providing legal, mental wellbeing, or other services or any damages that may arise therefrom, and you hereby release and indemnify the Foundation from any third party claims arising from the services the Foundation or its Volunteers provide. To the fullest extent permitted by law you assume all risk of injury or harm resulting from the services the Foundation provides and agree to release, indemnify, and forever discharge the Foundation from any and all potential liability as a consequence of those services; provided that you shall not be required to indemnify the Foundation for any harm, loss, or claim to the extent such harm, loss, or claim is due to the gross negligence or willful misconduct of the Foundation or its employees.",
        ],
    },
    TermsSection {
        heading: "4. Confidentiality and the information you share",
        paragraphs: &[
            "The Foundation agrees to maintain the information you provide in the course of seeking its services in strict confidence. Should a third party seek such information in the course of court proceedings or for other reasons, the Foundation will give you notice of such requests where permitted by law. The Foundation will not sell your information to third parties.",
            "You agree that, unless requested by and/or consented to by the Foundation, you will not provide the Foundation with unsolicited personally identifiable or protected health information beyond what is required for the Foundation to perform its services. You further agree not to provide the Foundation any information subject to the attorney-client privilege, physician-patient privilege, or therapist-patient privilege.",
        ],
    },
    TermsSection {
        heading: "5. Non-disparagement",
        paragraphs: &[
            "Recipient of Services will not disparage the Foundation or its officers, directors, employees, grantors, donors, clients, affiliates or agents, in any manner likely to be harmful to them or their business, business reputation or personal reputation; provided, however, you shall respond accurately and fully to any question, inquiry or request for information when required by legal process.",
        ],
    },
    TermsSection {
        heading: "6. Arbitration and governing law",
        paragraphs: &[
            "Any controversy or claim arising out of or relating to this contract, or the breach thereof, shall be settled by arbitration administered by the American Arbitration Association under its Commercial Arbitration Rules, and judgment on the award rendered by the arbitrator(s) may be entered in any court having jurisdiction thereof. The Parties agree that this Agreement shall be governed by New York State law, with venue for any dispute to be mandatory in the borough of Manhattan in New York County, New York.",
        ],
    },
    TermsSection {
        heading: "More information",
        paragraphs: &[
            "For more information regarding the Foundation's Terms of Service and Privacy Policy, please visit www.mommysheartfoundation.com.",
        ],
    },
];

/// The sentence shown immediately above the acceptance control. Kept separate
/// from [`TERMS_SECTIONS`] because it is what the checkbox attests to, not part
/// of the terms it attests about.
pub const TERMS_ATTESTATION: &str = "Recipient of Services acknowledges, affirms, and certifies that they have read and reviewed, and now and hereby agree to, all of the above-stated paragraphs. By accepting below, Recipient of Services certifies that they understand the terms and conditions set forth above are a binding contract with the Foundation.";

/// Shown beneath the acceptance control: minors cannot bind themselves, and the
/// signed-paperwork version of this agreement said so explicitly.
pub const TERMS_MINOR_NOTICE: &str = "If the Recipient of Services is under the age of 18, a parent or legal guardian must accept these terms on their behalf.";

/// Whether `version` is a terms version this build knows how to honour. Only the
/// current wording may be accepted — an older tab holding a stale version must
/// re-read the terms rather than consent to text it was never shown.
pub fn is_current(version: &str) -> bool {
    version == TERMS_VERSION
}
