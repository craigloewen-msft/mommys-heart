# Future brief 05: time units, funding reporting, and global search

This brief satisfies item 5 of `REQ-DOC-003`.

## User need

Operations staff need more detailed service units, grant/funding codes, funder
reporting, and global authorized search across records when the MVP's per-case
retrieval is no longer enough.

## Scope boundary

Add detailed time/service units, grants/funding codes, funder reporting, and
global authorized search.

This brief does **not** define a complete finance system, automated grant
compliance, or broad search access without explicit authorization rules.

## Dependencies

- Stable service-record and note data model
- Funding-code taxonomy and reporting ownership
- Search index strategy with strict audience filtering
- Export/report validation rules

## Open policy questions

1. Which records may appear in global search results, and to which roles?
2. What funding-code hierarchy is organization-approved?
3. Which reports are operational convenience vs. official funder submissions?

## Acceptance outcomes

- Staff can record service units and funding codes in structured form.
- Authorized reports aggregate data without copying sensitive free text by
  default.
- Global search returns only records the caller is allowed to know exist.
