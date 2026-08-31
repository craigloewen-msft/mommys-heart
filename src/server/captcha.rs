//! Cloudflare Turnstile CAPTCHA verification (SSR only).
//!
//! Returns true when the token is valid; the caller only invokes this when a
//! secret is configured.

const TURNSTILE_VERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

#[derive(serde::Deserialize)]
struct SiteVerifyResponse {
    success: bool,
}

pub async fn verify(secret: &str, token: Option<&str>, remote_ip: Option<&str>) -> bool {
    let Some(token) = token else {
        return false;
    };

    let mut form = vec![("secret", secret), ("response", token)];
    if let Some(ip) = remote_ip {
        form.push(("remoteip", ip));
    }

    match reqwest::Client::new()
        .post(TURNSTILE_VERIFY_URL)
        .form(&form)
        .send()
        .await
    {
        Ok(resp) => match resp.json::<SiteVerifyResponse>().await {
            Ok(body) => body.success,
            Err(err) => {
                eprintln!("CAPTCHA verification decode failed: {err}");
                false
            }
        },
        Err(err) => {
            eprintln!("CAPTCHA verification request failed: {err}");
            false
        }
    }
}
