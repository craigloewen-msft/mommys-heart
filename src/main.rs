#![recursion_limit = "256"]

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{middleware, Router};
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use mommys_heart_app::app::{shell, App};
    use mommys_heart_app::server::{api, rag, telemetry};
    use mommys_heart_app::server_fns;
    use tower_http::trace::TraceLayer;

    let _ = dotenvy::from_filename(".env.local");
    let _ = dotenvy::dotenv();

    // Start structured logging first so startup (incl. DB migrations/seeding) is
    // captured. Verbosity is controlled by `RUST_LOG`.
    telemetry::init();

    // REQ-SEC-001/004/005: production never starts with MFA, secure cookies,
    // origin validation, or malware quarantine accidentally disabled.
    if let Err(error) = mommys_heart_app::server::config::validate_production() {
        panic!("unsafe production configuration: {error}");
    }

    if std::env::args().nth(1).as_deref() == Some("seed") {
        if let Err(e) = mommys_heart_app::server::db::init().await {
            panic!("failed to initialize database: {e}");
        }
        if let Err(e) = mommys_heart_app::server::db::seed::reseed().await {
            panic!("failed to seed database: {e}");
        }
        tracing::info!("demo data loaded");
        return;
    }

    // `mommys-heart-app preview-emails [out-dir]` — render every notification
    // email template with placeholder data to standalone HTML files (default
    // `target/email-preview/`) and exit, without a database, email service, or
    // web server. Open the printed `index.html` in a browser to iterate on the
    // look. See `EMAIL_DRY_RUN` in .env.example for the runtime dry-run flag.
    if std::env::args().nth(1).as_deref() == Some("preview-emails") {
        use std::path::PathBuf;
        let out_dir = std::env::args()
            .nth(2)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target/email-preview"));
        match mommys_heart_app::server::email::preview::write_gallery(&out_dir) {
            Ok(index) => tracing::info!("email preview written — open {}", index.display()),
            Err(e) => panic!("failed to write email preview: {e}"),
        }
        return;
    }

    // Connect to PostgreSQL, run migrations, and seed on first run. Fail fast if
    // the database is unreachable — the app cannot function without it.
    if let Err(e) = mommys_heart_app::server::db::init().await {
        panic!("failed to initialize database: {e}");
    }

    // Configure Azure Blob Storage for evidence uploads. Missing configuration
    // is not fatal — uploads degrade gracefully like the RAG pipeline does.
    if let Err(e) = mommys_heart_app::server::storage::init().await {
        panic!("failed to initialize evidence storage: {e}");
    }

    // Kick off document ingestion in the background so the server starts
    // serving immediately; the RAG store fills in once embeddings complete.
    rag::start_background_ingest();

    // Prune audit-log entries older than the retention window, on startup and
    // then every 30 days.
    mommys_heart_app::server::db::audit::start_retention_task();

    // Same for the durable email-delivery-failure log surfaced in the admin
    // dashboard.
    mommys_heart_app::server::db::email_failures::start_retention_task();

    // Prune case-chat notifications that were never read within the retention
    // window (unread ones a user never opened), on startup and every 30 days.
    mommys_heart_app::server::db::channel_notifications::start_retention_task();

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        // The dedicated JSON API (+ CORS for the cross-origin Squarespace widget).
        .merge(api::router::<LeptosOptions>().layer(api::cors_layer()));

    // Wire the evidence HTTP surface
    let app = server_fns::evidence::install(app);
    let app = api::message_transcripts::install(app)
        .fallback(leptos_axum::file_and_error_handler(shell))
        // The security layer is outermost so it covers SSR, RPC, and REST responses.
        .layer(middleware::from_fn(
            mommys_heart_app::server::security::protect,
        ))
        // Log every incoming request (method, path, status, latency).
        .layer(TraceLayer::new_for_http())
        .with_state(leptos_options);

    tracing::info!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // The client-side (wasm) entry point is `lib::hydrate`; this binary target
    // only runs under the `ssr` feature.
}
