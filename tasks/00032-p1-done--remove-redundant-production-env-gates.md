# Remove redundant production environment gates

## Findings

The production deployment currently fails during `validate_production()` when either `COOKIE_SECURE=true` or `MALWARE_SCAN_ADDR` is absent.

- **Secure cookies are needed, but `COOKIE_SECURE` is not.** All authentication cookies must carry the `Secure` attribute in production. The application already has one production-mode decision (`config::is_production()`) that controls HSTS, MFA bypass prevention, and origin enforcement. Requiring a second variable for cookies duplicates that decision, permits contradictory configuration, and is the source of the deployment failure.
- **`MALWARE_SCAN_ADDR` is not needed by the currently deployed feature set.** The variable configures a ClamAV-compatible scanner for evidence uploads, but evidence is intentionally disabled in the UI and all direct upload, download, mutation, and folder endpoints reject requests before accessing evidence. Requiring a scanner merely to start unrelated application features is therefore unnecessary.
- The dormant malware scanner should remain fail-closed. If evidence is re-enabled later without a scanner, uploads must fail rather than accept unscanned files. Removing the current startup gate does not require deleting that preserved implementation.
- The other production gates—configured ACS email for MFA and `APP_URL` for same-origin mutation checks—protect active behavior and remain required.

## Plan

1. Base the implementation on the latest `main`, where the production validation and disabled evidence state exist.
2. Remove `COOKIE_SECURE` as an application setting.
   - Derive every authentication cookie's `Secure` attribute from `config::is_production()`.
   - Remove the redundant `COOKIE_SECURE` startup validation and environment-template entry.
   - Keep local HTTP development cookies non-secure and production cookies secure automatically.
3. Remove `MALWARE_SCAN_ADDR` from unconditional production startup validation.
   - Keep the disabled evidence boundaries unchanged.
   - Keep the dormant scanner's production behavior fail-closed so it cannot silently accept unscanned evidence if called.
   - Update the environment template and production-readiness documentation so the scanner is not presented as a requirement for deployments while evidence is disabled.
4. Verify that no production startup path still requires `COOKIE_SECURE` or an evidence scanner, and that documentation matches runtime behavior.
5. Run the SSR compile check through `etc/dev.sh -- cargo check --no-default-features --features ssr`. If practical, build/run the application and confirm production validation proceeds without those two variables while retaining the active ACS email and `APP_URL` checks.

## Acceptance criteria

- A release/production process does not fail startup because `COOKIE_SECURE` is absent.
- Authentication cookies are always marked `Secure` in production and remain usable over local development HTTP.
- A production process does not fail startup because `MALWARE_SCAN_ADDR` is absent while evidence is disabled.
- Disabled evidence UI and server-side rejection boundaries remain intact.
- The preserved evidence scanner still fails closed in production when invoked without configuration.
- ACS email and `APP_URL` production safety checks remain intact.
- Environment and operations documentation accurately describe the resulting requirements.
- The SSR compile check passes.

## Completion

- Removed `COOKIE_SECURE`; authentication cookies now derive `Secure` from the shared production-mode decision.
- Removed the unconditional production startup requirement for `MALWARE_SCAN_ADDR` while retaining the dormant scanner's fail-closed production behavior.
- Kept the active ACS email and `APP_URL` production gates unchanged and updated deployment documentation.
- Verified `cargo fmt --check` and the isolated SSR compile check pass.
- Verified production validation succeeds with both removed variables unset, production cookies include `Secure`, local development cookies omit it, the unconfigured production scanner returns an error, and the retained production gates still reject incomplete configuration.
- `etc/dev.sh build` was also attempted, but this environment does not have the `cargo-leptos` subcommand installed; it exited before compiling project code.
