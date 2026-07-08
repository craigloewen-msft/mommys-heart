// Shared API contract types, used by both the Nitro server routes and the Vue
// client. Keeping these in one place guarantees the website and the dedicated
// API never drift apart.

/** A single cited source returned by the RAG chat endpoint. */
export interface SourceInfo {
  filename: string
  heading: string
  snippet: string
  relevance: number
  anchor: string
}

/** Where a chat answer came from — mirrors the legacy Python contract. */
export type SourceType = 'documents' | 'general_knowledge' | 'mixed'

/** POST /api/chat request body. */
export interface ChatRequest {
  message: string
  captcha_token?: string | null
}

/** POST /api/chat response body. */
export interface ChatResponse {
  answer: string
  sources: SourceInfo[]
  source_type: SourceType
}

/** GET /api/version response body. */
export interface VersionResponse {
  version: string
  chat_model: string
  embedding_model: string
  captcha_enabled: boolean
}

/** GET /api/health response body. */
export interface HealthResponse {
  status: 'ok'
}

// --- CRM domain types (template/mock stage) ---

export interface Contact {
  id: string
  name: string
  email: string
  phone: string
  company: string
  status: 'lead' | 'active' | 'inactive'
  notes: string
  createdAt: string
}
