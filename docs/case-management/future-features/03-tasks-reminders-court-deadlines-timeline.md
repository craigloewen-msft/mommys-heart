# Future brief 03: tasks, reminders, court deadlines, and timeline

This brief satisfies item 3 of `REQ-DOC-003`.

## User need

Case teams need structured follow-up work, dated obligations, court proceedings,
deadlines, reminders, and a timeline view so important obligations are not lost
inside note narratives or chat.

## Scope boundary

Add tasks, reminders, court proceedings, deadlines, and Case Timeline events.

This brief does **not** promise calendar sync, guaranteed deadline monitoring,
or emergency/legal notice handling in the MVP.

## Dependencies

- Stable case activity/event model
- Reminder ownership and notification design
- Court/deadline date semantics and timezone handling
- Clear relationship between notes, messages, and timeline events

## Open policy questions

1. Which deadlines are informational vs. must-alert?
2. Who owns overdue tasks in shared cases?
3. What timeline items may clients eventually see?

## Acceptance outcomes

- Staff can assign and complete case tasks with due dates.
- Court events and deadlines appear on a case timeline.
- Reminders help users follow up without implying guaranteed legal deadline
  coverage.
