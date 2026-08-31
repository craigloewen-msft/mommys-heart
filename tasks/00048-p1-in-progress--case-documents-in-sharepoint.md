# Case documents in SharePoint

Bring back the disabled evidence feature as a **Documents** area whose source of
truth is a SharePoint document library.

## Problem

Evidence was switched off in task 00030: `ensure_evidence_available()` refused
every call, `cases::get` returned empty vectors, and the case page showed an
"under construction" notice. The implementation underneath still stored file
bytes in an Azure Blob container with a `case_folders` tree and one `evidence`
row per file mirrored in Postgres.

The organization already works out of Microsoft 365. Keeping a second, private
copy of every case file in blob storage meant a filing system nobody could reach
except through this app, and a permission model that had to be re-implemented
here. The client asked for the files to live in SharePoint, for a case to get its
folder automatically, and for CRM permission changes to grant and revoke access
to that folder without anyone doing it by hand.

## Approach

**SharePoint holds the files; Postgres holds only pointers and grants.** There is
no mirror table: subfolders and files are read live through Microsoft Graph, so
nothing can drift. The database keeps two things that cannot be recomputed —
`cases.drive_item_id` (where a case's folder is) and `case_document_permissions`
(which invitations we issued, so a revoke withdraws exactly what a grant made).

### Keeping the two audiences

A case's top-level folders each declare an audience (`NEW_CASE_FOLDERS`), and
that is real access control. So **invitations are issued per top-level folder,
never on the case root**: a volunteer is invited to all of them, a client only to
the shared ones. Inviting anyone to the case root would hand them the
volunteer-only paperwork.

### One reconcile instead of six grant sites

Case access changes in six places (assign, toggle, batch save, unassign, an
approved access request, account deactivation). Issuing an invitation at each
would be six chances to forget the matching revoke. All six instead call
`sharepoint::sync_case_access(case_id)`, which computes what access *should* be
from `case_assignments` and changes only the difference. Grant and revoke become
one code path, and a failed call is repaired by the next one rather than leaving
access wrong forever.

### Addressing files

The browser never sends a library id. It sends the case id and a path relative to
the case folder (`Intake/Service Agreement`), validated segment by segment and
resolved against the id stored for that case. The first segment names the
top-level folder, so the existing `require_cap` + `require_visibility` gate
applies before Graph is touched. There is no id a caller could substitute to
reach another case's files.

### Provisioning

Case creation is one transaction, and a network call has no business inside one —
nor should an unreachable library stop a case being created. So provisioning runs
*after* the commit, best-effort. A case left without a folder is caught by the
startup backfill or the "Set up documents folder" button on the case page. The
provisioning function is idempotent, so retrying is always safe.

### Testing it without a tenant

`DocumentStore` has two implementations: `graph` (the real library) and `local`
(a directory tree under `target/sharepoint/`, with invitations in a JSON file).
The trait is the only path to a store, so the whole feature — provisioning,
browsing, upload, download, delete, grant, revoke — is exercisable with no
Microsoft tenant. `SHAREPOINT_BACKEND=local` selects it; production refuses it.

## What shipped

- `migrations/0033_case_documents_sharepoint.sql` — the two `cases` columns, the
  `case_document_permissions` table, and **drops** `evidence`,
  `evidence_scan_log` and `case_folders`.
- `server/sharepoint/{mod,graph,local,sync}.rs` — the trait, its two
  implementations, and the reconcile.
- `server/db/case_documents.rs`, `server_fns/documents.rs`, and the Documents
  panel in `pages/cases.rs`.
- Removed: `server/storage.rs`, `server/malware.rs`, `server/db/evidence.rs`,
  `server/db/case_folders.rs`, `server_fns/evidence.rs`,
  `server_fns/case_folders.rs`, the three `azure_*` crates, and the Azurite
  service in `.kingdom/services.toml`.

## Decisions worth remembering

- **The capability slugs still say `evidence`.** `view_evidence` and friends are
  the values stored in `case_assignments` rows; renaming them would mean
  rewriting live permission data for a cosmetic gain. Only the labels changed.
- **Malware scanning is the tenant's now.** Microsoft 365 scans the library, so
  `MALWARE_SCAN_ADDR` and the ClamAV path are gone. The upload path still
  content-sniffs with `infer` and caps at 25 MB — a file is judged by its bytes,
  never its extension or declared type.
- **A non-empty folder still cannot be deleted.** Carried over deliberately from
  the old implementation: deleting a tree in one click is how people lose
  paperwork they meant to keep.
- **App-only access reads everything.** The server reads the library as itself,
  which is what lets the case page render a folder for someone not personally
  invited — and means this app's capability checks are the effective control on
  reads. Recorded in `docs/operations/production-readiness.md`.
- A case rename does not rename its SharePoint folder. The folder name carries
  the case id, so it stays findable; worth doing later, not before the basics
  are proven.

## Still to confirm with the organization

A tenant admin must supply an Entra app registration (tenant id, client id,
secret) with **Application** `Files.ReadWrite.All` + `Sites.ReadWrite.All` and
admin consent, plus the SharePoint site URL. Inviting clients additionally needs
external sharing enabled on that site — without it staff invitations and the
in-app view still work, only a client's own SharePoint access does not.

The startup log states which store is in use, and an unreachable or
misconfigured library is reported there rather than on first use.
