# Guard outbound email to 100 sends per hour

## Plan

1. Add a small process-wide sliding-window guard at the low-level ACS `send_email` entry point so it covers both authentication and notification emails.
2. Store send reservations as monotonic timestamps in a Tokio mutex-protected `VecDeque`.
   - On each real send, remove timestamps at least one hour old.
   - When 100 timestamps remain, warn and wait until the oldest ages out.
   - Append the current timestamp before releasing the lock and beginning network I/O, making concurrent checks atomic.
   - Keep waiting callers in FIFO order so later emails cannot jump ahead.
3. Count one logical email once, before its first ACS attempt; internal retries do not reserve additional slots. Keep a reservation when delivery fails because an ambiguous request may still have reached ACS, and failing closed protects the provider quota.
4. Keep the guard in memory and intentionally simple: it resets when the process restarts and applies per running application instance, with no database changes or new dependencies.
5. Run formatting checks and the repository's isolated SSR compile check.

## Acceptance criteria

- No running application instance begins more than 100 logical ACS email sends in any rolling one-hour window.
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
- The guard prunes reservations at least one hour old, then warns and waits when 100 remain.
- Tokio's FIFO mutex ordering and retaining the lock during limiter waits prevent later emails from jumping ahead.
- Each logical email reserves one slot before its first ACS attempt; retries do not consume extra slots.
- Kept dry-run and unconfigured behavior unchanged and added no new crates or database changes.
- Simplified reservation flow to break when capacity exists and append once after the wait loop.
- Verified `cargo fmt --all -- --check` and `etc/dev.sh -- cargo check --no-default-features --features ssr` pass.
