# Stop agent cargo commands from stalling `cargo leptos watch`

## Problem

While an agent works, `cargo leptos watch` appears to stop picking up edits.

The watcher is fine; the shared `target/` directory is the bottleneck. There is
no `CARGO_TARGET_DIR` or `.cargo/config.toml` in the repo, so every cargo
invocation shares one build directory:

1. **Exclusive lock.** `cargo check --no-default-features --features ssr`,
   `etc/dev-run.sh -- cargo run ... seed`, and `preview-emails` all take cargo's
   exclusive lock on `target/`. The watcher detects the edit, then blocks on
   "waiting for file lock on build directory" with no visible progress — for as
   long as the agent's build runs.
2. **Fingerprint thrash.** The agent's ssr-only profile differs from what
   cargo-leptos builds (ssr bin + hydrate wasm lib), so the two invalidate each
   other's artifacts. The next watch rebuild becomes a full rebuild.
3. `cargo run -- preview-emails` writes into `target/email-preview/`, adding
   churn under the watched tree.

## Plan

Give non-watch cargo work its own build directory. `cargo leptos watch` keeps
`target/` to itself.

1. **`etc/dev-run.sh`** — in the `--` passthrough branch, export
   `CARGO_TARGET_DIR="$REPO_ROOT/target/oneshot"` unless the caller already set
   `CARGO_TARGET_DIR`. Leave the `watch`/`serve` path untouched so cargo-leptos
   still uses `target/` (its `site-root = target/site` depends on it).
2. **`etc/dev-db.sh`** — same treatment for the `cargo run -- seed` call in
   `build_seed_dump`, so seeding never blocks a running watch.
3. **`AGENTS.md` / `README.md`** — document the rule: any cargo command run
   alongside a live `etc/dev-run.sh` must use the separate target dir. Update the
   quick-check line to the wrapper form, e.g.
   `etc/dev-run.sh -- cargo check --no-default-features --features ssr`, and add
   one sentence explaining why (lock + fingerprint thrash).
4. Optionally add `target/oneshot` housekeeping notes — it is inside `/target`,
   which is already gitignored, so no gitignore change is needed.

## Trade-off to measure (BEFORE and AFTER the change)

The isolated dir costs one cold build and roughly doubles dependency-artifact
disk usage, in exchange for never serializing against the watcher. Do not take
that on faith — measure it, report real numbers, and only then confirm the
change is worth keeping.

### Protocol

Same machine, no other agents running, containers already up. Time each command
with `/usr/bin/time -f '%e s'`, repeat 3x, report raw samples (not just a mean).
Disk via `du -sh`.

**Phase A — baseline, before any code change**

1. `cargo clean`, then time a cold `cargo check --no-default-features --features ssr`.
2. Time a warm re-run of the same command (no edits).
3. Touch one `.rs` file (e.g. `src/app.rs`), time the incremental `cargo check`.
4. Start `etc/dev-run.sh`, wait for first serve. With watch idle, touch a `.rs`
   file and record wall time from save to the rebuild/reload line in the log.
5. Repeat 4, but launch `cargo check --no-default-features --features ssr` first
   and save the edit while it runs — record how long the watch rebuild is
   delayed and capture the file-lock message. This is the bug being fixed.
6. `du -sh target`.

**Phase B — after the change**

Clean everything first (`cargo clean`, remove `target/oneshot`), then repeat
steps 1–6 verbatim with agent commands going through
`etc/dev-run.sh -- cargo check ...`. Additionally record:

- cold build time of `target/oneshot` (the new one-time cost),
- warm and incremental check times in the isolated dir vs Phase A,
- `du -sh target target/oneshot`,
- whether the watcher still does a *full* rebuild after an agent check
  (i.e. is the fingerprint thrash actually gone?).

### Report

Append a results table to this task file before handing back: cold build, warm
check, incremental check, watch reload latency (idle), watch reload latency
(during agent check), disk usage — baseline vs after. Call out the one-time cost
and the steady-state delta explicitly. If the steady-state numbers do not
justify the change, say so rather than shipping it.

## Verification

- Start `etc/dev-run.sh`; while it is running, run
  `etc/dev-run.sh -- cargo check --no-default-features --features ssr`.
- Edit a `.rs` file mid-check; the watcher must rebuild immediately and not
  print the file-lock message.
- Confirm `etc/dev-db.sh reset` works with a watch session live.
