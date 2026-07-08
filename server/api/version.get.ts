import type { VersionResponse } from '~/shared/types/api'

// Report the deployed build + model config (parity with the legacy /version).
// Never exposes secrets — only whether CAPTCHA enforcement is active.
export default defineEventHandler((): VersionResponse => {
  const config = useRuntimeConfig()
  return {
    version: config.public.appVersion,
    chat_model: config.azureOpenaiChatDeployment,
    embedding_model: config.azureOpenaiEmbeddingDeployment,
    captcha_enabled: Boolean(config.turnstileSecretKey),
  }
})
