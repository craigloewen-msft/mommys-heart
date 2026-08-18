# Guard outbound email to 100 sends per hour

## Plan

1. Add a small process-wide sliding-window guard at the low-level ACS `send_email` entry point so it covers both authentication and notification emails.
2. Store send reservations as monotonic timestamps in a mutex-protected `VecDeque`.
   - On each real send, remove timestamps at least one hour old.
   - Return a clear error when 100 timestamps remain.
   - Otherwise append the current timestamp before releasing the lock and beginning network I/O, making concurrent checks atomic.
3. Count one logical email once, before its first ACS attempt; internal retries do not reserve additional slots. Keep a reservation when delivery fails because an ambiguous request may still have reached ACS, and failing closed protects the provider quota.
4. Keep the guard in memory and intentionally simple: it resets when the process restarts and applies per running application instance, with no database changes or new dependencies.
5. Run formatting checks and the repository's isolated SSR compile check.

## Acceptance criteria

- No running application instance begins more than 100 logical ACS email sends in any rolling one-hour window.
- The oldest reservation automatically ages out after one hour, allowing another send.
- Concurrent callers cannot both claim the same remaining slot.
- Rejected sends return an error through the existing email failure handling and do not call ACS.
- Dry-run and unconfigured email paths remain unchanged because they do not invoke the transport.
- Existing ACS retries count as part of their original logical email rather than as new emails.
- No database migration or new crate is introduced.
- The SSR compile check passes.
