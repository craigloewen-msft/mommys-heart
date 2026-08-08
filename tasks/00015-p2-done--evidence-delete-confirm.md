# Replace the evidence "Edit" toggle with per-row delete + confirm

## Problem

On the case page the Evidence panel has an "Edit" button
(`src/pages/cases.rs:1502`) that only flips a `removing` signal. That signal
reveals a "Remove" button on file rows (`:959`) and a "Delete" button on
non-root sub-folder rows (`:1089`). At the top level of a case there are no
loose files and root folders cannot be deleted, so pressing "Edit" visibly does
nothing — which is what the user reported.

Two further problems:

- Deletes fire on a single click with no confirmation.
- Deleting a non-empty folder is only rejected server-side
  (`src/server_fns/case_folders.rs:149`), after the click, as a red error line.

## Plan

1. **Remove the `Edit`/`Done` toggle** and the `removing` signal from
   `src/pages/cases.rs`. Delete controls become always visible for users who
   have `CaseCapability::DeleteEvidence` (and `accepts_changes`), i.e. gated by
   `can_delete_evidence` alone.

2. **Per-row delete with inline confirm**, following the existing pattern in
   `src/components/volunteer_hours.rs:156-225`:
   - a single `pending_delete: RwSignal<Option<String>>` holding the id of the
     row awaiting confirmation (folder ids and evidence ids share the signal;
     ids are unique across both),
   - normal state: a small, unobtrusive garbage-can icon button at the end of
     the row (right-aligned, after the existing move/folder controls) rather
     than a labelled "Delete" button. It carries a `title`/`aria-label` of
     `"Delete"` for hover and screen readers,
   - confirming state: that row swaps to "Cancel" + a filled destructive
     "Delete" confirm button, with a short inline line naming what is about to
     be deleted (e.g. `Delete "Medical records.pdf"? This cannot be undone.`),
   - opening a confirm clears any other pending confirm and the folder error,
   - a busy flag disables the confirm button while the request is in flight, and
     `pending_delete` is cleared on success or failure.

3. **Do not allow folder delete when the folder is not empty.** `folder_row`
   already receives `file_count` and `child_count`. When either is non-zero,
   still render the garbage-can icon but in a *disabled* state (dimmed,
   `disabled` attribute, no confirm on click) with a hover hint
   (`title="Empty this folder before deleting it"`), so the rule is
   discoverable before the click rather than after. Root folders keep no delete
   control at all, as today.

4. Keep the server-side checks in `delete_case_folder` unchanged — they remain
   the real enforcement; the UI change is defence-in-depth plus clarity.

5. Styling: reuse the rose/slate button classes already used in
   `volunteer_hours.rs` and the current evidence rows so the panel keeps its
   look.

## Verification

- `cargo check --no-default-features --features ssr`.
- Manual pass with `etc/dev-run.sh` as a volunteer/admin on a case:
  - file row: Delete → Cancel leaves the file; Delete → Delete removes it,
  - empty sub-folder: Delete → confirm removes it,
  - sub-folder with contents: garbage can is dimmed/disabled, hover hint shown,
  - root folders: no Delete control,
  - client view (no `DeleteEvidence`): no delete controls anywhere.
