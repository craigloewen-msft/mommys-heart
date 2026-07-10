#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use mommys_heart_crm::app::{shell, App};
    use mommys_heart_crm::server::{api, rag, telemetry};
    use tower_http::trace::TraceLayer;

    // Load local .env in development (Azure OpenAI keys, ALLOWED_ORIGINS, etc.).
    let _ = dotenvy::dotenv();

    // Start structured logging first so startup (incl. DB migrations/seeding) is
    // captured. Verbosity is controlled by `RUST_LOG`.
    telemetry::init();

    // `mommys-heart-crm seed` — used by `etc/dev-db.sh seed` to (re)populate the
    // dev database with demo data, then exit without starting the web server.
    if std::env::args().nth(1).as_deref() == Some("seed") {
        if let Err(e) = mommys_heart_crm::server::db::init().await {
            panic!("failed to initialize database: {e}");
        }
        if let Err(e) = mommys_heart_crm::server::db::seed::reseed().await {
            panic!("failed to seed database: {e}");
        }
        tracing::info!("demo data loaded");
        return;
    }

    // Connect to PostgreSQL, run migrations, and seed on first run. Fail fast if
    // the database is unreachable — the CRM cannot function without it.
    if let Err(e) = mommys_heart_crm::server::db::init().await {
        panic!("failed to initialize database: {e}");
    }

    // Kick off document ingestion in the background so the server starts
    // serving immediately; the RAG store fills in once embeddings complete.
    rag::start_background_ingest();

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
        .merge(api::router::<LeptosOptions>().layer(api::cors_layer()))
        .fallback(leptos_axum::file_and_error_handler(shell))
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
