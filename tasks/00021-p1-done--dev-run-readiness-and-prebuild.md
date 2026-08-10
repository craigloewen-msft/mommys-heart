# Make `etc/dev-run.sh` signal readiness explicitly and fail fast instead of hanging

## Problem

An IDE/agent harness starts the app as a long-lived process and waits for a
readiness string:

```json
{ "cmd": "etc/dev-run.sh", "name": "dev7",
  "readiness": { "mode": "wait_for_text", "text": "listening on", "timeout_seconds": 900 } }
```

This *always* runs out the full 900s. `"listening on"` is emitted once, at
`src/main.rs:98`, by the app binary — the last thing before `TcpListener::bind`.
Three things stop it arriving:

1. **Cold build inside the readiness window.** A fresh worktree has no
   `target/`. `etc/dev-db.sh` may run the Rust seeder (`cargo run --features
   ssr -- seed` into `target/oneshot`), and then `cargo leptos watch` builds the
   ssr bin *and* the hydrate wasm lib *and* tailwind into `target/`. That is the
   whole dependency graph compiled twice, which routinely exceeds 900s. Since
   every agent worktree starts cold, it fails every time.
2. **Silent death before the line is printed.** `dev-run.sh:16` only calls
   `dev-db.sh up` when `.env.local` is *missing*. If `.env.local` exists but the
   containers are stopped/removed (or `wslc.exe` is unreachable), `main.rs:52`
   / `:58` `panic!` on db/storage init. The process exits, the readiness text
   never appears, and the watcher waits out the timeout on a dead process.
3. **No accurate early signal.** The only line `dev-run.sh` prints itself
   (`Starting '$MH_INSTANCE' on http://...`, line 30) is emitted *before* the
   build, so matching it would be a false ready.

## Decisions (from the user)

- Expose a **dedicated `MH_READY` token** rather than relying on the app's log
  format.
- **Split build from run.** Building is a separate, explicit command. `run` does
  **not** build — if the binary/site assets are missing it must fail immediately
  with a clear message telling the caller to build first. Update `AGENTS.md`
  accordingly.

## Plan

### 1. `etc/dev-db.sh` — add a `build` subcommand

Add `build` to the `case` dispatch and the usage header:

- Runs `dev-db.sh up` (containers + `.env.local`), then, with `.env.local`
  sourced, runs `cargo leptos build` in `target/` (the same target dir `watch`
  uses, so the artifacts are directly reusable — do **not** point it at
  `target/oneshot`).
- This is where all the cold-build cost lives. No timeout pressure: the harness
  runs it as an ordinary command, not under a readiness wait.
- Print a clear final line, e.g. `Build complete for '<slug>'.`

### 2. `etc/dev-run.sh` — three changes

**(a) Always ensure the instance is up.** Replace the conditional on line 16
with an unconditional `"$REPO_ROOT/etc/dev-db.sh" up` (it is idempotent and
cheap when containers already run). This removes failure mode 2 — a stale
`.env.local` can no longer produce a panic-then-hang.

**(b) Refuse to run unbuilt.** Before launching `cargo leptos watch`/`serve`,
check for the built artifacts (the ssr binary under `target/{debug,release}/`
and `target/site/pkg/`). If absent:

```
dev-run: not built yet — run 'etc/dev-db.sh build' first
```

and `exit 1` immediately. Never silently start a 20-minute compile inside
someone's readiness window.

**(c) Emit `MH_READY` from the wrapper, and `MH_FAILED` on early exit.** Run
`cargo leptos ...` as a child (not `exec`), and concurrently poll
`$LEPTOS_SITE_ADDR` until the TCP port accepts a connection. Then print exactly
one line to stdout:

```
MH_READY listening on http://127.0.0.1:<port>
```

The token must be a real check (port open), not a log-scrape, so it cannot
false-positive. If the child exits before the port opens, print
`MH_FAILED dev-run: server exited before becoming ready (status N)` and exit
with the child's status. Forward SIGINT/SIGTERM to the child and keep streaming
the child's stdout/stderr through unchanged so normal logs are still visible.

Keep the app's own `listening on` line untouched — `MH_READY` is additive.

### 3. `AGENTS.md`

Update the "Running the app" section:

- New order: `etc/dev-db.sh up` → `etc/dev-db.sh build` → `etc/dev-run.sh`.
- State plainly that `dev-run.sh` no longer builds and will exit non-zero if the
  project is not built, and why (so readiness waits are bounded).
- Document the `MH_READY` contract for harnesses: wait for the literal text
  `MH_READY` on stdout; a `MH_FAILED` line or process exit means give up now.
  Recommend a modest timeout (~120s) once `build` has been run separately.
- Mention `etc/dev-db.sh build` in the `--help`/usage list too.

### 4. IDE/harness config to use afterwards

```json
{ "cmd": "etc/dev-db.sh build" }                       // run to completion first
{ "cmd": "etc/dev-run.sh", "name": "dev7",
  "readiness": { "mode": "wait_for_text", "text": "MH_READY", "timeout_seconds": 120 } }
```

## Verification

1. Fresh worktree, no `target/`: `etc/dev-run.sh` exits non-zero within a second
   with the "not built yet" message.
2. `etc/dev-db.sh build` completes; then `etc/dev-run.sh` prints `MH_READY ...`
   and the printed URL actually serves a page.
3. Stop the containers (`etc/dev-db.sh down`) but leave `.env.local`, then run
   `etc/dev-run.sh`: it brings containers back up and still reaches `MH_READY`.
4. Force a startup failure (e.g. bogus `DATABASE_URL`): `MH_FAILED` is printed
   and the process exits promptly rather than hanging.
5. Ctrl-C cleanly terminates the child `cargo leptos` process (no orphans).

No `cargo test` work — per repo convention, remove any temporary test scaffolding
before review.
