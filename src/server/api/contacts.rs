//! `GET /api/contacts` and `GET /api/contacts/{id}` — CRM contacts.

use axum::{extract::Path, http::StatusCode, routing::get, Json, Router};

use crate::server::service;
use crate::types::Contact;

pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/contacts", get(contacts))
        .route("/api/contacts/{id}", get(contact))
}

async fn contacts() -> Json<Vec<Contact>> {
    Json(service::list_contacts())
}

async fn contact(Path(id): Path<String>) -> Result<Json<Contact>, (StatusCode, String)> {
    service::get_contact(&id)
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, format!("Contact '{id}' not found")))
}
