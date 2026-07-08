/**
 * CORS for the dedicated API.
 *
 * The embeddable Squarespace chat widget calls `/api/*` from a different origin,
 * so we echo an allowed origin and answer preflight requests. Configured via
 * `NUXT_ALLOWED_ORIGINS` (comma-separated, or "*" for any origin) — parity with
 * the legacy FastAPI CORSMiddleware.
 */
export default defineEventHandler((event) => {
  if (!event.path.startsWith('/api/')) return

  const allowed = useRuntimeConfig()
    .allowedOrigins.split(',')
    .map((o) => o.trim())
    .filter(Boolean)

  const requestOrigin = getRequestHeader(event, 'origin')
  const allowAny = allowed.includes('*')

  let allowOrigin: string | null = null
  if (allowAny) {
    allowOrigin = requestOrigin ?? '*'
  } else if (requestOrigin && allowed.includes(requestOrigin)) {
    allowOrigin = requestOrigin
  }

  if (allowOrigin) {
    setResponseHeader(event, 'Access-Control-Allow-Origin', allowOrigin)
    setResponseHeader(event, 'Vary', 'Origin')
    setResponseHeader(event, 'Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
    setResponseHeader(event, 'Access-Control-Allow-Headers', 'Content-Type, Authorization')
  }

  // Short-circuit CORS preflight requests.
  if (event.method === 'OPTIONS') {
    setResponseStatus(event, 204)
    return ''
  }
})
