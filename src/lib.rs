#![recursion_limit = "256"]

pub mod app;
pub mod helpers;
pub mod mockdata;
pub mod server_fns;
pub mod state;

pub mod components {
    pub mod admin_requests;
    pub mod case_intake;
    pub mod change_log;
    pub mod email_failures;
    pub mod guard;
    pub mod layout;
    pub mod loading;
    pub mod profile_link;
    pub mod volunteer_hours;
}

pub mod pages {
    pub mod admin;
    pub mod case_signup;
    pub mod cases;
    pub mod forgot_password;
    pub mod inbox;
    pub mod login;
    pub mod mfa;
    pub mod profile;
    pub mod register;
    pub mod reset_password;
    pub mod settings;
    pub mod verify_email;
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
