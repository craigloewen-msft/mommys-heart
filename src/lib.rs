pub mod api_client;
pub mod app;
pub mod mockdata;
pub mod state;
pub mod taxonomy;
pub mod types;

pub mod components {
    pub mod case_card;
    pub mod layout;
}

pub mod pages {
    pub mod admin;
    pub mod chat;
    pub mod clients;
    pub mod contact_detail;
    pub mod contacts;
    pub mod dashboard;
    pub mod evidence;
    pub mod insights;
    pub mod login;
    pub mod register;
    pub mod volunteer;
}

#[cfg(feature = "ssr")]
pub mod server;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
