# Hook up contact types across the unified Contacts workflow

## Problem and audit findings

The canonical CRM contact model already defines contact types as a required, multi-value classification, but the unified Contacts UI does not expose that model consistently.

- `ContactType` defines 13 fixed values and `ContactInput::validate` requires 1–6 distinct values (`src/server_fns/contacts.rs`).
- PostgreSQL stores the values in `contacts.types`, constrains the allowed slugs and non-empty cardinality, and indexes the array for filtering (`migrations/0019_crm_contacts_organizations_and_funding.sql`, `migrations/0020_connected_records_and_default_properties.sql`).
- Canonical contact create/update persistence correctly reads and writes all submitted types (`src/server/db/contacts.rs`).
- The shared structured `ContactForm` already renders a type multi-selector, and contact detail already displays type badges (`src/components/contact_form.rs`, `src/pages/people.rs`).
- The current `/contacts` list uses the newer contact-directory projection and `DirectoryContactEditor`. That projection/input omits types entirely (`src/server_fns/contact_directory.rs`, `src/server/db/contact_directory.rs`, `src/pages/contacts.rs`).
- Creating through the main **Add contact** workflow silently assigns `Other`; editing through **Edit directory fields** silently preserves the prior types. Users therefore cannot intentionally set types in those workflows even though the directory offers a working server-side **Contact type** filter.
- Directory result cards show dynamic categories/tags but not fixed contact types, which makes the type filter appear disconnected from the records it matches.
- The main Contacts workflow now has both fixed contact types and user-managed categories/tags, but the UI does not clearly explain their different purposes.

Existing data does not need migration: migration 0020 already repaired empty legacy arrays to `Other`, and the standing database check requires at least one type.

## Goal

Make contact types visible, understandable, and editable in every supported Contacts create/edit workflow while preserving one canonical type vocabulary and the existing database/server invariants.

## Requirements

### End-to-end directory projection

Extend the contact-directory input and contact projection to carry `Vec<ContactType>`.

- Read `contacts.types` into directory list/detail results using the same tolerant slug conversion as the canonical contact repository.
- Populate edit drafts from the contact's stored types.
- On directory create, persist the user's submitted types instead of silently assigning `Other`.
- On directory edit, persist the submitted types instead of silently preserving the previous list.
- Continue using the canonical `ContactInput::validate` and contact repository write path so deduplication, the 1–6 limit, allowed values, linked-account ownership, persistence, and audit behavior do not diverge.
- Do not create a second type enum, type table, or persistence path.

### One reusable type selector

Extract/reuse one contact-type selector for both the structured `ContactForm` and `DirectoryContactEditor` rather than maintaining two copies.

- Present every `ContactType::ALL` value as an accessible multi-select control.
- State that at least one and at most six types are required.
- Allow selected values to be removed when the maximum is reached; prevent or clearly reject selecting a seventh value.
- Preserve all stored selections when an existing contact is opened for editing.
- Keep authoritative validation on the server and show its readable error in the form.

### Coherent Contacts UX

- Add the type selector to the main `/contacts` **Add contact** form and **Edit directory fields** form.
- Keep types editable in the structured detail **Edit** form, including for contacts linked to an account; types are CRM classification and do not grant access or belong to account identity.
- Show stored contact-type badges on `/contacts` result cards so users can see why a type filter matched.
- Keep the existing type badges on contact detail.
- Add concise helper copy distinguishing fixed, required **Contact types** (broad roles such as Donor or Attorney) from optional, organization-defined **Categories and tags** (more specific grouping).
- Ensure the singular directory filter remains understandable for multi-typed contacts (a contact matches when it includes the selected type).
- Preserve category/tag assignment and filtering independently from types.

### Preserve existing behavior

Do not change the database schema or rewrite existing type values. Preserve:

- server-side keyword/type/organization/category/archive filtering and accurate totals;
- canonical `/contacts` and `/contacts/:id` routing;
- information-management authorization;
- linked-account field ownership and read-only identity controls;
- organization-create contact flow, archive behavior, custom properties, communications, related cases, and change history.

## Likely implementation areas

- `src/server_fns/contact_directory.rs`
- `src/server/db/contact_directory.rs`
- `src/pages/contacts.rs`
- `src/components/contact_form.rs`
- a small shared contact-type selector component/module, if extraction keeps the two forms simple
- concise current documentation only if behavior/copy would otherwise be inaccurate

No migration is expected.

## Acceptance scenarios

1. From `/contacts`, create a contact with one non-`Other` type, reload, and confirm the type appears on the result card and detail page and the matching type filter finds it.
2. Create a contact with multiple types and confirm each corresponding single-type filter finds the same contact.
3. Edit the contact through **Edit directory fields**, add and remove types, reload, and confirm display and filtering reflect the saved values.
4. Edit the same classification through the structured detail **Edit** form and confirm both edit paths remain consistent.
5. Open an existing seeded or linked-account contact and confirm all current types are preselected; change only types and confirm account-owned identity remains untouched.
6. Attempt to save no types and more than six types; receive clear feedback and persist no invalid state. Duplicate submitted values are normalized rather than stored twice.
7. Create a contact from an organization detail page and confirm the shared type selector, persistence, and filters behave the same way.
8. Confirm categories/tags can still be assigned, edited, displayed, and filtered independently of contact types.
9. Check desktop and mobile widths plus keyboard operation; labels, selected state, limits, focus, and errors remain understandable, with no console errors.

## Verification

- `cargo fmt --check`
- `etc/dev.sh -- cargo check --no-default-features --features ssr`
- `etc/dev.sh build`, then `etc/dev.sh run`, waiting for `MH_READY`
- Reset/seed as needed and run the acceptance scenarios in the browser
- Inspect persisted `contacts.types` for representative create/edit cases and confirm type-filter totals/results
- Do not add permanent Cargo tests; this repository does not use them
