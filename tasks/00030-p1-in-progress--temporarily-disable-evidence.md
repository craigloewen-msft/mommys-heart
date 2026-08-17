# Temporarily disable evidence views and interactions

## Goal

Temporarily take the current evidence feature out of service while its replacement is designed. Users should see a clear under-construction notice instead of evidence contents or controls, and stale/direct requests must not be able to read or change evidence.

Existing evidence records, blobs, database schema, capability assignments, and implementation code should remain intact for the future redesign.

## Plan

1. Add a single shared “evidence is under construction and temporarily unavailable” message/state so the UI and server responses use consistent wording.
2. Replace the shared evidence browser in case details with a non-interactive notice for every case-detail context (client, volunteer/staff, and admin read view).
   - Do not render folder names, file metadata, download links, upload inputs, move/delete controls, or folder-management controls.
   - Show the notice wherever the user would otherwise have had evidence access.
   - Remove now-unused client-side evidence browser/upload helpers and imports so no dormant controls or handlers ship in the hydrated UI.
3. Disable the current evidence HTTP/server-function surface as defense in depth.
   - Reject evidence uploads, placeholder creation, moves, deletes, and downloads with a temporary-unavailability response before any evidence data or blob is changed/read.
   - Reject evidence folder creation/deletion for the same reason.
   - Preserve all stored data and underlying implementation for later replacement; do not migrate or delete anything.
4. Keep unrelated case features and the existing authorization/capability model unchanged. Evidence capabilities may remain stored so access assignments are not destructively rewritten during this temporary shutdown.
5. Verify with the repository workflow:
   - Run the SSR compile check.
   - Build and run the application.
   - Inspect representative client/staff/admin case details to confirm the notice appears and no evidence data or controls are visible.
   - Confirm direct evidence mutation/download requests are refused and other case features still work.

## Acceptance criteria

- Every case evidence section that was previously available displays a clear under-construction/disabled-for-now notice.
- No website user can browse evidence folders/files or upload, download, move, or delete evidence.
- No website user can create or delete evidence folders.
- Direct or stale evidence requests cannot bypass the disabled UI.
- Existing evidence data and capability assignments are preserved.
- Case information, notes, contacts, messaging, and other unrelated behavior are unaffected.
