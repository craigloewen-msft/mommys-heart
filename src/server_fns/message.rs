//! Case chat message type shared by the server functions, the DB layer, and the
//! pages that render a case's chat thread.

use serde::{Deserialize, Serialize};

/// A message posted in one of a case's chat channels. Which users may read it
/// is decided entirely by its channel: standard channels are shared with
/// everyone who can view the case, while the volunteer-only channel is
/// restricted to volunteers and admins.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    /// The case whose chat this message belongs to.
    pub case_id: String,
    /// The channel within that case's chat.
    pub channel_id: String,
    /// User id of the author.
    pub author_id: String,
    /// Display name of the author.
    pub author: String,
    pub body: String,
    /// Human-readable timestamp (mock).
    pub sent_at: String,
}
