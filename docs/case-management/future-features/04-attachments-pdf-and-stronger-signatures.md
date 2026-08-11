# Future brief 04: attachments, PDF, and stronger signatures

This brief satisfies item 4 of `REQ-DOC-003`.

## User need

Staff need official records that can reference attached material, link to
supporting documents, generate portable outputs, and potentially support
stronger signature approaches than MVP in-app attestation.

## Scope boundary

Add Case Note/message attachments, document metadata/linking, PDF generation,
and stronger signature options.

This brief does **not** choose a digital-signature vendor, claim legal validity
for any future signature mode, or define full evidence-management policy.

## Dependencies

- Immutable note/message lifecycle
- Safe file storage, malware scanning, and document-link model
- PDF rendering approach with stable templates
- Signature policy and legal review

## Open policy questions

1. Which file types may be attached to official records?
2. Must generated PDFs include every addendum and audit marker?
3. What signature strength is actually required by organization policy or law?

## Acceptance outcomes

- Authorized users can attach or link supporting material without mutating the
  parent official record.
- Generated PDFs faithfully represent the underlying immutable record.
- Any stronger signature mode is explicit about what assurance it does and does
  not provide.
