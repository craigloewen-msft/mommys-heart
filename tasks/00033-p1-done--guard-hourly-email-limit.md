# Guard standard outbound email to 90 sends per hour

## Plan

1. Add a small process-wide sliding-window guard at the low-level ACS `send_email` entry point so it covers both authentication and notification emails.
2. Store ID-bearing send reservations as monotonic timestamps in a Tokio mutex-protected `VecDeque`.
   - Return the reservation ID to the transport and mark it complete after success or error.
   - Remove completed reservations at least one hour old; never age out an in-flight send.
   - When 90 timestamps remain, warn and wait until the oldest ages out.
   - Append the current timestamp before releasing the lock and beginning network I/O, making concurrent checks atomic.
   - Keep waiting callers in FIFO order so later emails cannot jump ahead.
   - Record MFA and email-verification OTP codes in the same history but let them bypass waiting.
3. Count one logical email once, before its first ACS attempt; internal retries do not reserve additional slots. Keep a reservation when delivery fails because an ambiguous request may still have reached ACS, and failing closed protects the provider quota.
4. Keep the guard in memory and intentionally simple: it resets when the process restarts and applies per running application instance, with no database changes or new dependencies.
5. Run formatting checks and the repository's isolated SSR compile check.

## Acceptance criteria

- No running application instance begins more than 90 standard ACS email sends in any rolling one-hour window.
- MFA and email-verification OTP emails bypass the queue and are attempted immediately.
- OTP attempts are recorded in the same history used to limit standard emails.
- In-flight sends remain reserved even when they run longer than one hour.
- Password-reset and regular notification emails remain subject to the standard limit.
- The oldest reservation automatically ages out after one hour, allowing another send.
- Concurrent callers cannot both claim the same remaining slot.
- Rate-limited sends log a warning, wait for the next slot, and then continue to ACS.
- Waiting sends reserve newly available slots in FIFO order.
- Dry-run and unconfigured email paths remain unchanged because they do not invoke the transport.
- Existing ACS retries count as part of their original logical email rather than as new emails.
- No database migration or new crate is introduced.
- The SSR compile check passes.

## Completion

- Added a process-local, Tokio mutex-protected rolling queue at the shared ACS transport boundary.
- The guard prunes completed reservations at least one hour old, but keeps in-flight reservations until success or error.
- MFA and email-verification OTP messages bypass waiting but are recorded; password-reset and notification messages wait when the combined history reaches 90.
- Each reservation has an ID that the transport marks complete after its final ACS result.
- Standard sends remain FIFO while the history mutex is released during waits, so OTP sends are never blocked behind them.
- Each logical email reserves one slot before its first ACS attempt; retries do not consume extra slots.
- Kept dry-run and unconfigured behavior unchanged and added no new crates or database changes.
- Waiting standard sends wake when an in-flight request completes and recheck the shared history before reserving capacity.
- Verified the guard with five temporary local-only checks: standard history, OTP bypass/history, OTP priority, window expiry, and a complete send to a loopback fake ACS endpoint. No external service or mailbox was contacted; the temporary tests were removed afterward.
- Verified `cargo fmt --all -- --check` and `etc/dev.sh -- cargo check --no-default-features --features ssr` pass.
