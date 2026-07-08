import type { Contact } from '~/shared/types/api'

// In-memory mock CRM data for the template stage. Replaced by a real database
// (SQLite via Drizzle) in a later phase.
export const contacts: Contact[] = [
  {
    id: '1',
    name: 'Amara Okafor',
    email: 'amara@brightpathngo.org',
    phone: '+1 (415) 555-0142',
    company: 'Bright Path NGO',
    status: 'active',
    notes: 'Recurring monthly donor. Interested in the maternal health program.',
    createdAt: '2026-02-11T09:24:00Z',
  },
  {
    id: '2',
    name: 'Daniel Reyes',
    email: 'dreyes@example.com',
    phone: '+1 (206) 555-0110',
    company: 'Reyes Family Foundation',
    status: 'lead',
    notes: 'Reached out via the website chat widget. Wants a grant proposal.',
    createdAt: '2026-05-03T15:40:00Z',
  },
  {
    id: '3',
    name: 'Priya Nair',
    email: 'priya.nair@example.com',
    phone: '+1 (312) 555-0175',
    company: 'Volunteer',
    status: 'active',
    notes: 'Lead volunteer coordinator for the spring fundraiser.',
    createdAt: '2025-11-20T18:05:00Z',
  },
  {
    id: '4',
    name: 'Tom Whitfield',
    email: 'tom.w@example.com',
    phone: '+1 (617) 555-0198',
    company: 'Whitfield & Co.',
    status: 'inactive',
    notes: 'Past corporate sponsor. Follow up before the next campaign.',
    createdAt: '2025-08-14T12:00:00Z',
  },
]
