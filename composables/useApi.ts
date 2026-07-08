import type {
  ChatRequest,
  ChatResponse,
  Contact,
  HealthResponse,
  VersionResponse,
} from '~/shared/types/api'

/**
 * Typed client for the dedicated API.
 *
 * The CRM website talks to the backend exclusively through these helpers, so
 * request/response shapes stay in sync with the server via the shared types.
 */
export function useApi() {
  return {
    health: () => $fetch<HealthResponse>('/api/health'),
    version: () => $fetch<VersionResponse>('/api/version'),
    listContacts: () => $fetch<Contact[]>('/api/contacts'),
    getContact: (id: string) => $fetch<Contact>(`/api/contacts/${id}`),
    chat: (body: ChatRequest) =>
      $fetch<ChatResponse>('/api/chat', { method: 'POST', body }),
  }
}
