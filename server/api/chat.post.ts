import type { ChatRequest, ChatResponse } from '~/shared/types/api'
import { verifyCaptcha } from '~/server/utils/captcha'

/**
 * POST /api/chat — the dedicated chat endpoint consumed by both the CRM website
 * and the embeddable Squarespace widget.
 *
 * NOTE: this is a STUB. It preserves the legacy response contract
 * (`answer`, `sources`, `source_type`) and the CAPTCHA gate, but returns a
 * placeholder answer. Porting the real RAG pipeline (Azure OpenAI embeddings +
 * vector search over the docx corpus in legacy/docs) is a later phase.
 */
export default defineEventHandler(async (event): Promise<ChatResponse> => {
  const body = await readBody<ChatRequest>(event)
  const message = body?.message?.trim()

  if (!message) {
    throw createError({ statusCode: 400, statusMessage: 'Message cannot be empty' })
  }

  const config = useRuntimeConfig()
  if (config.turnstileSecretKey) {
    const remoteIp = getRequestIP(event, { xForwardedFor: true }) ?? null
    const ok = await verifyCaptcha(body.captcha_token, remoteIp)
    if (!ok) {
      throw createError({ statusCode: 403, statusMessage: 'CAPTCHA verification failed' })
    }
  }

  return {
    answer:
      `Thanks for your message — the chat API is not wired up to the RAG ` +
      `pipeline yet. This is a placeholder response so the UI and widget can ` +
      `integrate against the real /api/chat contract. You asked: "${message}"`,
    sources: [],
    source_type: 'general_knowledge',
  }
})
