# Working in this repo

## Running the app

Every checkout gets its own database and storage containers on its own ports, so
several agents can run the app at the same time without interfering. You do not
need to pick ports or configure anything:

```bash
etc/dev-db.sh up      # start this checkout's containers (already seeded)
etc/dev-run.sh        # cargo leptos watch, on this checkout's port
```

`etc/dev-run.sh` prints the URL it is serving on. It calls `dev-db.sh up` for you
if the containers are not running yet.

Run one-off commands against the same instance with `--`. These automatically
get their own build directory (`target/oneshot`), so they never disturb a
running `cargo leptos watch`:

```bash
etc/dev-run.sh -- cargo run --no-default-features --features ssr -- seed
```

For a quick compile check, prefer the wrapper so it uses that separate build
directory too; no containers are needed for compiling:

```bash
etc/dev-run.sh -- cargo check --no-default-features --features ssr
```

Running a bare `cargo check` in the repo root still works, but it shares
`target/` with the watcher. Because the two build different feature sets
(ssr binary vs hydrate wasm lib) they invalidate each other's fingerprints, so
alternating between them makes both noticeably slower.

**If `cargo leptos watch` stops picking up edits, check `.gitignore` first.**
cargo-leptos builds its watch list from `.gitignore`, so any pattern that
matches your checkout's own source files makes the watcher go silently deaf.
This is why the `.phoenix` entry is anchored as `/.phoenix` — unanchored, it
matched agent worktrees living under `.phoenix/worktrees/`, and edits inside
them never triggered a rebuild.

You can run `./etc/dev-db.sh --help` to see additional dev database commands if needed.

## Cargo tests

This repo does not make use of any `cargo test` functionality so do not write any. If you need any to test your own code then feel free to write it, use it for temporary testing, but then remove it when it is time for the user to review and you are done your task.

## Coding convention

Make sure your comments and concise and short and usually max 1 or 2 sentences.

Top level database 'items' or 'objects' are represented in the coding structure. You can see them under 'src/componenents/server/db/' and they have similar structures under 'server_fns' to expose access to them.
