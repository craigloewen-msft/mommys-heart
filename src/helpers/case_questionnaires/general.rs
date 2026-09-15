//! The General Intake: ten sections asked of every new client, ending with the
//! routing questions that decide which specialized intakes are required.

use super::{
    CaseQuestionnaire, QuestionnaireBlock, QuestionnaireInput::*, QuestionnaireQuestion as Q,
    RepeatGroup, ShowWhen,
};

// ── Shared option lists ──────────────────────────────────────────────────────

const YES_NO: &[&str] = &["Yes", "No"];
const YES_NO_UNSURE: &[&str] = &["Yes", "No", "Unsure"];
const YES_NO_UNSURE_DECLINED: &[&str] = &["Yes", "No", "Unsure", "Declined"];
const SAFETY_STATUS: &[&str] = &["Yes", "No", "Unsure", "Not ready", "Declined"];

/// The abuse-screen questions all share this trigger.
const SAFETY_TRIGGER: ShowWhen = ShowWhen::Any(&[
    ShowWhen::AnswerIs {
        key: "Feels safe where staying today",
        values: &["No", "Unsure"],
    },
    ShowWhen::AnswerIs {
        key: "Fear of current or former partner, family, or household member",
        values: &["Yes", "Unsure"],
    },
]);

// ── Section 1: identity, address, safe contact ───────────────────────────────

const SECTION_1: &[Q] = &[
    Q::new("First name", "First name", Text),
    Q::new("Last name", "Last name", Text),
    Q::new("Middle name", "Middle name", Text).optional(),
    Q::new(
        "Prior, maiden, or other legal names",
        "Prior / maiden / other legal names",
        TextArea,
    )
    .optional()
    .help("One per line."),
    Q::new("Preferred name", "Preferred name", Text),
    Q::new("Date of birth", "Date of birth", Date),
    Q::new(
        "Date of birth status",
        "If the date of birth is not known",
        Select(&["Provided above", "Unknown", "Declined"]),
    )
    .optional(),
    Q::new(
        "Gender",
        "Gender",
        SelectOther(&[
            "Woman",
            "Man",
            "Nonbinary",
            "Another identity",
            "Unsure",
            "Declined",
        ]),
    ),
    Q::new(
        "Pronouns",
        "Pronouns",
        SelectOther(&["She/her", "He/him", "They/them", "Other", "Declined"]),
    )
    .optional(),
    Q::new(
        "Preferred language",
        "Preferred language",
        SelectOther(&[
            "English",
            "Spanish",
            "Mandarin",
            "Cantonese",
            "Bengali",
            "Russian",
            "Haitian Creole",
            "Arabic",
            "Korean",
            "French",
            "Urdu",
            "Polish",
            "Other",
            "Declined",
        ]),
    ),
    Q::new(
        "Interpreter needed",
        "Interpreter needed",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Residential street address",
        "Residential street address",
        TextArea,
    )
    .help("Street, unit, city, state, ZIP — or note shelter / no fixed address."),
    Q::new(
        "Address is confidential",
        "Address is confidential",
        Select(&["Yes", "No", "Unsure"]),
    ),
    Q::new("Safe mailing address", "Safe mailing address", TextArea).when(ShowWhen::AnswerIs {
        key: "Address is confidential",
        values: &["Yes", "Unsure"],
    }),
    Q::new("Phone", "Phone", Phone).optional(),
    Q::new("Email", "Email", Email).optional(),
    Q::new(
        "Safe contact methods",
        "Safe contact methods",
        MultiSelect(&["Call", "Text", "Email", "Voicemail", "Mail", "Other"]),
    ),
    Q::new(
        "Unsafe contact methods",
        "Unsafe contact methods",
        MultiSelect(&[
            "Call", "Text", "Email", "Voicemail", "Mail", "None", "Other",
        ]),
    ),
    Q::new(
        "Best time to contact",
        "Best time to contact",
        SelectOther(&[
            "Morning",
            "Afternoon",
            "Evening",
            "Specific window",
            "No preference",
        ]),
    )
    .optional(),
];

// ── Section 2: geography and service area ────────────────────────────────────

const SECTION_2: &[Q] = &[
    Q::new(
        "Is the client located in New York State",
        "Is the client located in New York State?",
        Select(YES_NO),
    ),
    Q::new("Out-of-state county", "Out-of-state county", Text).when(ShowWhen::AnswerIs {
        key: "Is the client located in New York State",
        values: &["No"],
    }),
    Q::new(
        "Out-of-state resource need",
        "Out-of-state resource need",
        MultiSelect(&[
            "DV",
            "Legal",
            "Immigration",
            "Housing",
            "Mental Health",
            "Employment",
            "Benefits",
            "Other",
        ]),
    )
    .when(ShowWhen::AnswerIs {
        key: "Is the client located in New York State",
        values: &["No"],
    }),
];

// ── Section 3: referral source ───────────────────────────────────────────────

const ORGANIZATIONAL_REFERRAL: &[&str] = &[
    "NYC agency",
    "Court",
    "Legal provider",
    "CBO",
    "DV provider",
    "FJC",
    "Healthcare",
    "School or college",
    "Workforce provider",
    "Elected or government office",
    "Faith community",
];

const SECTION_3: &[Q] = &[
    Q::new(
        "Referral source category",
        "Referral source category",
        Select(&[
            "Self",
            "Friend or family",
            "NYC agency",
            "Court",
            "Legal provider",
            "CBO",
            "DV provider",
            "FJC",
            "Healthcare",
            "School or college",
            "Workforce provider",
            "Elected or government office",
            "Faith community",
            "Social media",
            "Website or search",
            "Prior client",
            "Other",
        ]),
    ),
    Q::new(
        "Referring organization",
        "Referring organization",
        OrganizationLookup,
    )
    .when(ShowWhen::AnswerIs {
        key: "Referral source category",
        values: ORGANIZATIONAL_REFERRAL,
    }),
    Q::new("Referring person", "Referring person", Text).optional(),
    Q::new("Referrer contact", "Referrer contact", Text)
        .optional()
        .help("Email or phone."),
    Q::new("Referral date", "Referral date", Date),
    Q::new(
        "Written referral received",
        "Written referral received",
        Select(YES_NO),
    )
    .optional(),
    Q::new("Warm handoff", "Warm handoff", Select(YES_NO)).optional(),
    Q::new(
        "Known funding or program source",
        "Known funding / program source",
        MultiSelect(&[
            "ENDGBV",
            "DoVE",
            "HRA-DSS",
            "DYCD",
            "City Council",
            "OVW-VAWA",
            "Robin Hood",
            "Foundation",
            "General operating",
            "Unrestricted",
            "Other",
            "Unknown",
        ]),
    )
    .optional(),
];

// ── Section 4: household and children ────────────────────────────────────────

const CHILD_FIELDS: &[Q] = &[
    Q::new("full name", "Full name", Text),
    Q::new("date of birth", "Date of birth", Date),
    Q::new(
        "date of birth status",
        "Date of birth status",
        Select(&["Exact", "Approximate", "Unknown"]),
    )
    .optional(),
    Q::new(
        "gender",
        "Gender",
        SelectOther(&[
            "Girl / female",
            "Boy / male",
            "Nonbinary",
            "Another",
            "Unsure",
            "Declined",
        ]),
    ),
    Q::new(
        "lives with client",
        "Lives with client",
        SelectOther(&["Full time", "Part time", "No", "Other"]),
    ),
    Q::new("school or daycare", "School / daycare", Text)
        .optional()
        .help("Name, or N/A."),
    Q::new(
        "educational supports",
        "IEP / 504 / educational supports",
        Select(&["Yes", "No", "Unsure", "N/A"]),
    )
    .optional(),
    Q::new(
        "medical or developmental needs",
        "Medical / developmental / behavioral / sensory / access needs",
        TextArea,
    )
    .optional(),
    Q::new(
        "included in current court case",
        "Included in current court case",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "included in supervised visitation",
        "Included in supervised visitation / safe exchange",
        Select(YES_NO_UNSURE),
    ),
];

const CHILD_REPEAT: &[RepeatGroup] = &[RepeatGroup {
    count_key: "Number of dependent children",
    noun: "Child",
    fields: CHILD_FIELDS,
    max: 12,
}];

const SECTION_4: &[Q] = &[
    Q::new("Household size", "Household size", Number),
    Q::new(
        "Who lives with client",
        "Who lives with client",
        MultiSelect(&[
            "Children",
            "Spouse or partner",
            "Parent",
            "Other relatives",
            "Roommate",
            "Other",
            "Lives alone",
        ]),
    ),
    Q::new(
        "Number of dependent children",
        "Number of dependent children",
        Number,
    ),
];

// ── Section 5: demographics, accessibility, identity ─────────────────────────

const SECTION_5: &[Q] = &[
    Q::new(
        "Race",
        "Race",
        MultiSelect(&[
            "American Indian or Alaska Native",
            "Asian",
            "Black or African American",
            "Native Hawaiian or Other Pacific Islander",
            "White",
            "Middle Eastern or North African",
            "Other",
            "Declined",
        ]),
    ),
    Q::new(
        "Hispanic or Latino origin",
        "Hispanic / Latino origin",
        Select(&["Yes", "No", "Another description", "Declined"]),
    ),
    Q::new(
        "LGBTQIA+ identity",
        "LGBTQIA+ identity",
        Select(YES_NO_UNSURE_DECLINED),
    )
    .optional(),
    Q::new(
        "Disability or access condition",
        "Disability / access condition",
        Select(YES_NO_UNSURE_DECLINED),
    ),
    Q::new(
        "Accommodation requested",
        "Accommodation requested",
        MultiSelect(&[
            "Extra time",
            "Questions read aloud",
            "Written instructions",
            "Interpreter",
            "Breaks",
            "Remote access",
            "Mobility access",
            "Other",
        ]),
    )
    .when(ShowWhen::AnswerIs {
        key: "Disability or access condition",
        values: &["Yes"],
    }),
    Q::new("Country of origin", "Country of origin", Text),
    Q::new(
        "Immigrant identity",
        "Immigrant identity",
        Select(&["Yes", "No", "Unsure", "Prefer not to answer"]),
    ),
    Q::new(
        "Current immigration status",
        "Current immigration status (client-reported)",
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
    )
    .when(ShowWhen::AnswerIs {
        key: "Immigrant identity",
        values: &["Yes", "Unsure"],
    }),
    Q::new(
        "Veteran status",
        "Veteran status",
        Select(&["Yes", "No", "Declined"]),
    )
    .optional(),
];

// ── Section 6: income, employment, benefits ──────────────────────────────────

const INCOME_SOURCES: &[&str] = &[
    "Wages or salary",
    "Self-employment",
    "Gig",
    "Tips",
    "Overtime",
    "Commission",
    "Pension or retirement",
    "Child support",
    "Maintenance",
    "SSI",
    "SSDI",
    "Social Security",
    "Cash Assistance / TANF",
    "Unemployment",
    "Workers comp",
    "Family support",
    "Rental or investment",
    "None",
    "Other",
];

const BENEFITS: &[&str] = &[
    "SNAP",
    "Cash Assistance",
    "Medicaid",
    "Medicare",
    "SSI",
    "SSDI",
    "WIC",
    "Childcare subsidy",
    "Section 8 / HCV",
    "NYCHA",
    "FHEPS",
    "CityFHEPS",
    "Unemployment",
    "HEAP",
    "Other",
    "None",
    "Declined",
];

/// Income detail repeats per reported source; the count question keeps the
/// entry count explicit rather than inferring it from the multi-select.
const INCOME_REPEAT: &[RepeatGroup] = &[
    RepeatGroup {
        count_key: "Number of income sources to detail",
        noun: "Income source",
        fields: &[
            Q::new("name", "Source", Text),
            Q::new("amount", "Gross amount", Currency),
            Q::new(
                "frequency",
                "Frequency",
                Select(&[
                    "Weekly",
                    "Biweekly",
                    "Semimonthly",
                    "Monthly",
                    "Annual",
                    "Variable",
                ]),
            ),
        ],
        max: 8,
    },
    RepeatGroup {
        count_key: "Number of benefits to detail",
        noun: "Benefit",
        fields: &[
            Q::new("name", "Benefit", Text),
            Q::new(
                "status",
                "Status",
                Select(&[
                    "Active",
                    "Pending",
                    "Recertification due",
                    "Denied",
                    "Appealing",
                    "Interrupted",
                    "Unsure",
                ]),
            ),
        ],
        max: 10,
    },
    RepeatGroup {
        count_key: "Number of other household earners",
        noun: "Household earner",
        fields: &[
            Q::new("member", "Household member", Text),
            Q::new("source", "Income source", Text),
            Q::new("amount", "Gross amount", Currency).optional(),
            Q::new(
                "frequency",
                "Frequency",
                Select(&[
                    "Weekly",
                    "Biweekly",
                    "Semimonthly",
                    "Monthly",
                    "Annual",
                    "Variable",
                    "Unknown",
                ]),
            )
            .optional(),
        ],
        max: 8,
    },
];

const SECTION_6: &[Q] = &[
    Q::new(
        "Current employment status",
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
    ),
    Q::new("Income sources", "Income source(s)", MultiSelect(INCOME_SOURCES)),
    Q::new(
        "Number of income sources to detail",
        "How many income sources need amounts recorded?",
        Number,
    )
    .optional()
    .help("Leave blank or zero if the only answer above was None."),
    Q::new(
        "Income verification status",
        "Income verification status",
        Select(&[
            "Client reported",
            "Document verified",
            "Partially verified",
            "Unknown",
        ]),
    )
    .optional(),
    Q::new(
        "Benefits currently received",
        "Benefits currently received",
        MultiSelect(BENEFITS),
    ),
    Q::new(
        "Number of benefits to detail",
        "How many benefits need a status recorded?",
        Number,
    )
    .optional(),
    Q::new(
        "Help with benefits requested",
        "Help with benefits requested",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Number of other household earners",
        "How many other household members have cash income?",
        Number,
    )
    .optional()
    .when(ShowWhen::CountAbove {
        key: "Household size",
        than: 1,
    }),
];

// ── Section 7: housing and basic stability ───────────────────────────────────

const SECTION_7: &[Q] = &[
    Q::new(
        "Current housing situation",
        "Current housing situation",
        SelectOther(&[
            "Own or rent, stable",
            "With family or friends",
            "Shelter",
            "Transitional or supportive",
            "Hotel or motel",
            "At risk of eviction",
            "Unsheltered",
            "Other",
            "Declined",
        ]),
    ),
    Q::new(
        "Can safely remain for 30 days",
        "Can the client safely remain there for the next 30 days?",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Eviction or utility papers",
        "Eviction / housing court / utility papers",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Enough food for several days",
        "Enough food for the household for the next several days",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Enough income for monthly expenses",
        "Enough income for basic monthly expenses",
        Select(YES_NO_UNSURE),
    ),
];

// ── Section 8: systems, barriers, biopsychosocial screen ─────────────────────

const SECTION_8: &[Q] = &[
    Q::new(
        "Systems currently involved",
        "Systems currently involved",
        MultiSelect(&[
            "Family Court",
            "Supreme Court",
            "Housing Court",
            "Criminal Court",
            "ACS-CPS",
            "USCIS",
            "Immigration Court",
            "HRA-DSS",
            "SSA",
            "Medicaid",
            "DOE",
            "Shelter or housing",
            "Healthcare",
            "Child Support",
            "Workforce",
            "Probation or parole",
            "Other",
            "None",
        ]),
    ),
    Q::new(
        "Ability to manage paperwork",
        "Ability to manage paperwork / appointments / deadlines",
        Select(&[
            "Easy",
            "Somewhat difficult",
            "Very difficult",
            "Need significant help",
            "Unsure",
        ]),
    ),
    Q::new(
        "Reliable childcare",
        "Reliable childcare",
        Select(&["Yes", "No", "Sometimes", "Unsure", "N/A"]),
    ),
    Q::new(
        "Reliable transportation",
        "Reliable transportation",
        Select(&["Yes", "No", "Sometimes", "Unsure"]),
    ),
    Q::new(
        "Digital access level",
        "Digital access and skills",
        Select(&[
            "Independent",
            "Some help",
            "Significant help",
            "No access",
        ]),
    ),
    Q::new(
        "Digital tasks needing help",
        "Digital tasks needing help",
        MultiSelect(&[
            "Email",
            "Upload files",
            "Video",
            "Online forms",
            "Other",
        ]),
    )
    .optional(),
    Q::new(
        "Schedule barrier",
        "Work / school / caregiving schedule barrier",
        MultiSelect(&[
            "Work",
            "School",
            "Caregiving",
            "Court",
            "Medical",
            "None",
            "Other",
        ]),
    ),
    Q::new(
        "How manageable things feel",
        "How manageable does everything feel right now?",
        Select(&[
            "Mostly manageable",
            "Difficult but manageable",
            "Very difficult",
            "Overwhelmed",
            "Unsure",
        ]),
    ),
    Q::new(
        "Trusted support person available",
        "Trusted support person available",
        Select(&["Yes", "Sometimes", "No", "Unsure", "Declined"]),
    )
    .optional(),
    Q::new(
        "What is going well",
        "What is going well / stable",
        TextArea,
    )
    .optional(),
    Q::new("Top client priority", "Top client priority", TextArea)
        .help("In the client's own words."),
    Q::new("Priority 2", "Priority 2", Text).optional(),
    Q::new("Priority 3", "Priority 3", Text).optional(),
];

// ── Section 9: domestic violence and safety screen ───────────────────────────

const SECTION_9: &[Q] = &[
    Q::new(
        "Feels safe where staying today",
        "Feels safe where staying today",
        Select(SAFETY_STATUS),
    ),
    Q::new(
        "Fear of current or former partner, family, or household member",
        "Fear of current / former partner, family, or household member",
        Select(SAFETY_STATUS),
    ),
    Q::new("Physical abuse", "Physical abuse", Select(SAFETY_STATUS)).when(SAFETY_TRIGGER),
    Q::new(
        "Strangulation or suffocation",
        "Strangulation / suffocation",
        Select(SAFETY_STATUS),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Sexual violence or coercion",
        "Sexual violence / coercion",
        Select(SAFETY_STATUS),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Psychological or emotional abuse",
        "Psychological / emotional abuse",
        Select(SAFETY_STATUS),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Financial or economic abuse",
        "Financial / economic abuse",
        Select(SAFETY_STATUS),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Coercive control or isolation",
        "Coercive control / isolation",
        Select(SAFETY_STATUS),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Stalking or monitoring",
        "Stalking / monitoring",
        Select(SAFETY_STATUS),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Technology-facilitated abuse",
        "Technology-facilitated abuse",
        Select(SAFETY_STATUS),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Threats or intimidation",
        "Threats / intimidation / threats to kill",
        Select(SAFETY_STATUS),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Immigration-related abuse",
        "Immigration-related abuse / threats involving ICE or immigration authorities",
        Select(SAFETY_STATUS),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Court or litigation abuse",
        "Court / custody / litigation abuse",
        Select(SAFETY_STATUS),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Work, school, or housing interference",
        "Work / school / housing interference",
        MultiSelect(&[
            "Work",
            "School",
            "Housing",
            "Appointments",
            "None",
            "Other",
        ]),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Most recent incident",
        "Most recent incident",
        Select(&[
            "Within 24 hours",
            "Within 7 days",
            "Within 30 days",
            "Within 3 months",
            "Within 1 year",
            "More than 1 year",
            "Ongoing",
            "Unsure",
            "Not ready",
        ]),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Weapons or threatened weapon",
        "Weapons / threatened weapon",
        Select(&["Yes", "No", "Unsure", "Not ready"]),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Order of Protection",
        "Order of Protection",
        Select(&["Current", "Past", "Never", "Unsure"]),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Safety planning requested",
        "Safety planning requested",
        Select(&["Yes", "No", "Unsure", "Not now"]),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Relationship of person causing harm",
        "Relationship of person causing harm to client",
        SelectOther(&[
            "Current spouse",
            "Former spouse",
            "Current intimate partner",
            "Former intimate partner",
            "Dating partner",
            "Co-parent",
            "Family member",
            "Household member",
            "Other",
            "Unsure",
            "Not ready",
            "Declined",
        ]),
    )
    .when(SAFETY_TRIGGER),
    Q::new(
        "Children affected",
        "Children present, exposed, threatened, or otherwise affected",
        Select(SAFETY_STATUS),
    )
    .when(SAFETY_TRIGGER),
];

// ── Section 10: specialized intake routing ───────────────────────────────────

const SECTION_10: &[Q] = &[
    Q::new(
        "Family or matrimonial legal help needed",
        "Family / matrimonial legal help needed",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Immigration help needed",
        "Immigration help needed",
        Select(YES_NO_UNSURE_DECLINED),
    ),
    Q::new(
        "Mental health support wanted",
        "Mental health / counseling support wanted, or functioning affected",
        Select(&["Yes", "No", "Unsure", "Maybe later", "Declined"]),
    ),
    Q::new(
        "Employment help wanted",
        "Employment / career / training help wanted",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Supervised visitation needed",
        "Supervised visitation / monitored or safe exchange ordered, requested, or being considered",
        Select(YES_NO_UNSURE),
    ),
    Q::new(
        "Resource or stabilization help needed",
        "Resource / stabilization help needed",
        MultiSelect(&[
            "Housing",
            "Food",
            "Benefits",
            "Childcare",
            "Transportation",
            "Healthcare",
            "Technology",
            "Documents",
            "Other",
            "None",
        ]),
    ),
];

pub const GENERAL_QUESTIONNAIRE: CaseQuestionnaire = CaseQuestionnaire {
    slug: "general-intake",
    title: "General intake",
    description: "Asked of every new client. The final section decides which specialized intakes are required.",
    section: "Questionnaire: General intake",
    blocks: &[
        QuestionnaireBlock::new(
            "1. Client identity, address, and safe contact",
            "Every new client.",
            SECTION_1,
        ),
        QuestionnaireBlock::new(
            "2. Geography and service area",
            "Determines whether the client is inside the service area.",
            SECTION_2,
        ),
        QuestionnaireBlock::new("3. Referral source", "How the client reached us.", SECTION_3),
        QuestionnaireBlock::new(
            "4. Household and children",
            "Child details appear once dependent children are reported.",
            SECTION_4,
        )
        .repeating(CHILD_REPEAT),
        QuestionnaireBlock::new(
            "5. Demographics, accessibility, and identity",
            "Used for accommodation and for funder reporting.",
            SECTION_5,
        ),
        QuestionnaireBlock::new(
            "6. Income, employment, and benefits",
            "Amounts and statuses are recorded per source below.",
            SECTION_6,
        )
        .repeating(INCOME_REPEAT),
        QuestionnaireBlock::new(
            "7. Housing and basic stability",
            "Immediate stability screen.",
            SECTION_7,
        ),
        QuestionnaireBlock::new(
            "8. Systems, barriers, and biopsychosocial screen",
            "What the client is already navigating, and what makes it harder.",
            SECTION_8,
        ),
        QuestionnaireBlock::new(
            "9. Domestic violence and safety screen",
            "The first two questions are asked of everyone; the rest appear if either indicates a concern.",
            SECTION_9,
        ),
        QuestionnaireBlock::new(
            "10. Specialized intake routing",
            "These answers decide which specialized intakes are required.",
            SECTION_10,
        ),
    ],
};
