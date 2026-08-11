# Case-management roadmap

This roadmap satisfies `REQ-DOC-002` by mapping the client's broader request to
`MVP`, `next`, `later`, or `needs policy decision`.

## Status key

- `MVP` — intentionally included in the approved case-management MVP.
- `next` — likely follow-on work after MVP once dependencies are ready.
- `later` — explicitly deferred beyond the near follow-on window.
- `needs policy decision` — blocked on legal, operational, security, or product
  policy decisions that are not settled in the approved task.

## Coverage note

The approved task references a client-supplied 19-section specification but does
not reproduce the exact section titles in-repo. To keep roadmap coverage stable
and honest, this file uses normalized section labels derived from the approved
scope, deferred items, and future-brief clusters. When the client packet is
later decomposed line-by-line, these labels should be cross-walked to the exact
section names rather than renumbered.

## Roadmap matrix

| Client area | Normalized section | Status | MVP / deferred summary | Dependencies | Acceptance question |
| --- | --- | --- | --- | --- | --- |
| 01 | Case access and audience boundaries | MVP | Server-enforced case and audience authorization for channels, notes, counts, and audit visibility. | Existing capabilities model; audience-aware audit work. | Can a client fail to discover volunteer-only records across UI, direct calls, counts, and exports? |
| 02 | Shared secure messaging | MVP | Authenticated text-only in-app case chat with immutable messages and throttled sending. | Existing auth, MFA, case/channel model, transactional persistence. | Can an assigned client and volunteer exchange messages without edit/delete paths? |
| 03 | Volunteer-only collaboration | MVP | Preserve the permanent volunteer-only channel and make its metadata invisible to clients. | Existing volunteer-only channel model; hardened repository filtering. | Does every client-facing path suppress volunteer-only existence and unread signals? |
| 04 | Message archival and transcript export | MVP | Archive standard channels instead of deleting them and allow authorized UTF-8 CSV export. | New archive state; export authorization; restricted audit metadata. | Can admins export only channels they may see, and are exports themselves audited? |
| 05 | Notification safety and delivery limits | MVP | Content-free message notifications only; no case or message details in email/browser summaries. | Current notification settings; template update; post-commit delivery path. | Do all new-message notifications avoid leaking case identity or message content? |
| 06 | Structured Case Note authoring | MVP | Bounded structured Case Note form with explicit MVP fields instead of the full 19-section dynamic form. | New note domain modules; create/detail UI route; server validation. | Can staff create a useful internal record without implying the full client form already exists? |
| 07 | Draft save, discard, and finalization | MVP | Draft lifecycle with author-only editing, discard tombstones, validation, and in-app attestation. | Transactional persistence; authoritative server timestamps and identity. | Can an incomplete draft survive reload and later finalize only after passing validation? |
| 08 | Legacy note preservation and addenda | MVP | Migrate historical notes to immutable legacy records; allow immutable signed addenda only. | Forward-only migration; legacy audience preservation; addendum parentage. | Does migration preserve historical content and audience without silent widening or revocation? |
| 09 | Case-level note list, filters, and pagination | MVP | Per-case paginated notes list with server-side filters; no global cross-client note search yet. | Paginated repository queries; filter UI; keyword search within a case. | Can authorized staff filter note history without loading every note for the case? |
| 10 | Restricted audit trail and retention honesty | MVP | Content-free restricted audit metadata, audience-aware queries, and honest 10-year current audit retention wording. | Audit schema changes; current audit retention task; documentation discipline. | Do docs avoid claiming indefinite retention, legal hold support, or certifications not actually present? |
| 11 | Supervisor review and returned revisions | next | Add supervisor review, returned revision, and approval states above MVP finalization. | Future workflow engine; role model clarification; approval UI. | Can a supervisor return a note with a reason and preserve the full review trail? |
| 12 | Urgent safety alerts and acknowledgement | needs policy decision | Add urgent safety escalation and acknowledgement only after operations policy and on-call ownership are defined. | Escalation policy; contact roster; legal review; logging and acknowledgement design. | Who must be alerted, how fast, through which channel, and what acknowledgement is mandatory? |
| 13 | Referrals, providers, and service records | next | Add referrals, outside providers, service records, programs, and linked service history. | Data model for external organizations; note/link model; privacy review. | Can staff record service relationships without turning case notes into unstructured workaround text? |
| 14 | Goals, plans, and objectives | next | Add client goals and service-plan objectives beyond the MVP note-purpose field. | Longitudinal service-plan model; review cadence; reporting needs. | Can staff track progress against explicit goals over time instead of retyping them in notes? |
| 15 | Tasks, reminders, deadlines, and timeline | later | Add tasks, reminders, court proceedings, deadlines, and a Case Timeline view. | Notification engine; calendar semantics; event model; reminder ownership. | Can the system track dated obligations without implying emergency or legal deadline guarantees? |
| 16 | Attachments, document links, PDF, and stronger signatures | later | Add note/message attachments, document metadata/linking, PDF generation, and stronger signature options. | Storage/linking design; malware scanning; PDF renderer; signature policy decision. | Can generated or attached records stay immutable, attributable, and safe to download? |
| 17 | Time units, grants, funding codes, and reporting | later | Add detailed service units, grants/funding codes, and funder reporting beyond MVP totals. | Funding taxonomy; reporting extracts; data validation; finance/ops ownership. | Are service records structured enough to support credible funder reports without duplicate entry? |
| 18 | Sensitive-note controls and break-glass access | needs policy decision | Add granular permissions, break-glass access, access reviews, safe-contact preferences, and field-level restrictions. | Permission matrix; audit/alert expectations; policy approval; UX for emergency access. | Which roles may override restrictions, under what audit burden, and how are clients protected? |
| 19 | Governance, migration, security, and accessibility operations | needs policy decision | Add legal holds, backup/restore drills, incident response, accessibility commitments, migration/export tooling, and independent review. | Organizational governance; budget; vendor/process selection; operational runbooks. | What operational controls must be in place before the organization can claim stronger assurance? |
| S1 | Official record submission outcomes | MVP | Official record submission for structured Case Notes is draft save, finalize with attestation, or discard with tombstone only. Approval and return-for-revision are deferred. | Structured notes lifecycle; attestation; future supervisor workflow. | Is it clear which record states exist today and which approval states do not? |
| S2 | Authorized search and retrieval outcomes | MVP | Messaging supports authorized transcript export; notes support per-case list/filter/keyword. Global authorized search is deferred. | Export endpoint; paginated note queries; future search/index design. | Can staff retrieve what the MVP promises without creating cross-case metadata leakage? |

## Cross-cutting dependencies

- Existing case capability enforcement must remain the primary authorization seam.
- Audience-aware audit visibility must land before sensitive record rollout is
  considered complete.
- Documentation must continue to distinguish current app behavior from policy,
  compliance, or retention claims not yet supported.
- Any feature that sends alerts outside the authenticated app depends on settled
  operations ownership and escalation procedures.

## Open policy questions

1. What exact client-spec section titles correspond to the normalized section
   labels above?
2. Which roles may supervise, approve, or return official records?
3. What constitutes an urgent safety event that requires escalation beyond the
   app, and who must acknowledge it?
4. What retention, legal-hold, and export obligations are organization-approved
   rather than merely desired?
5. What minimum accessibility and independent security-review standard is needed
   before stronger external assurances are made?

## Acceptance outcomes for this roadmap

- Every major area named in the approved task is accounted for.
- The roadmap covers 19 normalized client sections plus explicit
  submission/search outcomes.
- Deferred items are visible without being misrepresented as current behavior.
- Policy-blocked work is separated from straightforward follow-on engineering.
