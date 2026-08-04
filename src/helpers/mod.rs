//! Small shared vocabulary and pure helpers that belong to no single feature.
//!
//! A module lands here when more than one part of the app needs it and it
//! answers a question none of them owns. [`visibility`] is the clearest case: a
//! case property and a case file each carry a visibility, so putting the type in
//! either one would make the other import a concept from a module it has nothing
//! to do with.
//!
//! Everything here is plain data and pure functions — no database access, no
//! request handling — so it compiles for both the server and the WASM client.

pub mod format;
pub mod case_intake;
pub mod new_case_fields;
pub mod sections;
pub mod visibility;
