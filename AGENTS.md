# Working in this repo

## Running the app

Every checkout gets its own database and storage containers on its own ports, so
several agents can run the app at the same time without interfering. You do not
need to pick ports or configure anything:

```bash
etc/dev-db.sh up      # start this checkout's containers (already seeded)
etc/dev-db.sh build   # compile the app (slow the first time; do this once)
etc/dev-run.sh        # cargo leptos watch, on this checkout's port
```

`etc/dev-run.sh` prints the URL it is serving on, and calls `dev-db.sh up` for
you so stopped containers come back automatically.

**`dev-run.sh` does not build.** If the project has not been built it exits
immediately with `dev-run: not built yet — run 'etc/dev-db.sh build' first`.
Building is a separate step on purpose: a cold `cargo leptos` build takes many
minutes, and hiding that inside the run command makes startup unbounded.

### Readiness signal (for IDEs and automated harnesses)

Do not wait on an application log line. `etc/dev-run.sh` waits until the site
port genuinely accepts connections and then prints one line:

```
MH_READY listening on http://127.0.0.1:<port>
```

If the server dies during startup it prints `MH_FAILED ...` and exits non-zero.
Wait for `MH_READY`; treat `MH_FAILED` or process exit as immediate failure.
Run `etc/dev-db.sh build` to completion first, then a ~120s timeout is plenty.

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
