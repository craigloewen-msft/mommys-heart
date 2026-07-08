import type { HealthResponse } from '~/shared/types/api'

// Lightweight liveness probe (parity with the legacy /healthz).
export default defineEventHandler((): HealthResponse => {
  return { status: 'ok' }
})
