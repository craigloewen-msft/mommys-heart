/**
 * Verify a Cloudflare Turnstile token server-side.
 *
 * Ported from the legacy Python `_verify_captcha`. Returns true when CAPTCHA is
 * disabled (no secret configured) or the token is valid; false otherwise.
 */
const TURNSTILE_VERIFY_URL = 'https://challenges.cloudflare.com/turnstile/v0/siteverify'

export async function verifyCaptcha(
  token: string | null | undefined,
  remoteIp: string | null | undefined,
): Promise<boolean> {
  const secret = useRuntimeConfig().turnstileSecretKey
  if (!secret) return true // CAPTCHA disabled
  if (!token) return false

  const body = new URLSearchParams({ secret, response: token })
  if (remoteIp) body.set('remoteip', remoteIp)

  try {
    const result = await $fetch<{ success: boolean }>(TURNSTILE_VERIFY_URL, {
      method: 'POST',
      body,
    })
    return Boolean(result?.success)
  } catch (error) {
    console.error('CAPTCHA verification request failed:', error)
    return false
  }
}
