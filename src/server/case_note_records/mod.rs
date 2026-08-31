//! Filing a case note as a document in the case's SharePoint `Case Notes`
//! folder (SSR only).
//!
//! A finalized case note is two things at once, and this module is the join
//! between them:
//!
//! * the **record**, which is the immutable `case_notes` row — protected by
//!   triggers, searchable, audited, and the thing the app reasons about; and
//! * the **document**, which is a `.docx` in the case's `Case Notes` folder —
//!   the thing a person opens, prints, or hands to a court, sitting with the
//!   rest of that case's paperwork.
//!
//! The library is the source of truth for the document. Nothing here keeps a
//! copy of its bytes, mirrors its listing, or serves it through a path of its
//! own: it is browsed, downloaded and opened by exactly the same surface as
//! every other case file. Postgres keeps only the pointer
//! ([`crate::server::db::case_note_documents`]), for the same reason a case
//! keeps its folder id — so the app can find the file again without searching
//! the library by name.
//!
//! # Filing never blocks the record
//!
//! A note becomes immutable the moment its transaction commits, and a
//! SharePoint outage must not be able to fail that write. So [`file_note`] runs
//! *after* the commit, spawned by [`file_note_in_background`] exactly as
//! [`sync_case_access`](crate::server::sharepoint::sync_case_access) is, and
//! anything it misses is repaired later by [`start_filing_backfill`] rather
//! than being remembered as a queue. That is the same reconcile-don't-remember
//! shape the rest of the SharePoint integration uses: the desired state is
//! computable ("every finalized note has an up-to-date document"), so it is
//! computed rather than tracked.

pub mod docx;

use crate::server::db::{case_note_documents as db, case_notes, cases};
use crate::server::sharepoint::{self, CASE_NOTES_FOLDER};
use crate::server_fns::case_notes::{CaseNoteDetail, CaseNoteState};

/// How many notes one backfill pass repairs. Bounded like
/// [`unprovisioned_case_ids`](crate::server::db::case_documents::unprovisioned_case_ids),
/// so a large backlog is worked through over several restarts instead of
/// hammering the library at boot.
const BACKFILL_LIMIT: i64 = 50;

/// File a note's document without making the caller wait for the library.
///
/// Used by finalization and by adding an addendum: both have already committed
/// the thing that matters, and neither should fail because SharePoint is slow.
pub fn file_note_in_background(note_id: String) {
    tokio::spawn(async move {
        if let Err(e) = file_note(&note_id).await {
            tracing::warn!("could not file the document for case note {note_id}: {e}");
        }
    });
}

/// Write a note's document into its case's `Case Notes` folder, replacing any
/// document already filed for that note, and record where it went.
///
/// Idempotent: filing the same note twice writes the same file name, which the
/// store replaces rather than duplicating. That is what makes an addendum a
/// re-file rather than a second document — the filed record is always the whole
/// note including its corrections.
///
/// Awaited rather than spawned, so the retry button and the backfill can both
/// report what happened.
pub async fn file_note(note_id: &str) -> Result<(), String> {
    let detail = case_notes::detail_for_filing(note_id)
        .await
        .map_err(|e| format!("database error: {e}"))?
        .ok_or_else(|| "that note no longer exists".to_string())?;

    // Drafts are private to their author until finalized, and a discarded note
    // has had its content removed — there is nothing to file in either case.
    if !matches!(
        detail.state,
        CaseNoteState::Finalized | CaseNoteState::Legacy
    ) {
        return Err(format!(
            "a note in state '{}' is not filed",
            detail.state.slug()
        ));
    }

    let case_name = cases::name(&detail.case_id)
        .await
        .map_err(|e| format!("database error: {e}"))?
        .unwrap_or_else(|| detail.case_id.clone());

    let bytes = docx::render(&detail, &case_name)?;
    let file_name = docx::file_name(&detail);

    // The case's folder, created now if this case has never had one — a case
    // that predates the documents feature files its notes just the same.
    let case_folder = sharepoint::sync::ensure_folder_ref(&detail.case_id).await?;
    let store = sharepoint::store()?;
    // `Case Notes` is part of the standing tree, but a folder somebody deleted
    // in SharePoint would otherwise make filing fail forever.
    let folder = store
        .ensure_folder(&case_folder.item_id, CASE_NOTES_FOLDER)
        .await?;

    store
        .upload(&folder.item_id, &file_name, bytes, docx::DOCX_MIME)
        .await?;

    let web_url = document_url(store, &folder.item_id, &file_name).await;
    db::record(
        &detail.id,
        &detail.case_id,
        &file_name,
        &web_url,
        detail.addenda.len() as i32,
    )
    .await
    .map_err(|e| format!("database error: {e}"))?;

    // One content-free audit entry, using the `export_note` action the note
    // audit schema already defines. The filed document is a copy of the record
    // leaving the app, which is exactly what that action is for.
    if let Err(e) = case_notes::record_filing_audit(&detail, &file_name).await {
        tracing::warn!("could not record the filing of case note {note_id}: {e}");
    }

    tracing::info!(
        "filed case note {} as '{CASE_NOTES_FOLDER}/{file_name}' on case {}",
        detail.id,
        detail.case_id
    );
    Ok(())
}

/// The browser link to the document that was just uploaded.
///
/// Read back from the folder listing because upload does not return one, and
/// best-effort: an empty link costs the "Open in SharePoint" button, while a
/// failure here would cost the filing.
async fn document_url(
    store: &dyn sharepoint::DocumentStore,
    folder_id: &str,
    file_name: &str,
) -> String {
    match store.list_children(folder_id).await {
        Ok(entries) => entries
            .into_iter()
            .find(|entry| entry.name == file_name)
            .map(|entry| entry.web_url)
            .unwrap_or_default(),
        Err(e) => {
            tracing::warn!("could not read back the link for '{file_name}': {e}");
            String::new()
        }
    }
}

/// File the notes whose document is missing or out of date, in the background.
///
/// Runs once at startup, and covers three cases with one query: notes finalized
/// while the library was unreachable, notes that gained an addendum during an
/// outage, and every note that already existed before notes were filed at all.
pub fn start_filing_backfill() {
    tokio::spawn(async move {
        // Let startup logging finish, and let the case-folder backfill go first
        // — filing needs those folders to exist.
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;

        let note_ids = match db::needing_filing(BACKFILL_LIMIT).await {
            Ok(ids) => ids,
            Err(e) => {
                tracing::warn!("could not look for case notes needing filing: {e}");
                return;
            }
        };
        if note_ids.is_empty() {
            return;
        }
        tracing::info!("filing documents for {} case note(s)", note_ids.len());
        for note_id in note_ids {
            if let Err(e) = file_note(&note_id).await {
                tracing::warn!("could not file the document for case note {note_id}: {e}");
                // Almost certainly the library rather than this note; stop
                // rather than failing once per note through the whole batch.
                return;
            }
        }
    });
}

/// The filed document for a note, if there is one.
pub async fn filed_document(note_id: &str) -> Result<Option<db::NoteDocumentRef>, String> {
    db::get(note_id)
        .await
        .map_err(|e| format!("database error: {e}"))
}

/// Whether a note's filed document is up to date with the note.
///
/// Used by the note page to say "filing is pending" honestly rather than
/// showing a link to a document that predates the addendum being read.
pub fn is_current(detail: &CaseNoteDetail, filed: Option<&db::NoteDocumentRef>) -> bool {
    filed.is_some_and(|doc| doc.addenda_included as usize == detail.addenda.len())
}
