//! The Volunteer Agreement and the version string recording which wording was
//! accepted. Mirrors [`crate::helpers::terms`].
//!
//! The wording is the Foundation's approved text, transcribed verbatim. Revising
//! it means changing the prose *and* bumping [`VOLUNTEER_AGREEMENT_VERSION`].

use crate::helpers::terms::TermsSection;

/// The version of the volunteer agreement currently in force. Stored verbatim on
/// every acceptance; bump it whenever [`VOLUNTEER_AGREEMENT_SECTIONS`] changes in
/// substance.
pub const VOLUNTEER_AGREEMENT_VERSION: &str = "2026-08-10.2";

/// The agreement, in display order.
pub const VOLUNTEER_AGREEMENT_SECTIONS: &[TermsSection] = &[
    TermsSection {
        heading: "",
        paragraphs: &[
            "WHEREAS, the Volunteer desires an opportunity to help those in need and gain valuable knowledge, experience, education, and training in the Foundation\u{2019}s causes and undertakings for legal and social reform;",
            "WHEREAS, the Foundation offers the Volunteer this opportunity;",
            "NOW, therefore, the Foundation and Volunteer, in consideration of the mutual promises, conditions and covenants contained herein, hereby agree as follows:",
        ],
    },
    TermsSection {
        heading: "1. The Volunteer Position, Duties, and Responsibilities",
        paragraphs: &[
            "The Volunteer shall work without financial compensation in one or more of the Foundation\u{2019}s (1) operations, (2) marketing and public relations, (3) fundraising, (4) legal, and/or (5) mental health services department.",
            "The Volunteer\u{2019}s duties will be determined by Julianne Michelle Reeves Stroh, the Foundation\u{2019}s President, or an officially designated officer/employee nominated and confirmed as agent by the Foundation\u{2019}s President. The Foundation expects all volunteers to engage in and to perform, as they may be directed to do, in one or more of the following duties: (1) to assist in marketing and promotional efforts, (2) research, (3) outreach, (4) fundraising, (5) financial reporting and analysis, (6) product design and/or (7) service delivery, and/or finally (8) in developing strategies of advocacy, (by conversation and investigation, both on-line and \u{201c}in library\u{201d}) and writing to advance the mission of the foundation.",
            "The Volunteer may be responsible for the following duties and/or tasks, including but not limited to: reading, writing, sponsorship outreach, content creation, generating financial statements, research, search engine optimization, online ads and marketing, fundraising, strategy, press releases, intakes, case management and organization, responding to press inquiries, and event planning and promotion. (\u{201c}Duties and Responsibilities\u{201d}).",
        ],
    },
    TermsSection {
        heading: "2. Work Schedule",
        paragraphs: &[
            "The Volunteer is not required to work particular days or hours; the Volunteer will determine the days and hours, during the Foundation\u{2019}s business hours, during which the Volunteer will be working. However, the Foundation expects the Volunteer to establish and maintain a relatively consistent work schedule, informing his or her supervisor(s) of the same for day-to-day project planning, such that the Foundation can expect the Volunteer to be available to work on approximately the same days and at approximately the same times each week. Volunteer will apprise the Foundation\u{2019}s president of his or her expected work schedule each week.",
        ],
    },
    TermsSection {
        heading: "3.",
        paragraphs: &[
            "The Foundation takes no responsibility for, denies and will accept no liability for any assignments or work the volunteer accepts or documents the Volunteer creates without the Foundation\u{2019}s express knowledge or consent.",
            "Compensation. The Parties agree this is an unpaid position in that the volunteer will neither be directly nor indirectly financially compensated for the duties performed at the Foundation.  The volunteer will at all times be responsible for maintaining his or her own insurance, as the Foundation does not and will not provide coverage of any kind.  The Volunteer agrees that the opportunity to help others and garner practical experience and insight under supervision by and with the assistance of attorneys, legal and business consultants, mental health practitioners, and other experts will constitute good and sufficient compensation for his/her services.",
        ],
    },
    TermsSection {
        heading: "4.",
        paragraphs: &[
            "The Volunteer accepts the opportunity to work for the Foundation as reasonable compensation and consideration for his or her time, and agrees to the Volunteer helping others and gaining valuable knowledge, experience, education, and training in the Foundation's industry as consideration for the Duties and Responsibilities. The Volunteer\u{2019}s work for the Foundation may also entitle the Volunteer to credit from the college or university she or he is attending.",
        ],
    },
    TermsSection {
        heading: "5. Employment Status",
        paragraphs: &[
            "The Volunteer is not an employee of the Foundation. Since there is no financial compensation, the Foundation will not withhold any federal, state or city income or any other taxes on behalf of the Volunteer. The Volunteer will not be regarded as an employee or servant of the Foundation for the purpose of any federal or state employment, labor, unemployment, tax, or other law or regulation. The Volunteer again acknowledges: the Foundation will not obtain disability, worker\u{2019}s compensation, or any other type of insurance on the Volunteer\u{2019}s behalf or for the benefit of Volunteer. The Foundation will not provide the Volunteer with any of the benefits which may or may not be provided by the Foundation to its employees, such as group medical coverage and paid vacation time. No Volunteer will receive any perquisites of any kind from the Foundation except for acknowledgment and confirmation of his or her services.  Lastly, the Volunteer will not act as an agent or officer of the Foundation and does not have the authority to bind the Foundation in any manner whatsoever.",
        ],
    },
    TermsSection {
        heading: "6. Work for Hire",
        paragraphs: &[
            "All images, videos, text, writings, working papers, manuals, computer programs, source codes, graphics, plans, artwork, copyrights or trademarks, documents and other works (\u{201c}Works\u{201d}) created wholly or in part by the Volunteer, as a result of the Volunteer\u{2019}s working for the Foundation, shall conclusively be deemed to be \u{201c}works made for hire\u{201d} within the meaning of 17 U.S.C. \u{a7} 201(b), even though Volunteer is not an employee of the Foundation. All copyright and other intellectual property rights in and to such Works belong to the Foundation, not to the Volunteer. The Volunteer hereby assigns to the Foundation all intellectual property rights in all Works created wholly or in part by Volunteer pursuant to this Agreement.",
        ],
    },
    TermsSection {
        heading: "7. Term",
        paragraphs: &[
            "This Agreement shall commence upon the Effective Date of this Agreement and will continue for one year.",
        ],
    },
    TermsSection {
        heading: "8. Unbecoming Conduct",
        paragraphs: &[
            "You agree to refrain from making any statements or engaging in any actions that might be offensive to the Foundation\u{2019}s employees or volunteers and/or that might be construed by another person as harassment based in part or in whole on such person\u{2019}s sex, race, national origin, age, sexual orientation, disability, religion, or political affiliation.  The Volunteer shall refrain from making statements on behalf of the Foundation, on social media or any other form of communication, unless expressly authorized to do so.  The Volunteer shall refrain from any use of alcohol and recreational drugs during the working day and at all times while at the office, in court, or anywhere else in public or private meetings with other Foundation personnel.  Additionally, the Volunteer agrees not to disparage the Foundation or its officers, directors, employees, grantors, donors, clients, affiliates or agents, in any manner likely to be harmful to them or their business, business reputation or personal reputation; provided, however, that Volunteer shall respond accurately and fully to any question, inquiry or request for information when required by legal process.",
        ],
    },
    TermsSection {
        heading: "NONDISCLOSURE AGREEMENT, COMMITMENT, & LIABILITY",
        paragraphs: &[],
    },
    TermsSection {
        heading: "9. Maintaining Confidentiality of Information",
        paragraphs: &[
            "In working for the Foundation, Volunteer will have access to and will obtain non-public information concerning the Foundation\u{2019}s business, board members, actual and prospective clients, actual and prospective sponsors or donors, suppliers, finances, marketing and business strategies and plans, contacts, products or services, passwords or login credentials, marketing and promotion concepts and ideas (whether or not copyrightable), and other matters (collectively, \u{201c}Information\u{201d}). Volunteer agrees that all such Information is confidential and proprietary information which derives independent economic value, actually or potentially, from not being generally known to the public, to competitors, and to other persons that can or might obtain economic value from the disclosure or use of it.  The Volunteer at all times accepts full responsibility and liability for his or her breach of this agreement, and commits to exercising the maximum caution in the handling of proprietary information without exception or exclusion.",
        ],
    },
    TermsSection {
        heading: "10.",
        paragraphs: &[
            "Volunteer will maintain all such Information on a confidential basis while working for the Foundation and at all times thereafter, without limitation, unless expressly ordered to disclose any such information by a Administrative Agency or Court of Competent Jurisdiction in proceedings of which the Volunteer guarantees that, by his or her own effects and responsibility, he or she shall provide the Foundation of any such court or administrative order at least 7 business days advance, legally proper, notification.",
        ],
    },
    TermsSection {
        heading: "11. Termination",
        paragraphs: &[
            "This Agreement may be terminated at follows:",
            "a. At any time by either Party upon written notice to the other Party.",
            "b. By the Foundation due to the Volunteer\u{2019}s breach of the Agreement.",
            "Upon termination, the Volunteer shall return all the Foundation content, materials, and all work product to the Foundation on the same day of termination, unless barred by supervening order or \u{201c}act of God.\u{201d}  Volunteer accepts full responsibility and liability for immediately returning all material in all media, including but not limited to all papers, electronically stored data or programs, and memoranda belonging to the Foundation which might have been used or created by the labor of the volunteer, but in no event beyond thirty (30) days after the date of termination. Failure to return documents and electronic programs or media within thirty (30) days may result in legal action to recover the same.",
        ],
    },
    TermsSection {
        heading: "12. Representations and Warranties",
        paragraphs: &[
            "The performance and obligations of either Party will not violate or infringe upon the rights of any third-party or violate any other agreement between the Parties, individually, and any other person, organization, or business or any law or governmental regulation.",
        ],
    },
    TermsSection {
        heading: "13.",
        paragraphs: &[
            "The Volunteer further represents that the Volunteer is duly authorized to work wherever assigned in the United States and is of legal age to accept and perform work (including express permission of parents or guardian, if working as a minor).",
        ],
    },
    TermsSection {
        heading: "ACCEPTANCE OF RISK with RELEASE AND INDEMNITY",
        paragraphs: &[],
    },
    TermsSection {
        heading: "14. ASSUMPTION OF RISK AND INDEMNIFICATION",
        paragraphs: &[
            "THE VOLUNTEER RELEASES AND FOREVER DISCHARGES AND AGREES TO INDEMNIFY THE FOUNDATION, ITS OFFICERS, AGENTS, AND AFFILIATES FROM ANY AND ALL LIABILITY FOR ANY INJURY, DAMAGE, LOSS, COST, OR EXPENSE (INCLUDING, WITHOUT LIMITATION, ATTORNEY\u{2019}S FEES) THAT THE VOLUNTEER MAY AT ANY TIME HAVE OR INCUR, ARISING OUT OF OR IN ANY MANNER RELATED TO ANY LOSS, DAMAGE, OR INJURY, INCLUDING BUT NOT LIMITED TO EMOTIONAL INJURY, SUFFERING, BODILY INJURY, LOSS OF PROPERTY, OR DEATH, THAT MAY BE SUSTAINED BY THE VOLUNTEER OR BY ANY PROPERTY BELONGING TO THE VOLUNTEER, WHILE WORKING WITH THE FOUNDATION.  THE VOLUNTEER ALSO AGREES TO INDEMNIFY THE FOUNDATION, ITS OFFICERS, AGENTS, AND AFFILIATES FOR ANY DAMAGES, CLAIMS, LOSSES, COSTS, OBLIGATIONS, LIABILITIES, AND EXPENSES, INCLUDING BUT NOT LIMITED TO ATTORNEY\u{2019}S FEES, DIRECTLY OR INDIRECTLY SUFFERED OR INCURRED BY THE FOUNDATION AND/OR ANY OF ITS AFFILIATES AS A RESULT OF ANY DIRECT OR INDIRECT ACT OR OMISSION OF THE VOLUNTEER.",
        ],
    },
    TermsSection {
        heading: "15.",
        paragraphs: &[
            "Sections 8, 9, 10, and 14 remain in full force and effect even after termination of the Agreement by its natural termination or early termination by either Party.",
        ],
    },
    TermsSection {
        heading: "16. Severability",
        paragraphs: &[
            "In the event any provision of this Agreement is deemed invalid or unenforceable, in whole or in part, that part shall be severed from the remainder of the Agreement and all other provisions shall continue in full force and effect as valid and enforceable.",
        ],
    },
    TermsSection {
        heading: "17. Waiver",
        paragraphs: &[
            "The failure by either Party to exercise any right, power, or privilege under the terms of this Agreement will not be construed as a waiver of any subsequent or further exercise of that right, power, or privilege or the exercise of any other right, power, or privilege.",
        ],
    },
    TermsSection {
        heading: "18. Dispute",
        paragraphs: &[
            "Any controversy or claim arising out of or relating to this contract, or the breach thereof, shall be settled by arbitration administered by the American Arbitration Association under its Commercial Arbitration Rules, and judgment on the award rendered by the arbitrator(s) may be entered in any court having jurisdiction thereof.  The Parties agree that this Agreement shall be governed by New York State law, with venue for any dispute to be mandatory in the borough of Manhattan in New York County, New York.",
        ],
    },
    TermsSection {
        heading: "19. Legal and Binding Agreement",
        paragraphs: &[
            "This Agreement is legal and binding between the Parties as stated above. This Agreement may be entered into and is legal and binding both in the United States and throughout Europe. The Parties each represent that they have the authority to enter into this Agreement.",
        ],
    },
    TermsSection {
        heading: "20. Entire Agreement",
        paragraphs: &[
            "This Agreement constitutes the entire agreement and understanding between the Volunteer and the Foundation. It supersedes any and all previous discussions, negotiations, understandings, and agreements between them.",
            "The Agreement between the parties is entirely written, and does not include any oral promises, oral representations, or other oral statements. This Agreement may not be modified except in a document signed by both parties.",
        ],
    },
];

/// The sentence shown immediately above the acceptance control. Kept separate
/// from [`VOLUNTEER_AGREEMENT_SECTIONS`] because it is what the checkbox attests
/// to, not part of the agreement it attests about.
///
/// The signature this attests to is the typed full legal name captured below it,
/// together with the Foundation President's countersignature.
pub const VOLUNTEER_ATTESTATION: &str = "Volunteer acknowledges, affirms, and certifies that they have read and reviewed, and now and hereby agree to all above-stated 20 paragraphs.  By affixing their signature, together with the Foundation President, Volunteer certifies that they understand the terms and conditions set forth above are a binding contract with the Foundation.";

/// The heading above the electronic consent provision.
pub const ELECTRONIC_CONSENT_HEADING: &str = "ELECTRONIC CONSENT AND SIGNATURE";

/// The electronic consent provision, which the tick box below it adopts.
pub const ELECTRONIC_CONSENT: &str = "By checking the acknowledgment box below and typing my full legal name in the electronic-signature field, I represent and certify that I am the Volunteer identified in this Agreement or, if the Volunteer is under 18 years of age, that I am the Volunteer\u{2019}s parent or legal guardian and have full legal authority to enter into this Agreement on the Volunteer\u{2019}s behalf; that all information I have provided is true, accurate, and complete; that I have received, carefully read, understand, and voluntarily agree to this entire Volunteer Agreement, including its confidentiality, intellectual-property, work-for-hire, conduct, assumption-of-risk, release, indemnification, arbitration, background-check authorization, and other provisions; that I have had sufficient time and opportunity to ask questions and seek independent legal advice before signing; that I am signing knowingly and voluntarily, without coercion or undue influence; that I understand this is an unpaid volunteer position and does not create an employment relationship or entitlement to wages, benefits, insurance, or continued volunteer service; that I consent to conducting this transaction and receiving, signing, and retaining this Agreement electronically; and that I specifically intend to adopt the full legal name typed below as my electronic signature. I understand and agree that my electronic signature identifies me, authenticates this Agreement, evidences my intent to be legally bound by all of its terms, and has the same legal validity, force, and effect as my handwritten signature. I further consent to Mommy\u{2019}s Heart, Inc. retaining this electronically signed Agreement and related authentication records, including the date and time of submission, document version, account or email information, and other reasonable audit-trail information, and I agree that accurate electronic copies and records may be used as evidence of my acceptance to the same extent as an original paper document. I confirm that I can access, download, print, and retain a complete copy of this Agreement and understand that I may request a paper copy by contacting Mommy\u{2019}s Heart, Inc. at info@mommysheartinc.org.";

/// The wording of the acknowledgment box itself.
pub const CONSENT_CHECKBOX_LABEL: &str = "I have read, understand, and agree to the Electronic Consent and Signature provision above; I voluntarily accept all terms of this Volunteer Agreement; and I adopt the full legal name typed below as my electronic signature.";

/// The counterparts clause shown beneath the signature blocks.
pub const ELECTRONIC_EXECUTION_HEADING: &str = "Electronic Execution and Counterparts";
pub const ELECTRONIC_EXECUTION: &str = "This Agreement may be executed electronically and in counterparts. Each electronically signed counterpart will be deemed an original, and all counterparts together will constitute one agreement. The Parties agree that electronic signatures and electronic records used in connection with this Agreement will have the same validity, force, and effect as handwritten signatures and original paper records to the fullest extent permitted by applicable law. This Agreement will become effective on the date it is electronically signed by the last Party.";

/// Who countersigns for the Foundation, shown read-only in the acceptance block.
pub const FOUNDATION_SIGNATORY: &str = "Julianne Michelle Reeves Stroh";
pub const FOUNDATION_SIGNATORY_TITLE: &str = "President and Executive Director";

/// Whether `version` is an agreement version this build knows how to honour. Only
/// the current wording may be accepted — an older tab holding a stale version
/// must re-read the agreement rather than consent to text it was never shown.
pub fn is_current(version: &str) -> bool {
    version == VOLUNTEER_AGREEMENT_VERSION
}
