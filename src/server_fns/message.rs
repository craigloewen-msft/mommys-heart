//! Case chat message type shared by the server functions, the DB layer, and the
//! pages that render a case's chat thread.

use serde::{Deserialize, Serialize};

/// A message posted in a case's chat thread. Every case has its own thread that
/// any assigned user with the `SendMessages` capability can read and post to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    /// The case whose chat this message belongs to.
    pub case_id: String,
    /// User id of the author.
    pub author_id: String,
    /// Display name of the author.
    pub author: String,
    pub body: String,
    /// Human-readable timestamp (mock).
    pub sent_at: String,
}
