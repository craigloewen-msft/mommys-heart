pub mod api_client;
pub mod app;
pub mod types;

pub mod components {
    pub mod layout;
}

pub mod pages {
    pub mod chat;
    pub mod contact_detail;
    pub mod contacts;
    pub mod dashboard;
}

#[cfg(feature = "ssr")]
pub mod server {
    pub mod captcha;
    pub mod data;
    pub mod rest;
    pub mod service;
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
