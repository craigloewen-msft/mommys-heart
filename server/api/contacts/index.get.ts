import type { Contact } from '~/shared/types/api'
import { contacts } from '~/server/data/contacts'

// GET /api/contacts — list all CRM contacts (mock data for now).
export default defineEventHandler((): Contact[] => {
  return contacts
})
