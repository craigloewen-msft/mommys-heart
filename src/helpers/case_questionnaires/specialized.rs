//! The seven specialized intakes, each triggered by an answer in the General
//! Intake's routing section.

use super::{
    CaseQuestionnaire, QuestionnaireBlock, QuestionnaireInput::*, QuestionnaireQuestion as Q,
    RepeatGroup, ShowWhen,
};

const YES_NO: &[&str] = &["Yes", "No"];
const YES_NO_UNSURE: &[&str] = &["Yes", "No", "Unsure"];
const YES_NO_UNSURE_DECLINED: &[&str] = &["Yes", "No", "Unsure", "Declined"];
const SAFETY_STATUS: &[&str] = &["Yes", "No", "Unsure", "Not ready", "Declined"];

// ── 1. Legal conflict check ──────────────────────────────────────────────────

const ADVERSE_PARTY_FIELDS: &[Q] = &[
    Q::new("name", "Name", Text),
    Q::new("relationship", "Relationship", Text),
    Q::new("date of birth", "Date of birth if known", Date).optional(),
];

const CONFLICT_QUESTIONS: &[Q] = &[
    Q::new(
        "Client legal name and prior names",
        "Client legal name / prior names",
        TextArea,
    )
    .help("Confirm against the master record."),
    Q::new(
        "Opposing party full name",
        "Opposing / adverse party full legal name",
        Text,
    ),
    Q::new(
        "Opposing party aliases",
        "Opposing party aliases / prior names",
        TextArea,
    )
    .optional()
    .help("One per line."),
    Q::new("Opposing party date of birth", "Opposing party date of birth", Date),
    Q::new(
        "Opposing party date of birth status",
        "If the date of birth is not known",
        Select(&["Provided above", "Unknown"]),
    )
    .optional(),
    Q::new(
        "Opposing party gender",
        "Opposing party gender",
        Select(&[
            "Woman", "Man", "Nonbinary", "Another", "Unsure", "Declined",
        ]),
    ),
    Q::new(
        "Opposing party relationship to client",
        "Relationship to client",
        SelectOther(&[
            "Spouse",
            "Ex-spouse",
            "Partner",
            "Former partner",
            "Co-parent",
            "Family member",
            "Other",
        ]),
    ),
    Q::new("Opposing attorney", "Opposing attorney", ContactLookup).optional(),
    Q::new("Opposing firm", "Opposing firm", OrganizationLookup).optional(),
    Q::new(
        "Number of other adverse parties",
        "How many other adverse parties are there?",
        Number,
    )
    .optional(),
    Q::new(
        "Conflict status",
        "Conflict status",
        Select(&[
            "Pending",
            "Clear",
            "Potential conflict",
            "Confirmed conflict",
            "Supervisor review",
        ]),
    ),
];

pub const LEGAL_CONFLICT_QUESTIONNAIRE: CaseQuestionnaire = CaseQuestionnaire {
    slug: "legal-conflict-check",
    title: "Legal conflict check",
    description: "Run before any legal representation begins.",
    section: "Questionnaire: Legal conflict check",
    blocks: &[QuestionnaireBlock::new(
        "Parties and conflict status",
        "Identify every adverse party so the conflict check is complete.",
        CONFLICT_QUESTIONS,
    )
    .repeating(&[RepeatGroup {
        count_key: "Number of other adverse parties",
        noun: "Adverse party",
        fields: ADVERSE_PARTY_FIELDS,
        max: 8,
    }])],
};

// ── 2. Family and matrimonial ────────────────────────────────────────────────

const CUSTODY_BRANCH: ShowWhen = ShowWhen::AnswerIncludes {
    key: "Legal issue type",
    values: &[
        "Custody",
        "Parenting time",
        "Paternity or parentage",
        "Relocation",
        "Modification",
        "Enforcement or violation",
    ],
};

const SUPPORT_BRANCH: ShowWhen = ShowWhen::AnswerIncludes {
    key: "Legal issue type",
    values: &["Child support", "Maintenance"],
};

const CHILD_SUPPORT_BRANCH: ShowWhen = ShowWhen::AnswerIncludes {
    key: "Legal issue type",
    values: &["Child support"],
};

const MATRIMONIAL_BRANCH: ShowWhen = ShowWhen::AnswerIncludes {
    key: "Legal issue type",
    values: &["Divorce"],
};

const FILED: ShowWhen = ShowWhen::AnswerIs {
    key: "Case already filed",
    values: &["Pending", "Recently decided", "Post-judgment", "Appeal"],
};

const FAMILY_CORE: &[Q] = &[
    Q::new(
        "Legal issue type",
        "Legal issue type",
        MultiSelect(&[
            "Custody",
            "Parenting time",
            "Divorce",
            "Child support",
            "Maintenance",
            "Order of Protection / family offense",
            "Paternity or parentage",
            "Relocation",
            "Enforcement or violation",
            "Modification",
            "Child welfare",
            "Other",
            "Unsure",
        ]),
    ),
    Q::new(
        "Case already filed",
        "Case already filed",
        Select(&[
            "No",
            "Pending",
            "Recently decided",
            "Post-judgment",
            "Appeal",
            "Unsure",
        ]),
    ),
    Q::new(
        "Court type",
        "Court type",
        SelectOther(&[
            "Family Court",
            "Supreme Court matrimonial",
            "IDV",
            "Appellate",
            "Other",
            "Unsure",
        ]),
    )
    .when(FILED),
    Q::new("Court location", "Court state / county / court name", Text).when(FILED),
    Q::new("Case caption", "Case caption", Text)
        .when(FILED)
        .help("The names as they appear on the papers."),
    Q::new("Index or docket number", "Index / docket / petition number", Text).when(FILED),
    Q::new(
        "Judge or referee",
        "Judge / referee / support magistrate",
        Text,
    )
    .when(FILED),
    Q::new("Next court date", "Next court date", Date).optional(),
    Q::new(
        "Other filing or response deadline",
        "Other filing / service / response deadline",
        Date,
    )
    .optional()
    .when(FILED),
    Q::new(
        "Current counsel status",
        "Current counsel status",
        Select(&[
            "Represented",
            "Pro se",
            "Consultation only",
            "Seeking counsel",
            "Unsure",
        ]),
    ),
    Q::new("Current lawyer", "Current lawyer name / organization", Text).when(ShowWhen::AnswerIs {
        key: "Current counsel status",
        values: &["Represented"],
    }),
    Q::new(
        "Other party represented",
        "Other party represented",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Attorney for Child",
        "Attorney for Child",
        Select(YES_NO_UNSURE),
    )
    .when(CUSTODY_BRANCH),
    Q::new(
        "Current orders",
        "Current temporary / final orders",
        MultiSelect(&["None", "Temporary", "Final", "Multiple", "Unsure"]),
    ),
    Q::new(
        "Desired relief",
        "What the client wants help with / desired relief",
        MultiSelect(&[
            "Initial custody",
            "Modify",
            "Enforce",
            "Parenting schedule",
            "Supervised visits",
            "Safe exchange",
            "Relocation",
            "Child support",
            "Maintenance",
            "Arrears",
            "Property",
            "Pension",
            "Debt",
            "Exclusive occupancy",
            "Counsel fees",
            "Other",
            "Unsure",
        ]),
    ),
    Q::new(
        "Documents available",
        "Documents available",
        MultiSelect(&[
            "Petition or complaint",
            "Summons",
            "Motion or OSC",
            "Affidavit",
            "Order",
            "Judgment",
            "Stipulation",
            "Police report",
            "ACS records",
            "Financial docs",
            "Other",
            "None",
        ]),
    ),
];

const FAMILY_CUSTODY: &[Q] = &[
    Q::new(
        "Children involved in the matter",
        "Children involved in the custody or parenting matter",
        TextArea,
    )
    .help("Name the children from the general intake roster."),
    Q::new(
        "Current legal custody arrangement",
        "Current legal custody arrangement",
        SelectOther(&[
            "Joint",
            "Sole to client",
            "Sole to other party",
            "No order",
            "Unsure",
            "Other",
        ]),
    ),
    Q::new(
        "Current primary residence of each child",
        "Current primary residence of each child",
        TextArea,
    )
    .help("Client / other parent / shared / third party / foster care / other."),
    Q::new(
        "Current parenting-time schedule",
        "Current parenting-time schedule",
        TextArea,
    )
    .help("Days, frequency, duration, exchange time and place."),
    Q::new(
        "Current supervision requirement",
        "Current supervision requirement",
        SelectOther(&[
            "None",
            "Professional",
            "Nonprofessional",
            "Agency or site",
            "Unclear",
            "Other",
        ]),
    ),
    Q::new(
        "Current safe-exchange restriction",
        "Current safe-exchange or communication restriction",
        MultiSelect(&[
            "Safe exchange",
            "Staggered arrival",
            "Third-party communication",
            "No direct contact",
            "None",
            "Unsure",
            "Other",
        ]),
    ),
    Q::new(
        "Missed or denied parenting time",
        "Missed, denied, or interrupted parenting time",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Missed parenting time detail",
        "Dates and number of missed occasions",
        TextArea,
    )
    .optional()
    .when(ShowWhen::AnswerIs {
        key: "Missed or denied parenting time",
        values: &["Yes"],
    }),
    Q::new(
        "Evaluator assigned",
        "Attorney for Child or evaluator assigned",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Relocation requested or disputed",
        "Relocation is requested or disputed",
        Select(&[
            "Client seeks relocation",
            "Other party seeks relocation",
            "Disputed move already occurred",
            "No",
            "Unsure",
        ]),
    ),
    Q::new(
        "Custody relief requested",
        "Custody or parenting relief requested",
        MultiSelect(&[
            "Initial order",
            "Modification",
            "Enforcement",
            "Contempt",
            "Parenting time",
            "Supervision",
            "Safe exchange",
            "Relocation",
            "Other",
        ]),
    ),
];

const FAMILY_SUPPORT: &[Q] = &[
    Q::new(
        "Support matter type",
        "Support matter type",
        MultiSelect(&[
            "Child support",
            "Spousal maintenance",
            "Arrears",
            "Enforcement",
            "Modification",
            "Other",
        ]),
    ),
    Q::new(
        "Other party income if known",
        "Other party income or employment if known",
        TextArea,
    )
    .optional(),
    Q::new(
        "Current support order",
        "Current child-support or maintenance order",
        Select(YES_NO_UNSURE),
    ),
    Q::new("Ordered support amount", "Current ordered support amount", Currency).when(
        ShowWhen::AnswerIs {
            key: "Current support order",
            values: &["Yes"],
        },
    ),
    Q::new(
        "Ordered support frequency",
        "Current ordered support frequency",
        Select(&[
            "Weekly",
            "Biweekly",
            "Semimonthly",
            "Monthly",
            "Other",
            "Unknown",
        ]),
    )
    .when(ShowWhen::AnswerIs {
        key: "Current support order",
        values: &["Yes"],
    }),
    Q::new("Claimed support arrears", "Claimed support arrears", Currency).optional(),
    Q::new(
        "Arrears as-of date",
        "Arrears as-of date",
        Date,
    )
    .optional(),
];

const FAMILY_CHILD_SUPPORT: &[Q] = &[
    Q::new(
        "Work-related childcare expense",
        "Work-related childcare expense",
        Currency,
    )
    .optional(),
    Q::new(
        "Child health-insurance responsibility",
        "Child health-insurance responsibility",
        Select(&[
            "Client",
            "Other party",
            "Public coverage",
            "Neither",
            "Unsure",
        ]),
    ),
    Q::new(
        "Child health-insurance cost",
        "Child health-insurance cost",
        Currency,
    )
    .optional(),
    Q::new(
        "Extraordinary child expenses",
        "Extraordinary child expenses",
        MultiSelect(&[
            "Medical",
            "Educational",
            "Childcare",
            "Special needs",
            "Other",
            "None",
            "Unknown",
        ]),
    ),
];

const FAMILY_MATRIMONIAL: &[Q] = &[
    Q::new("Date of marriage", "Date of marriage", Date),
    Q::new(
        "Date of separation",
        "Date of separation",
        Date,
    )
    .optional(),
    Q::new(
        "Marital residence status",
        "Marital residence status",
        SelectOther(&[
            "Client remains",
            "Other party remains",
            "Both remain",
            "Neither remains",
            "Sold",
            "Other",
            "Unsure",
        ]),
    ),
    Q::new(
        "Known marital asset categories",
        "Known marital asset categories",
        MultiSelect(&[
            "Real property",
            "Bank accounts",
            "Vehicles",
            "Personal property",
            "Investments",
            "Other",
            "None known",
            "Unsure",
        ]),
    ),
    Q::new(
        "Known marital debt categories",
        "Known marital debt categories",
        MultiSelect(&[
            "Mortgage",
            "Credit cards",
            "Loans",
            "Tax debt",
            "Medical debt",
            "Other",
            "None known",
            "Unsure",
        ]),
    ),
    Q::new(
        "Pension or retirement interests",
        "Pension or retirement interests",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Pension or retirement detail",
        "Employer / plan if known",
        TextArea,
    )
    .optional()
    .when(ShowWhen::AnswerIs {
        key: "Pension or retirement interests",
        values: &["Yes"],
    }),
    Q::new(
        "Business ownership",
        "Business ownership or business interest",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Maintenance requested or disputed",
        "Maintenance is requested or disputed",
        Select(&[
            "Client requests",
            "Other party requests",
            "Existing order",
            "Not requested",
            "Unsure",
        ]),
    ),
    Q::new(
        "Exclusive occupancy",
        "Exclusive occupancy is requested or ordered",
        Select(&["Requested", "Ordered", "Not requested", "Unsure"]),
    ),
    Q::new(
        "Counsel fees requested",
        "Counsel fees are requested",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Financial disclosure status",
        "Financial disclosure status",
        SelectOther(&[
            "Not started",
            "Net worth statement requested",
            "In progress",
            "Exchanged",
            "Filed",
            "Other",
            "Unsure",
        ]),
    ),
];

pub const FAMILY_QUESTIONNAIRE: CaseQuestionnaire = CaseQuestionnaire {
    slug: "family-matrimonial",
    title: "Family and matrimonial",
    description: "For family or matrimonial legal help, once the conflict check is cleared.",
    section: "Questionnaire: Family and matrimonial",
    blocks: &[
        QuestionnaireBlock::new(
            "Matter, court, and counsel",
            "What is being sought, and where it stands.",
            FAMILY_CORE,
        ),
        QuestionnaireBlock::new(
            "Custody and parenting",
            "Shown when the matter involves custody, parenting time, parentage, or relocation.",
            FAMILY_CUSTODY,
        )
        .when(CUSTODY_BRANCH),
        QuestionnaireBlock::new(
            "Support",
            "Shown when the matter involves child support or maintenance.",
            FAMILY_SUPPORT,
        )
        .when(SUPPORT_BRANCH),
        QuestionnaireBlock::new(
            "Child support detail",
            "Shown when the matter involves child support.",
            FAMILY_CHILD_SUPPORT,
        )
        .when(CHILD_SUPPORT_BRANCH),
        QuestionnaireBlock::new(
            "Matrimonial and financial",
            "Shown when the matter involves divorce.",
            FAMILY_MATRIMONIAL,
        )
        .when(MATRIMONIAL_BRANCH),
    ],
};

// ── 3. Immigration ───────────────────────────────────────────────────────────

const IMMIGRATION_QUESTIONS: &[Q] = &[
    Q::new("Country of citizenship", "Country of citizenship", Text),
    Q::new("Most recent US entry date", "Most recent U.S. entry date", Date).optional(),
    Q::new(
        "Manner or status at entry",
        "Manner / status at entry",
        SelectOther(&[
            "Visa status",
            "Parole",
            "Entered without inspection",
            "Other",
            "Unsure",
            "Declined",
        ]),
    ),
    Q::new(
        "Current immigration status or document",
        "Current immigration status / document if known",
        SelectOther(&[
            "US citizen",
            "LPR",
            "Visa status",
            "Refugee or asylee",
            "TPS",
            "DACA",
            "Pending",
            "Overstay / no current status",
            "Other",
            "Unsure",
            "Declined",
        ]),
    ),
    Q::new(
        "Work authorization",
        "Work authorization",
        Select(&["Valid", "Expired", "Pending", "None", "Unsure", "Declined"]),
    ),
    Q::new(
        "Applications or petitions filed",
        "Immigration applications / petitions filed",
        Select(YES_NO_UNSURE_DECLINED),
    ),
    Q::new(
        "Applications or petitions detail",
        "Which applications, and when",
        TextArea,
    )
    .optional()
    .when(ShowWhen::AnswerIs {
        key: "Applications or petitions filed",
        values: &["Yes"],
    }),
    Q::new(
        "Immigration court or removal matter",
        "Immigration court / removal matter",
        Select(YES_NO_UNSURE_DECLINED),
    ),
    Q::new(
        "USC or LPR family relationship",
        "USC / LPR family relationship",
        MultiSelect(&[
            "USC spouse",
            "USC parent",
            "USC adult child",
            "LPR spouse",
            "LPR parent",
            "Other",
            "None",
            "Unsure",
            "Declined",
        ]),
    ),
    Q::new(
        "Abuse-related immigration concern",
        "Abuse-related immigration concern",
        Select(SAFETY_STATUS),
    ),
    Q::new(
        "Crime victimization",
        "Crime victimization / law enforcement involvement",
        Select(SAFETY_STATUS),
    ),
    Q::new(
        "Trafficking indicators",
        "Trafficking / exploitation indicators",
        Select(SAFETY_STATUS),
    ),
    Q::new(
        "Immigration need",
        "Immigration need",
        MultiSelect(&[
            "Status advice",
            "Work authorization",
            "Family petition",
            "Adjustment",
            "Naturalization",
            "VAWA / abuse-related",
            "U visa review",
            "T visa review",
            "Asylum",
            "Removal",
            "Document replacement",
            "Other",
            "Unsure",
        ]),
    ),
    Q::new(
        "Immigration documents available",
        "Documents available",
        MultiSelect(&[
            "Passport",
            "Visa",
            "I-94",
            "Green card",
            "EAD",
            "Receipt",
            "Approval",
            "Court notice",
            "Petition",
            "Prior decision",
            "Other",
            "None",
        ]),
    ),
];

pub const IMMIGRATION_QUESTIONNAIRE: CaseQuestionnaire = CaseQuestionnaire {
    slug: "immigration",
    title: "Immigration",
    description: "For immigration help, once the conflict check is cleared where required.",
    section: "Questionnaire: Immigration",
    blocks: &[QuestionnaireBlock::new(
        "Status, history, and need",
        "Client-reported; nothing here is legal advice.",
        IMMIGRATION_QUESTIONS,
    )],
};

// ── 4. Mental health and wellbeing ───────────────────────────────────────────

const MENTAL_HEALTH_QUESTIONS: &[Q] = &[
    Q::new(
        "Current mental-health provider",
        "Current mental-health provider",
        Select(YES_NO_UNSURE_DECLINED),
    ),
    Q::new(
        "Would like counseling support",
        "Would the client like counseling / emotional support",
        Select(&["Yes", "No", "Unsure", "Maybe later"]),
    ),
    Q::new(
        "Primary concerns",
        "Primary concern(s)",
        MultiSelect(&[
            "Stress or overwhelm",
            "Trauma reactions",
            "Anxiety or worry",
            "Low mood",
            "Grief",
            "Sleep",
            "Concentration",
            "Parenting stress",
            "DV-related distress",
            "Isolation",
            "Work functioning",
            "Other",
            "None",
            "Unsure",
        ]),
    ),
    Q::new(
        "Functional impact",
        "Functional impact",
        Select(&[
            "Not at all",
            "A little",
            "Moderately",
            "A lot",
            "Extremely",
            "Unsure",
        ]),
    ),
    Q::new(
        "Preferred format",
        "Preferred format",
        MultiSelect(&[
            "Individual",
            "Group",
            "In-person",
            "Telehealth",
            "Either",
            "Unsure",
        ]),
    ),
    Q::new(
        "Insurance or coverage",
        "Insurance / coverage",
        Select(&[
            "Medicaid",
            "Medicare",
            "Private",
            "Marketplace",
            "Uninsured",
            "Unsure",
            "Declined",
        ]),
    ),
    Q::new(
        "Barriers to counseling",
        "Barriers to counseling",
        MultiSelect(&[
            "Childcare",
            "Transportation",
            "Cost",
            "Schedule",
            "Language",
            "Privacy or safety",
            "Technology",
            "Prior negative experience",
            "Other",
            "None",
            "Unsure",
        ]),
    ),
    Q::new(
        "Referral disposition",
        "Referral disposition",
        Select(&[
            "Internal clinical review",
            "External referral",
            "Group or resource",
            "Crisis or safety escalation",
            "Information only",
            "Declined",
            "Not appropriate",
        ]),
    ),
    Q::new(
        "Referral outcome",
        "Referral outcome",
        Select(&[
            "Pending",
            "Appointment scheduled",
            "Attended",
            "Connected ongoing",
            "Waitlisted",
            "Declined",
            "Unable to reach",
            "Other",
        ]),
    )
    .when(ShowWhen::AnswerIs {
        key: "Referral disposition",
        values: &[
            "Internal clinical review",
            "External referral",
            "Group or resource",
            "Crisis or safety escalation",
        ],
    }),
    Q::new(
        "Education status",
        "Education status",
        SelectOther(&[
            "Not enrolled",
            "K-12",
            "HSE or GED",
            "College",
            "Graduate",
            "Vocational training",
            "Other",
            "Unsure",
            "Declined",
        ]),
    ),
];

pub const MENTAL_HEALTH_QUESTIONNAIRE: CaseQuestionnaire = CaseQuestionnaire {
    slug: "mental-health",
    title: "Mental health and wellbeing",
    description: "For clients who want counseling support or whose functioning is affected.",
    section: "Questionnaire: Mental health and wellbeing",
    blocks: &[QuestionnaireBlock::new(
        "Support needs and referral",
        "Screening only; this is not a clinical assessment.",
        MENTAL_HEALTH_QUESTIONS,
    )],
};

// ── 5. Employment and career ─────────────────────────────────────────────────

const EMPLOYED: ShowWhen = ShowWhen::AnswerIs {
    key: "Employment status at intake",
    values: &[
        "Employed full time",
        "Employed part time",
        "Self-employed",
        "Gig or contract",
    ],
};

const EMPLOYMENT_QUESTIONS: &[Q] = &[
    Q::new(
        "Employment status at intake",
        "Current employment status",
        SelectOther(&[
            "Employed full time",
            "Employed part time",
            "Self-employed",
            "Gig or contract",
            "Unemployed seeking",
            "Unemployed not seeking",
            "Student or training",
            "On leave",
            "Retired",
            "Unable to work",
            "Other",
        ]),
    )
    .help("Confirm against the general intake answer."),
    Q::new("Current job title", "Current job title", Text).when(EMPLOYED),
    Q::new("Current industry", "Current industry", Text).when(EMPLOYED),
    Q::new("Current wage or salary", "Current wage / salary", Currency).when(EMPLOYED),
    Q::new("Hours per week", "Hours per week", Text).when(EMPLOYED),
    Q::new(
        "Job benefits",
        "Job benefits",
        MultiSelect(&[
            "Health insurance",
            "Paid leave",
            "Retirement",
            "Other",
            "None",
            "Unsure",
        ]),
    )
    .when(EMPLOYED),
    Q::new(
        "Number of prior jobs to record",
        "How many prior jobs should be recorded?",
        Number,
    )
    .optional(),
    Q::new(
        "Highest education",
        "Highest education",
        SelectOther(&[
            "Less than high school",
            "High school",
            "GED or HSE",
            "Some college",
            "Associate",
            "Bachelor",
            "Graduate or professional",
            "Trade or technical",
            "Foreign credential",
            "Other",
            "Declined",
        ]),
    ),
    Q::new(
        "Credentials and licenses",
        "Credentials / licenses",
        MultiSelect(&[
            "Driver",
            "CDL",
            "CNA-HHA",
            "Security",
            "OSHA",
            "Food handler",
            "Childcare or teaching",
            "IT",
            "Trade",
            "Professional",
            "Other",
            "None",
        ]),
    ),
    Q::new(
        "Digital skills",
        "Digital skills",
        MultiSelect(&[
            "Email",
            "Word processing",
            "Spreadsheets",
            "Video",
            "Online applications",
            "Job boards",
            "Upload and download",
            "Smartphone only",
            "Needs basic support",
            "Other",
        ]),
    ),
    Q::new(
        "Primary career goal",
        "Primary career goal",
        Select(&[
            "Obtain job",
            "Return to workforce",
            "Increase hours",
            "Increase wage",
            "Change career",
            "Credential",
            "Training",
            "Advance",
            "Self-employment",
            "Stabilize job",
            "Explore",
        ]),
    ),
    Q::new("Target roles", "Target roles", TextArea).help("Up to three."),
    Q::new("Target industries", "Target industries", TextArea).optional(),
    Q::new("Minimum or target wage", "Minimum / target wage", Currency),
    Q::new(
        "Desired schedule",
        "Desired schedule",
        MultiSelect(&[
            "Full time",
            "Part time",
            "Day",
            "Evening",
            "Weekend",
        ]),
    ),
    Q::new(
        "Desired modality",
        "Desired modality",
        MultiSelect(&["On-site", "Hybrid", "Remote"]),
    ),
    Q::new("Commute tolerance", "Commute tolerance", Text).optional(),
    Q::new(
        "Job-search readiness",
        "Job-search readiness",
        MultiSelect(&[
            "Resume",
            "LinkedIn",
            "Applications",
            "Interview",
            "References",
            "Networking",
            "Technology",
        ]),
    )
    .help("Tick what is already in place."),
    Q::new(
        "Employment barriers",
        "Employment barriers",
        MultiSelect(&[
            "Childcare",
            "Transportation",
            "Housing",
            "DV safety",
            "Immigration or work authorization question",
            "Disability accommodation",
            "Technology",
            "Benefits cliff",
            "Court demands",
            "Clothing",
            "Limited English",
            "Education or training",
            "Work-history gap",
            "Other",
            "None",
        ]),
    ),
    Q::new(
        "Ready to apply",
        "Ready to apply",
        Select(&[
            "Now",
            "Within 30 days",
            "1-3 months",
            "Not yet",
            "Unsure",
        ]),
    ),
];

pub const EMPLOYMENT_QUESTIONNAIRE: CaseQuestionnaire = CaseQuestionnaire {
    slug: "employment-career",
    title: "Employment and career",
    description: "For clients who want employment, career, or training help.",
    section: "Questionnaire: Employment and career",
    blocks: &[QuestionnaireBlock::new(
        "Work history, goals, and barriers",
        "Current work appears only when the client is working.",
        EMPLOYMENT_QUESTIONS,
    )
    .repeating(&[RepeatGroup {
        count_key: "Number of prior jobs to record",
        noun: "Prior job",
        fields: &[
            Q::new("employer", "Employer", Text),
            Q::new("title", "Title", Text),
            Q::new("dates", "Dates", Text).optional(),
            Q::new("wage", "Wage", Currency).optional(),
            Q::new("reason left", "Reason for leaving", Text).optional(),
        ],
        max: 8,
    }])],
};

// ── 6. Supervised visitation and safe exchange ───────────────────────────────

const SUPERVISED_QUESTIONS: &[Q] = &[
    Q::new(
        "Visitation referral source",
        "Referral source",
        SelectOther(&["Court", "Attorney", "Agency", "Self", "Other"]),
    ),
    Q::new(
        "Visitation referral organization",
        "Referring organization",
        OrganizationLookup,
    )
    .optional(),
    Q::new(
        "Service ordered or requested",
        "Service ordered / requested",
        SelectOther(&[
            "Supervised visitation",
            "Monitored exchange",
            "Safe exchange",
            "Other",
        ]),
    ),
    Q::new(
        "Governing court order",
        "Governing court order / written referral",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Visitation court details",
        "Court / judge / docket / order date / review or expiration",
        TextArea,
    )
    .help("Pull from the family intake where available, then verify."),
    Q::new(
        "Children included in visitation",
        "Children included",
        TextArea,
    )
    .help("Name the children from the general intake roster."),
    Q::new("Visiting parent", "Visiting parent", Text),
    Q::new("Custodial parent", "Custodial parent", Text),
    Q::new(
        "Number of authorized pickup persons",
        "How many authorized pickup persons are there?",
        Number,
    )
    .optional(),
    Q::new(
        "Required frequency and duration",
        "Required / requested frequency and duration",
        TextArea,
    ),
    Q::new(
        "Last contact or visitation history",
        "Last contact / current visitation history",
        TextArea,
    ),
    Q::new(
        "Relevant DV or safety concerns",
        "Relevant DV / safety concerns",
        MultiSelect(&[
            "Stalking",
            "Threats",
            "Weapons",
            "Coercive control",
            "Child safety",
            "Other",
            "None known",
        ]),
    ),
    Q::new(
        "Order of Protection in visitation",
        "Order of Protection / prohibited contact",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Order of Protection detail",
        "Protected parties / restrictions / expiration if known",
        TextArea,
    )
    .optional()
    .when(ShowWhen::AnswerIs {
        key: "Order of Protection in visitation",
        values: &["Yes"],
    }),
    Q::new(
        "Child-protective restrictions",
        "ACS-CPS / child-protective restrictions",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Child accessibility needs for visits",
        "Child accessibility / medical / developmental needs relevant to visits",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Interpreter need for visits",
        "Interpreter / language need",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Separate waiting or staggered arrival",
        "Separate waiting / staggered arrival or departure needed",
        MultiSelect(&[
            "Separate waiting",
            "Staggered arrival",
            "Staggered departure",
            "Separate communication",
            "Other",
            "None",
        ]),
    ),
    Q::new("Assigned location", "Assigned location", Text),
    Q::new("Primary monitor", "Primary monitor", ContactLookup),
    Q::new("Second monitor", "Second monitor", ContactLookup).optional(),
    Q::new("Program start date", "Program start date", Date),
    Q::new("Program review date", "Program review date", Date).optional(),
    Q::new(
        "Planned supervision model",
        "Planned supervision model / level",
        SelectOther(&[
            "Standard group milieu",
            "Enhanced lower-ratio group",
            "1:1 supervision",
            "Safe exchange only",
            "Other",
            "Referred out",
        ]),
    ),
    Q::new(
        "Monitors assigned",
        "Monitors assigned",
        Number,
    ),
    Q::new(
        "Maximum concurrent families",
        "Maximum concurrent families for this case or shift",
        Number,
    ),
];

pub const SUPERVISED_VISITATION_QUESTIONNAIRE: CaseQuestionnaire = CaseQuestionnaire {
    slug: "supervised-visitation",
    title: "Supervised visitation and safe exchange",
    description: "For visitation or exchange that is ordered, requested, or being considered.",
    section: "Questionnaire: Supervised visitation and safe exchange",
    blocks: &[QuestionnaireBlock::new(
        "Referral, parties, safety, and staffing",
        "Safety details here drive how visits are staffed and scheduled.",
        SUPERVISED_QUESTIONS,
    )
    .repeating(&[RepeatGroup {
        count_key: "Number of authorized pickup persons",
        noun: "Authorized person",
        fields: &[
            Q::new("name", "Name", Text),
            Q::new("relationship", "Relationship to child", Text),
            Q::new("contact", "Contact", Text).optional(),
        ],
        max: 6,
    }])],
};

// ── 7. Resource navigation ───────────────────────────────────────────────────

const RESOURCE_QUESTIONS: &[Q] = &[
    Q::new(
        "Need or service category",
        "Need / service category",
        MultiSelect(&[
            "Legal",
            "DV",
            "Immigration",
            "Housing",
            "Benefits",
            "Mental health",
            "Healthcare",
            "Employment",
            "Childcare",
            "Transportation",
            "Other",
        ]),
    ),
    Q::new(
        "Client location for referral",
        "Client location for referral",
        Text,
    )
    .help("State / county / ZIP."),
    Q::new(
        "Referral organization",
        "Referral organization / provider",
        OrganizationLookup,
    ),
    Q::new("Eligibility notes", "Eligibility / contact notes", TextArea).optional(),
    Q::new("Resource warm handoff", "Warm handoff", Select(YES_NO)),
    Q::new(
        "Referral status",
        "Referral status",
        Select(&[
            "Sent",
            "Contact attempted",
            "Appointment scheduled",
            "Connected",
            "Waitlisted",
            "Denied",
            "Unable to reach",
            "Client declined",
            "Alternate referral needed",
        ]),
    ),
    Q::new("Follow-up date", "Follow-up date", Date).when(ShowWhen::AnswerIs {
        key: "Referral status",
        values: &[
            "Sent",
            "Contact attempted",
            "Waitlisted",
            "Unable to reach",
            "Alternate referral needed",
        ],
    }),
];

pub const RESOURCE_QUESTIONNAIRE: CaseQuestionnaire = CaseQuestionnaire {
    slug: "resource-navigation",
    title: "Resource navigation",
    description: "For stabilization help, outside referrals, or clients outside New York State.",
    section: "Questionnaire: Resource navigation",
    blocks: &[QuestionnaireBlock::new(
        "Need, provider, and outcome",
        "Track the referral through to a connection.",
        RESOURCE_QUESTIONS,
    )],
};
