import type { Contact } from '~/shared/types/api'
import { contacts } from '~/server/data/contacts'

// GET /api/contacts/:id — fetch a single CRM contact (mock data for now).
export default defineEventHandler((event): Contact => {
  const id = getRouterParam(event, 'id')
  const contact = contacts.find((c) => c.id === id)
  if (!contact) {
    throw createError({ statusCode: 404, statusMessage: `Contact '${id}' not found` })
  }
  return contact
})
