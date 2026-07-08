// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2025-01-01',
  devtools: { enabled: true },

  modules: ['@nuxt/ui', '@nuxt/eslint'],

  // App-wide metadata and default document head.
  app: {
    head: {
      title: "Mommy's Heart CRM",
      meta: [
        { charset: 'utf-8' },
        { name: 'viewport', content: 'width=device-width, initial-scale=1' },
      ],
    },
  },

  // Runtime configuration. Values are read from environment variables at
  // runtime (NUXT_* overrides the nested keys). `public` is exposed to the
  // browser; everything else stays server-only (never sent to the client).
  runtimeConfig: {
    // --- server-only secrets (ported from the legacy Python config) ---
    azureOpenaiApiKey: '', // NUXT_AZURE_OPENAI_API_KEY
    azureOpenaiEndpoint: '', // NUXT_AZURE_OPENAI_ENDPOINT
    azureOpenaiChatDeployment: 'gpt-4o', // NUXT_AZURE_OPENAI_CHAT_DEPLOYMENT
    azureOpenaiEmbeddingDeployment: 'text-embedding-ada-002', // NUXT_AZURE_OPENAI_EMBEDDING_DEPLOYMENT
    azureOpenaiApiVersion: '2024-12-01-preview', // NUXT_AZURE_OPENAI_API_VERSION
    turnstileSecretKey: '', // NUXT_TURNSTILE_SECRET_KEY
    // Comma-separated origins allowed to call the API cross-origin (the
    // Squarespace site that embeds the chat widget). "*" allows any origin.
    allowedOrigins: '*', // NUXT_ALLOWED_ORIGINS

    public: {
      // Exposed to the browser. Bump on meaningful releases; surfaced by
      // GET /api/version.
      appVersion: '2.0.0-dev',
    },
  },

  typescript: {
    strict: true,
  },
})
