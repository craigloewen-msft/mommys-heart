//! In-memory mock CRM data for the template stage. Replaced by a real database
//! in a later phase.

use crate::types::{Contact, ContactStatus};

pub fn contacts() -> Vec<Contact> {
    vec![
        Contact {
            id: "1".into(),
            name: "Amara Okafor".into(),
            email: "amara@brightpathngo.org".into(),
            phone: "+1 (415) 555-0142".into(),
            company: "Bright Path NGO".into(),
            status: ContactStatus::Active,
            notes: "Recurring monthly donor. Interested in the maternal health program.".into(),
            created_at: "2026-02-11T09:24:00Z".into(),
        },
        Contact {
            id: "2".into(),
            name: "Daniel Reyes".into(),
            email: "dreyes@example.com".into(),
            phone: "+1 (206) 555-0110".into(),
            company: "Reyes Family Foundation".into(),
            status: ContactStatus::Lead,
            notes: "Reached out via the website chat widget. Wants a grant proposal.".into(),
            created_at: "2026-05-03T15:40:00Z".into(),
        },
        Contact {
            id: "3".into(),
            name: "Priya Nair".into(),
            email: "priya.nair@example.com".into(),
            phone: "+1 (312) 555-0175".into(),
            company: "Volunteer".into(),
            status: ContactStatus::Active,
            notes: "Lead volunteer coordinator for the spring fundraiser.".into(),
            created_at: "2025-11-20T18:05:00Z".into(),
        },
        Contact {
            id: "4".into(),
            name: "Tom Whitfield".into(),
            email: "tom.w@example.com".into(),
            phone: "+1 (617) 555-0198".into(),
            company: "Whitfield & Co.".into(),
            status: ContactStatus::Inactive,
            notes: "Past corporate sponsor. Follow up before the next campaign.".into(),
            created_at: "2025-08-14T12:00:00Z".into(),
        },
    ]
}
