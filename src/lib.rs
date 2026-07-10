pub mod app;
pub mod mockdata;
pub mod server_fns;
pub mod state;
pub mod types;

pub mod components {
    pub mod guard;
    pub mod layout;
}

pub mod pages {
    pub mod admin;
    pub mod cases;
    pub mod grants;
    pub mod inbox;
    pub mod login;
    pub mod register;
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
