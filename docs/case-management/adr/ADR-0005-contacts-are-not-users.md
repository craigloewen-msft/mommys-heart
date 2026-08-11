# ADR-0005: Contacts are not users

- Status: accepted
- Date: 2026-08-11

## Context

Before the CRM, `users` was the only representation of a human being in the
system. That table requires a unique email and a password hash, because its
purpose is authentication.

A CRM has to record people who must never be given a login: donors, funder
program officers, partner-agency caseworkers, opposing attorneys, court clerks,
board members, and the emergency contact a client names at intake. Before this
change the only place to put such a person was free text — or, in the case of a
volunteer's emergency contact, four flat columns on the `volunteers` table
(`emergency_first_name`, `emergency_last_name`, `emergency_relationship`,
`emergency_phone`), which cannot be searched, reused, or linked to a case.

## Decision

- Add a `contacts` table for **people**, separate from `users`, which remains the
  table for **accounts**.
- A contact may link to at most one user through a nullable `user_id` with a
  partial unique index; a user may therefore have at most one contact.
- When a contact is linked, `users` stays the source of truth for identity,
  email, and role. Reads join those fields for display; the contact never stores
  a copy, a password hash, or a role, and never becomes a second login path.
- `contacts.user_id` is `ON DELETE SET NULL`, so removing an account does not
  erase the person record, and archiving a contact never affects the account.
- Migration 0019 backfills one contact per existing user so the directory is
  populated and current staff and clients are manageable from day one.
- Contact properties reuse the `case_properties` shape (ordered key/value rows
  under a free-text section) but deliberately carry **no `visibility` column**.
  On a case, `shared` versus `volunteer_only` answers "may the client see this?";
  clients cannot see contacts at all, so the same words would mean something
  weaker here, and `migrations/0004` is explicit that the app should have one
  word per question.

## Consequences

- The organization can record anyone it deals with without creating an account
  for them, which is the precondition for donor, funder, and partner tracking.
- A person's CRM record and their ability to sign in have independent lifecycles.
- Two names can exist for the same human if a contact is created and an account
  is later added without linking them. Linking is an explicit administrative
  action; duplicate detection and merging are future work.
- Field-level sensitivity on contact data is not available, and adding it later
  means designing a real permission model rather than reusing case visibility.

## Alternatives considered

- Add nullable columns to `users` and allow password-less accounts. Rejected:
  every authentication path would then have to defend against a user row that
  cannot log in, which is exactly the sort of implicit state that causes
  security bugs.
- Store contacts as case properties or note text. Rejected: they could not be
  searched, reused across cases, or attached to a grant.
- Reuse `Visibility` on contact properties. Rejected as above — it would give an
  established vocabulary a second, weaker meaning.

## Requirement links

- `REQ-CRM-001`
- `REQ-CRM-002`
- `REQ-CRM-003`
- `REQ-CRM-006`
- `REQ-CRM-020`
