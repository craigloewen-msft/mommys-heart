# Simplify etc/ dev scripts down to build / run / reset

## Problem

`etc/dev-db.sh` is 390 lines exposing 10 subcommands (`up`, `build`, `reset`,
`rebuild-seed`, `down`, `drop`, `info`, `list`, `psql`, `logs`) — and `build` is
really an app command, not a database command. Meanwhile `etc/dev-run.sh` is a
separate entry point that secretly calls `dev-db.sh up` first. So the two files
overlap, the naming lies (`dev-db.sh build` compiles Rust), and a newcomer has
to read a 20-line usage block to find the two commands they actually need.

## Goal

One script, `etc/dev.sh`, with three commands:

```bash
etc/dev.sh build     # containers up + cargo leptos build
etc/dev.sh run       # containers up + cargo leptos watch, prints MH_READY
etc/dev.sh reset     # wipe DB/storage back to fresh seed data
```

All the machinery that makes this repo multi-worktree friendly (per-checkout
slug, port block allocation, `.env.local` generation, the baked seed image)
stays exactly as-is — it is load-bearing and invisible to the user. This is a
**command-surface** change, not a behaviour change.

## What stays beyond build/run/reset (and why)

- **`etc/dev.sh -- <cmd...>`** — not a command, a passthrough flag. It must
  stay: it is the only way to run a one-off (`cargo check`, `seed`) with this
  instance's env *and* the isolated `target/oneshot` build dir, and AGENTS.md
  documents it. Without it, agents run bare `cargo check` and thrash the
  watcher's fingerprints.
- **`etc/dev.sh clean`** — I want to keep exactly one more, hidden at the bottom
  of the help. Deleting a worktree without it leaves an `mh-db-*` container,
  an `mh-storage-*` container and two volumes on the machine forever, and there
  is no other way to reclaim them. `reset` cannot cover this: reset must leave
  you with a *working* instance. If you'd rather have three commands flat, say
  so and I'll drop `clean` and document `wslc remove` in the README instead.

## What gets deleted

| Gone | Replacement |
|---|---|
| `up` | implicit in `build` and `run` (both already call it) |
| `down` / `stop` | Ctrl-C stops the app; containers are idle and cheap. `clean` removes them |
| `drop` | renamed to `clean` |
| `info` | `run` and `build` already print the summary block |
| `list` | `wslc list --all` — a one-liner, not our job |
| `psql` | print `DATABASE_URL` in the summary; use any client |
| `logs` | `wslc logs -f mh-db-<slug>`, named in the failure message we already emit |
| `rebuild-seed` | folded into `reset --rebuild-seed`; it is a rare escape hatch for a stale seed image, not a top-level verb |
| `seed` (alias of reset) | gone |

## Plan

1. Create `etc/dev.sh` from the existing two scripts: keep every helper function
   (identity, port allocation, seed image build/bake, `wait_for_db`,
   `write_env_local`, readiness polling, `MH_READY`/`MH_FAILED` contract)
   verbatim; replace only the `case` dispatch and the header comment.
2. Split the file with clear section banners so the *policy* (the ~40 lines of
   command dispatch) sits at the top and the *plumbing* below it, so the
   simplicity is visible on opening the file.
3. Keep the `run` guard that refuses to start unbuilt
   (`dev: not built yet — run 'etc/dev.sh build' first`) and the MH_READY line;
   IDE/agent harnesses depend on both.
4. Delete `etc/dev-db.sh` and `etc/dev-run.sh`.
5. Update references: `AGENTS.md`, `README.md`, `.env.example` (2 spots),
   `src/server/db/seed.rs` doc comment, `.gitignore` comment.
6. Verify by hand: `etc/dev.sh clean && etc/dev.sh build && etc/dev.sh run`
   (wait for `MH_READY`, curl the port), then `etc/dev.sh reset`, then
   `etc/dev.sh -- cargo check --no-default-features --features ssr`.

## Open question

Name: `etc/dev.sh` is my preference (`etc/dev.sh run` reads well). Alternatives
are keeping `dev-run.sh` as the single file, or a top-level `./dev`. Tell me if
you want a different one before I start.
