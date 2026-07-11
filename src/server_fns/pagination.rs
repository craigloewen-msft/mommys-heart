//! Generic pagination envelope shared by every paginated server function and
//! the client that renders it.

use serde::{Deserialize, Serialize};

/// One page of a larger result set: the rows for this page plus the total
/// number of rows matching the query (so a UI can show "showing N of M" and
/// decide whether to offer "Load more").
///
/// Deliberately generic and minimal so it is reused by every paginated list
/// (users today; cases, grants, messages, … as they grow) instead of each
/// endpoint inventing its own shape.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    /// Total rows matching the query across every page (not just this one).
    pub total: i64,
}
