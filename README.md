# Mommy's Heart management website

A Rust + Leptos (Axum SSR + WASM hydrate) CRM. The CRM domain (users, cases,
evidence, grants, messages, audit logs) is persisted in **PostgreSQL** via SQLx,
with server-side authentication (argon2 password hashing + session cookies).

## Local development

1. **Start the backing services** (Postgres + Azurite, both via `wslc.exe`, the
   WSL container CLI). One script manages both containers — the CRM database and
   the [Azurite](https://learn.microsoft.com/azure/storage/common/storage-use-azurite)
   Azure Storage emulator that holds case evidence *files*:

   ```
   etc/dev-db.sh up            # create + start both containers (idempotent)
   etc/dev-db.sh down          # stop + remove both containers (keeps data)
   etc/dev-db.sh reset         # remove both containers AND their data volumes
   etc/dev-db.sh seed          # (re)populate the database with demo/test data
   etc/dev-db.sh logs [db|storage]   # tail a container's logs (default: db)
   etc/dev-db.sh psql          # open a psql shell inside the database container
   ```

   The `evidence` blob container is created automatically on app startup. Only
   the metadata (filename, type, size, SHA-256, blob path) is stored in Postgres;
   the bytes live in Blob Storage.

2. **Configure environment.** Copy `.env.example` to `.env` and adjust as
   needed. The key variables for the database are `DATABASE_URL`,
   `SESSION_SECRET`, and `COOKIE_SECURE`. For evidence storage, the local
   `AZURE_STORAGE_CONNECTION_STRING` (Azurite's well-known dev key, already filled
   in) and `AZURE_STORAGE_CONTAINER` are all that's needed. Logging verbosity is
   controlled by `RUST_LOG` (see **Logging** below).

3. **Run the app.** On startup the server applies the migrations in
   `migrations/` and, if the database is empty, seeds it from the demo fixtures.

   ```
   cargo leptos watch     # dev server with hot reload at http://127.0.0.1:3000
   ```

   Demo accounts (seeded on an empty database):

   | Role      | Email                     | Password       |
   | --------- | ------------------------- | -------------- |
   | Admin     | admin@mommysheart.org     | admin123       |
   | Volunteer | dana@mommysheart.org      | volunteer123   |
   | Client    | jamie@example.com         | client123      |

   To reload the demo/test data at any time (for example after experimenting in
   the UI), run `etc/dev-db.sh seed`. It wipes the existing CRM rows and re-inserts
   the fixtures using the app's own seeder, so the demo passwords stay valid. This
   is a development convenience only — remove or ignore it before going live.

## Logging

The server uses [`tracing`](https://docs.rs/tracing) for structured, timestamped
logs covering both **API requests** (method, path, status, latency) and
**database operations** (one line per SQL query). Verbosity is controlled
entirely by the standard `RUST_LOG` environment variable, so logs can be tuned or
silenced without code changes:

```
RUST_LOG=warn                       # warnings + errors only
RUST_LOG=info                       # app events + API requests (no SQL)
RUST_LOG=debug                      # everything, including each SQL statement
RUST_LOG="info,sqlx::query=debug"   # API events plus SQL only
```

When `RUST_LOG` is unset, a developer-friendly default shows API requests and
database operations.

## Container commands

```
wslc.exe build -t craigsdevcontainers.azurecr.io/mommys-heart:latest .

wslc.exe push craigsdevcontainers.azurecr.io/mommys-heart:latest
```

In production, set `DATABASE_URL` to your Azure Database for PostgreSQL
connection string and `COOKIE_SECURE=true` (cookies require HTTPS).
