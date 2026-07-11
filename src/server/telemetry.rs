//! Logging/telemetry setup (SSR only).
//!
//! Initializes a [`tracing_subscriber`] that prints timestamped, level-tagged
//! events for both **API operations** (every HTTP/RPC request, via the
//! [`TraceLayer`](tower_http::trace::TraceLayer) added in `main`) and
//! **database operations** (SQLx emits a `tracing` event per query under the
//! `sqlx::query` target).
//!
//! Verbosity is controlled entirely by the `RUST_LOG` environment variable, so
//! logs can be tuned or silenced without code changes, e.g.:
//!
//! * `RUST_LOG=warn` — quiet: warnings and errors only.
//! * `RUST_LOG=info` — API requests + high-level app events (no SQL).
//! * `RUST_LOG=debug` — everything, including each SQL statement.
//! * `RUST_LOG="info,sqlx::query=debug"` — API events plus SQL only.
//!
//! When `RUST_LOG` is unset we default to a developer-friendly level that shows
//! API requests and database operations.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// The filter applied when `RUST_LOG` is not set: informative app + request
/// logs, plus SQLx per-query logs, while keeping noisy dependencies quiet.
const DEFAULT_FILTER: &str = "info,mommys_heart_crm=debug,tower_http=debug,sqlx::query=debug";

/// Install the global tracing subscriber. Safe to call once at startup; a second
/// call is ignored so tests and re-inits don't panic.
pub fn init() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));

    let _ = tracing_subscriber::registry()
        .with(filter)
        // Timestamps are included by default; target shows the emitting module.
        .with(fmt::layer().with_target(true))
        .try_init();
}
