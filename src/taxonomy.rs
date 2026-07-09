//! Built-in service taxonomy shared by the server and the client.
//!
//! The organization catalogs a client's needs against a fixed, two-level
//! taxonomy: five top-level [`ServiceCategory`] areas, each containing several
//! granular [`ServiceType`] services. Cases are tagged with one or more service
//! types so we can understand both the individual service components and the
//! broader client journey across categories.

use serde::{Deserialize, Serialize};

/// A top-level area of service the organization provides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceCategory {
    FamilyLaw,
    Housing,
    Immigration,
    SocialServices,
    MentalHealth,
}

impl ServiceCategory {
    /// Every category, in display order.
    pub const ALL: [ServiceCategory; 5] = [
        ServiceCategory::FamilyLaw,
        ServiceCategory::Housing,
        ServiceCategory::Immigration,
        ServiceCategory::SocialServices,
        ServiceCategory::MentalHealth,
    ];

    /// Human label for display.
    pub fn label(self) -> &'static str {
        match self {
            ServiceCategory::FamilyLaw => "Family Law",
            ServiceCategory::Housing => "Housing",
            ServiceCategory::Immigration => "Immigration",
            ServiceCategory::SocialServices => "Social Services",
            ServiceCategory::MentalHealth => "Mental Health & Support",
        }
    }

    /// URL/serialization-friendly identifier.
    pub fn slug(self) -> &'static str {
        match self {
            ServiceCategory::FamilyLaw => "family_law",
            ServiceCategory::Housing => "housing",
            ServiceCategory::Immigration => "immigration",
            ServiceCategory::SocialServices => "social_services",
            ServiceCategory::MentalHealth => "mental_health",
        }
    }

    /// Parse a category from its slug.
    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.slug() == s)
    }

    /// The service types that belong to this category, in display order.
    pub fn types(self) -> &'static [ServiceType] {
        match self {
            ServiceCategory::FamilyLaw => &[
                ServiceType::Divorce,
                ServiceType::CustodyVisitation,
                ServiceType::ChildSupport,
                ServiceType::OrdersOfProtection,
            ],
            ServiceCategory::Housing => &[
                ServiceType::ShelterPlacement,
                ServiceType::RentalArrears,
                ServiceType::EvictionPrevention,
                ServiceType::HousingSubsidies,
            ],
            ServiceCategory::Immigration => &[
                ServiceType::Vawa,
                ServiceType::AdjustmentOfStatus,
                ServiceType::WorkAuthorization,
                ServiceType::Naturalization,
            ],
            ServiceCategory::SocialServices => &[
                ServiceType::Snap,
                ServiceType::CashAssistance,
                ServiceType::ChildCareAssistance,
                ServiceType::EmergencyFinancialAssistance,
            ],
            ServiceCategory::MentalHealth => &[
                ServiceType::TherapyReferrals,
                ServiceType::CrisisIntervention,
                ServiceType::SafetyPlanning,
                ServiceType::SupportGroups,
            ],
        }
    }

    /// Tailwind classes for a category-colored tag/badge.
    pub fn badge_classes(self) -> &'static str {
        match self {
            ServiceCategory::FamilyLaw => {
                "bg-violet-500/15 text-violet-300 ring-1 ring-violet-500/30"
            }
            ServiceCategory::Housing => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
            ServiceCategory::Immigration => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            ServiceCategory::SocialServices => {
                "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
            }
            ServiceCategory::MentalHealth => "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30",
        }
    }
}

/// A granular service within a [`ServiceCategory`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceType {
    // Family Law
    Divorce,
    CustodyVisitation,
    ChildSupport,
    OrdersOfProtection,
    // Housing
    ShelterPlacement,
    RentalArrears,
    EvictionPrevention,
    HousingSubsidies,
    // Immigration
    Vawa,
    AdjustmentOfStatus,
    WorkAuthorization,
    Naturalization,
    // Social Services
    Snap,
    CashAssistance,
    ChildCareAssistance,
    EmergencyFinancialAssistance,
    // Mental Health & Support
    TherapyReferrals,
    CrisisIntervention,
    SafetyPlanning,
    SupportGroups,
}

impl ServiceType {
    /// Every service type, in category display order.
    pub const ALL: [ServiceType; 20] = [
        ServiceType::Divorce,
        ServiceType::CustodyVisitation,
        ServiceType::ChildSupport,
        ServiceType::OrdersOfProtection,
        ServiceType::ShelterPlacement,
        ServiceType::RentalArrears,
        ServiceType::EvictionPrevention,
        ServiceType::HousingSubsidies,
        ServiceType::Vawa,
        ServiceType::AdjustmentOfStatus,
        ServiceType::WorkAuthorization,
        ServiceType::Naturalization,
        ServiceType::Snap,
        ServiceType::CashAssistance,
        ServiceType::ChildCareAssistance,
        ServiceType::EmergencyFinancialAssistance,
        ServiceType::TherapyReferrals,
        ServiceType::CrisisIntervention,
        ServiceType::SafetyPlanning,
        ServiceType::SupportGroups,
    ];

    /// The category this service belongs to.
    pub fn category(self) -> ServiceCategory {
        match self {
            ServiceType::Divorce
            | ServiceType::CustodyVisitation
            | ServiceType::ChildSupport
            | ServiceType::OrdersOfProtection => ServiceCategory::FamilyLaw,
            ServiceType::ShelterPlacement
            | ServiceType::RentalArrears
            | ServiceType::EvictionPrevention
            | ServiceType::HousingSubsidies => ServiceCategory::Housing,
            ServiceType::Vawa
            | ServiceType::AdjustmentOfStatus
            | ServiceType::WorkAuthorization
            | ServiceType::Naturalization => ServiceCategory::Immigration,
            ServiceType::Snap
            | ServiceType::CashAssistance
            | ServiceType::ChildCareAssistance
            | ServiceType::EmergencyFinancialAssistance => ServiceCategory::SocialServices,
            ServiceType::TherapyReferrals
            | ServiceType::CrisisIntervention
            | ServiceType::SafetyPlanning
            | ServiceType::SupportGroups => ServiceCategory::MentalHealth,
        }
    }

    /// Human label for display.
    pub fn label(self) -> &'static str {
        match self {
            ServiceType::Divorce => "Divorce",
            ServiceType::CustodyVisitation => "Custody & visitation",
            ServiceType::ChildSupport => "Child support",
            ServiceType::OrdersOfProtection => "Orders of protection",
            ServiceType::ShelterPlacement => "Shelter placement",
            ServiceType::RentalArrears => "Rental arrears",
            ServiceType::EvictionPrevention => "Eviction prevention",
            ServiceType::HousingSubsidies => "Housing subsidies",
            ServiceType::Vawa => "VAWA",
            ServiceType::AdjustmentOfStatus => "Adjustment of status",
            ServiceType::WorkAuthorization => "Work authorization",
            ServiceType::Naturalization => "Naturalization",
            ServiceType::Snap => "SNAP",
            ServiceType::CashAssistance => "Cash assistance",
            ServiceType::ChildCareAssistance => "Child care assistance",
            ServiceType::EmergencyFinancialAssistance => "Emergency financial assistance",
            ServiceType::TherapyReferrals => "Therapy referrals",
            ServiceType::CrisisIntervention => "Crisis intervention",
            ServiceType::SafetyPlanning => "Safety planning",
            ServiceType::SupportGroups => "Support groups",
        }
    }

    /// URL/serialization-friendly identifier.
    pub fn slug(self) -> &'static str {
        match self {
            ServiceType::Divorce => "divorce",
            ServiceType::CustodyVisitation => "custody_visitation",
            ServiceType::ChildSupport => "child_support",
            ServiceType::OrdersOfProtection => "orders_of_protection",
            ServiceType::ShelterPlacement => "shelter_placement",
            ServiceType::RentalArrears => "rental_arrears",
            ServiceType::EvictionPrevention => "eviction_prevention",
            ServiceType::HousingSubsidies => "housing_subsidies",
            ServiceType::Vawa => "vawa",
            ServiceType::AdjustmentOfStatus => "adjustment_of_status",
            ServiceType::WorkAuthorization => "work_authorization",
            ServiceType::Naturalization => "naturalization",
            ServiceType::Snap => "snap",
            ServiceType::CashAssistance => "cash_assistance",
            ServiceType::ChildCareAssistance => "child_care_assistance",
            ServiceType::EmergencyFinancialAssistance => "emergency_financial_assistance",
            ServiceType::TherapyReferrals => "therapy_referrals",
            ServiceType::CrisisIntervention => "crisis_intervention",
            ServiceType::SafetyPlanning => "safety_planning",
            ServiceType::SupportGroups => "support_groups",
        }
    }

    /// Parse a service type from its slug.
    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.slug() == s)
    }

    /// Tailwind classes for a tag/badge, colored by the parent category.
    pub fn badge_classes(self) -> &'static str {
        self.category().badge_classes()
    }
}
