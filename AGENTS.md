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

Run one-off commands against the same instance with `--`:

```bash
etc/dev-run.sh -- cargo run --no-default-features --features ssr -- seed
```

Use `cargo check --no-default-features --features ssr` for a quick compile check;
no containers are needed for that.

You can run `./etc/dev-db.sh --help` to see additional dev database commands if needed.

## Cargo tests

This repo does not make use of any `cargo test` functionality so do not write any. If you need any to test your own code then feel free to write it, use it for temporary testing, but then remove it when it is time for the user to review and you are done your task.

## Coding convention

Make sure your comments and concise and short and usually max 1 or 2 sentences.

Top level database 'items' or 'objects' are represented in the coding structure. You can see them under 'src/componenents/server/db/' and they have similar structures under 'server_fns' to expose access to them.
