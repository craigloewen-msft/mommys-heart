#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use mommys_heart_crm::app::{shell, App};
    use mommys_heart_crm::server::rest;

    // Load local .env in development (Azure OpenAI keys, ALLOWED_ORIGINS, etc.).
    let _ = dotenvy::dotenv();

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
        .merge(rest::router::<LeptosOptions>().layer(rest::cors_layer()))
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    log!("listening on http://{}", &addr);
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
