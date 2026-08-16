//! The initial contact-directory taxonomy supplied by the organization.
//!
//! These presets live in code like the default case fields and folders. The
//! categories themselves remain database rows so contacts can reference them
//! and authorized users can add more at runtime.

pub struct ContactCategoryPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub children: &'static [ContactSubcategoryPreset],
}

pub struct ContactSubcategoryPreset {
    pub id: &'static str,
    pub name: &'static str,
}

pub const CONTACT_CATEGORIES: &[ContactCategoryPreset] = &[
    ContactCategoryPreset {
        id: "cc-legal",
        name: "Legal",
        children: &[
            ContactSubcategoryPreset {
                id: "cc-law-firms",
                name: "Law firms",
            },
            ContactSubcategoryPreset {
                id: "cc-individual-attorneys",
                name: "Individual attorneys",
            },
            ContactSubcategoryPreset {
                id: "cc-family-law",
                name: "Family law",
            },
            ContactSubcategoryPreset {
                id: "cc-domestic-violence-legal",
                name: "Domestic violence",
            },
            ContactSubcategoryPreset {
                id: "cc-matrimonial-law",
                name: "Matrimonial law",
            },
            ContactSubcategoryPreset {
                id: "cc-appellate-law",
                name: "Appellate law",
            },
            ContactSubcategoryPreset {
                id: "cc-law-schools",
                name: "Law schools",
            },
            ContactSubcategoryPreset {
                id: "cc-law-school-clinics",
                name: "Law school clinics",
            },
            ContactSubcategoryPreset {
                id: "cc-legal-services-organizations",
                name: "Legal services organizations",
            },
        ],
    },
    ContactCategoryPreset {
        id: "cc-social-services",
        name: "Social Services",
        children: &[
            ContactSubcategoryPreset {
                id: "cc-housing",
                name: "Housing",
            },
            ContactSubcategoryPreset {
                id: "cc-public-benefits",
                name: "Public benefits",
            },
            ContactSubcategoryPreset {
                id: "cc-domestic-violence-services",
                name: "Domestic violence services",
            },
            ContactSubcategoryPreset {
                id: "cc-mental-health-services",
                name: "Mental health",
            },
            ContactSubcategoryPreset {
                id: "cc-child-family-services",
                name: "Child / family services",
            },
            ContactSubcategoryPreset {
                id: "cc-food-assistance",
                name: "Food assistance",
            },
            ContactSubcategoryPreset {
                id: "cc-financial-assistance",
                name: "Financial assistance",
            },
            ContactSubcategoryPreset {
                id: "cc-employment-career-services",
                name: "Employment / career services",
            },
            ContactSubcategoryPreset {
                id: "cc-community-based-organizations",
                name: "Community-based organizations",
            },
        ],
    },
    ContactCategoryPreset {
        id: "cc-education",
        name: "Education",
        children: &[
            ContactSubcategoryPreset {
                id: "cc-colleges-universities",
                name: "Colleges / universities",
            },
            ContactSubcategoryPreset {
                id: "cc-graduate-schools",
                name: "Graduate schools",
            },
            ContactSubcategoryPreset {
                id: "cc-social-work-schools",
                name: "Social work schools",
            },
            ContactSubcategoryPreset {
                id: "cc-education-law-schools",
                name: "Law schools",
            },
            ContactSubcategoryPreset {
                id: "cc-field-placement-offices",
                name: "Internship / field placement offices",
            },
            ContactSubcategoryPreset {
                id: "cc-career-services",
                name: "Career services",
            },
            ContactSubcategoryPreset {
                id: "cc-alumni-associations",
                name: "Alumni associations",
            },
            ContactSubcategoryPreset {
                id: "cc-faculty-professors",
                name: "Faculty / professors",
            },
        ],
    },
    ContactCategoryPreset {
        id: "cc-medical-mental-health",
        name: "Medical / Mental Health",
        children: &[
            ContactSubcategoryPreset {
                id: "cc-therapists",
                name: "Therapists",
            },
            ContactSubcategoryPreset {
                id: "cc-social-workers",
                name: "Social workers",
            },
            ContactSubcategoryPreset {
                id: "cc-psychologists",
                name: "Psychologists",
            },
            ContactSubcategoryPreset {
                id: "cc-psychiatrists",
                name: "Psychiatrists",
            },
            ContactSubcategoryPreset {
                id: "cc-physicians",
                name: "Physicians",
            },
            ContactSubcategoryPreset {
                id: "cc-clinics-hospitals",
                name: "Clinics / hospitals",
            },
        ],
    },
    ContactCategoryPreset {
        id: "cc-government-public-affairs",
        name: "Government / Public Affairs",
        children: &[
            ContactSubcategoryPreset {
                id: "cc-public-officials",
                name: "Public officials",
            },
            ContactSubcategoryPreset {
                id: "cc-retired-public-officials",
                name: "Retired public officials",
            },
            ContactSubcategoryPreset {
                id: "cc-government-agencies",
                name: "Government agencies",
            },
            ContactSubcategoryPreset {
                id: "cc-legislative-offices",
                name: "Legislative offices",
            },
            ContactSubcategoryPreset {
                id: "cc-policy-advocacy-organizations",
                name: "Policy / advocacy organizations",
            },
        ],
    },
    ContactCategoryPreset {
        id: "cc-media-entertainment",
        name: "Media / Entertainment",
        children: &[
            ContactSubcategoryPreset {
                id: "cc-actors",
                name: "Actors",
            },
            ContactSubcategoryPreset {
                id: "cc-musicians",
                name: "Musicians",
            },
            ContactSubcategoryPreset {
                id: "cc-tv-hosts",
                name: "TV hosts",
            },
            ContactSubcategoryPreset {
                id: "cc-newscasters",
                name: "Newscasters",
            },
            ContactSubcategoryPreset {
                id: "cc-reporters-journalists",
                name: "Reporters / journalists",
            },
            ContactSubcategoryPreset {
                id: "cc-producers",
                name: "Producers",
            },
            ContactSubcategoryPreset {
                id: "cc-media-organizations",
                name: "Media organizations",
            },
        ],
    },
    ContactCategoryPreset {
        id: "cc-philanthropy-fundraising",
        name: "Philanthropy / Fundraising",
        children: &[
            ContactSubcategoryPreset {
                id: "cc-philanthropists",
                name: "Philanthropists",
            },
            ContactSubcategoryPreset {
                id: "cc-foundations",
                name: "Foundations",
            },
            ContactSubcategoryPreset {
                id: "cc-corporate-sponsors",
                name: "Corporate sponsors",
            },
            ContactSubcategoryPreset {
                id: "cc-donors",
                name: "Donors",
            },
            ContactSubcategoryPreset {
                id: "cc-prospective-donors",
                name: "Prospective donors",
            },
            ContactSubcategoryPreset {
                id: "cc-funders-grantmakers",
                name: "Funders / grantmaking organizations",
            },
        ],
    },
    ContactCategoryPreset {
        id: "cc-business-corporate",
        name: "Business / Corporate",
        children: &[
            ContactSubcategoryPreset {
                id: "cc-companies",
                name: "Companies",
            },
            ContactSubcategoryPreset {
                id: "cc-corporate-partners",
                name: "Corporate partners",
            },
            ContactSubcategoryPreset {
                id: "cc-professional-services",
                name: "Professional services",
            },
            ContactSubcategoryPreset {
                id: "cc-vendors",
                name: "Vendors",
            },
            ContactSubcategoryPreset {
                id: "cc-prospective-sponsors",
                name: "Prospective sponsors",
            },
        ],
    },
    ContactCategoryPreset {
        id: "cc-nonprofit-community",
        name: "Nonprofit / Community",
        children: &[
            ContactSubcategoryPreset {
                id: "cc-nonprofit-organizations",
                name: "Nonprofit organizations",
            },
            ContactSubcategoryPreset {
                id: "cc-advocacy-organizations",
                name: "Advocacy organizations",
            },
            ContactSubcategoryPreset {
                id: "cc-community-organizations",
                name: "Community organizations",
            },
            ContactSubcategoryPreset {
                id: "cc-referral-partners",
                name: "Referral partners",
            },
            ContactSubcategoryPreset {
                id: "cc-prospective-partners",
                name: "Prospective partners",
            },
        ],
    },
];
