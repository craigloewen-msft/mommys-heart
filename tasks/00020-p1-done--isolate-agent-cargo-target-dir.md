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

---

# RESULTS

## The diagnosis in the plan was wrong (partly)

Measuring first paid off. The build-lock theory was **not** the cause, and it is
not even expensive. The real cause was found by instrumenting the watcher.

**Root cause: `.gitignore` blinded the watcher.** cargo-leptos builds its watch
list from the project's `.gitignore` (`src/service/notify.rs`: `Gitignore::new`,
then `ignore_paths()` drops any watched path whose file *or any parent directory*
matches). The entry was the unanchored pattern `.phoenix`, which git matches at
**every depth**. Agent worktrees live at `.phoenix/worktrees/<id>/`, so from
inside such a worktree every source file had an ignored ancestor and cargo-leptos
registered **zero** watch paths. Edits produced no rebuild at all — not a slow
one, not a delayed one. Silence.

This is exactly the reported symptom ("stops picking up edits") and it correlates
with "when the agent is running" only because agents *are* the ones working
inside `.phoenix/worktrees/`.

### Evidence

| Condition | Edit → rebuild |
| --- | --- |
| `.gitignore` contains unanchored `.phoenix` (baseline) | **never** (timed out at 610 s, 4 trials) |
| `.phoenix` line removed | 8 s |
| anchored as `/.phoenix` (the fix) | **7.52 / 7.12 / 7.13 s** |

Note also that `touch` alone never triggers cargo-leptos; it compares content, so
the measurement harness had to append real text.

## Performance measurements

16 cores. Each figure is a raw sample, 3 runs.

### Agent `cargo check` (`--no-default-features --features ssr`)

| Scenario | Shared `target/` (baseline) | Isolated `target/oneshot` |
| --- | --- | --- |
| Cold build | 135.00 s | 63.80 s (one-time) |
| Warm, no edits | 5.42 / 1.75 / 1.38 s | **0.26 / 0.25 / 0.26 s** |
| Incremental (1 file edited) | 18.68 / 18.24 / 13.18 s | **2.35 / 2.78 / 3.09 s** |

### Watch reload latency (with the `.gitignore` fix applied)

| Scenario | Time |
| --- | --- |
| Idle watcher | 7.52 / 7.12 / 7.13 s |
| During agent check, shared `target/` | 10.02 / 8.93 / 8.73 s |
| During agent check, isolated `target/oneshot` | 8.74 / 7.53 / 9.66 s |

### Disk

| | Size |
| --- | --- |
| `target/` baseline (after watch + checks) | 1.2 G → grows to 6.5 G with watch artifacts (`debug` 4.7 G + `front` 2.0 G) |
| `target/oneshot` (new) | **1.2 G** |
| Total after change | 7.7 G |

## Verdict on the trade-off

**One-time cost:** one 63.8 s cold build, +1.2 GB disk (~18 % on top of a 6.5 GB
`target/`). Not the "roughly doubles" the plan assumed — the wasm `front` and
`debug` trees dominate and are not duplicated.

**Steady-state gain:** warm checks 1.38–5.42 s → 0.26 s (**5–20× faster**);
incremental checks 13–19 s → 2.4–3.1 s (**~6× faster**). That gain is the
fingerprint thrash disappearing, and it repeats on every single agent check.
The cold build pays for itself after roughly four incremental checks.

**On lock contention specifically:** it is real but minor — reload latency went
7.1–7.5 s idle → 8.7–10.0 s during a concurrent check, about +1.5 s. No
"Blocking waiting for file lock on build directory" message ever appeared. So
the isolated target dir is justified by the *fingerprint thrash*, not by locking.

Both changes are worth keeping, but they are independent: the `.gitignore`
anchor is the actual bug fix; the target-dir split is a solid speed win.

## Changes made

1. `.gitignore` — `.phoenix` → `/.phoenix`, with a comment explaining why the
   anchor is load-bearing. **This is the fix for the reported bug.**
2. `etc/dev-run.sh` — `--` passthrough exports
   `CARGO_TARGET_DIR=$REPO_ROOT/target/oneshot` (caller can override).
3. `etc/dev-db.sh` — the seeder's `cargo run` uses the same isolated dir.
4. `AGENTS.md` — documents the wrapper for compile checks and, prominently, the
   "if watch goes deaf, check `.gitignore` first" rule.

## Verification performed

- Fresh `etc/dev-run.sh` reaches "Serving at" and an edit rebuilds in ~7 s.
- Edit saved *while* `etc/dev-run.sh -- cargo check` runs: watcher rebuilt in
  21 s (includes the check competing for all 16 cores), check exited 0, and the
  watch log contained **zero** build-directory lock messages.
- `etc/dev-run.sh -- bash -c 'echo $CARGO_TARGET_DIR'` prints `.../target/oneshot`.
- Working tree left clean of all measurement probes.
