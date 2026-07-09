//! Org-owned communications store (SSR only).
//!
//! Retained communications live **here on the server**, not in a volunteer's
//! browser session or personal device — that is what makes them "owned by the
//! organization" as the client asked. Conversations reference volunteers/cases
//! by id, so reassigning (or clearing) an owner when someone leaves never loses
//! the history.
//!
//! Storage is in-memory for the proof of concept, but every access goes through
//! the [`CommRepository`] trait. Swapping in a SQLite/SQLx-backed implementation
//! later is a drop-in change: implement the trait and point [`repo`] at it — the
//! service layer, API, and UI never change.

use std::sync::{OnceLock, RwLock};

use crate::types::{
    AuthorKind, Channel, Conversation, ConversationStatus, ConversationThread, Message,
    MessageDirection, SourceInfo,
};

/// Filters accepted by [`CommRepository::list`]. All `None` means "everything".
#[derive(Clone, Debug, Default)]
pub struct ConversationFilter {
    pub status: Option<ConversationStatus>,
    pub channel: Option<Channel>,
    pub assigned_volunteer_id: Option<String>,
    pub case_id: Option<String>,
    pub contact_id: Option<String>,
}

/// A message to append, without server-assigned fields (`id`, `created_at`).
#[derive(Clone, Debug)]
pub struct NewMessage {
    pub channel: Channel,
    pub direction: MessageDirection,
    pub author_kind: AuthorKind,
    pub author_id: Option<String>,
    pub author_label: String,
    pub body: String,
    pub sources: Vec<SourceInfo>,
}

/// A conversation to create, without server-assigned fields.
#[derive(Clone, Debug)]
pub struct NewConversation {
    pub channel: Channel,
    pub subject: String,
    pub status: ConversationStatus,
    pub contact_id: Option<String>,
    pub case_id: Option<String>,
    pub assigned_volunteer_id: Option<String>,
}

/// The persistence boundary for communications. The rest of the app depends on
/// this trait, not on the concrete in-memory store, so the backing store can be
/// replaced (e.g. with a database) without touching callers.
pub trait CommRepository: Send + Sync {
    fn list(&self, filter: &ConversationFilter) -> Vec<Conversation>;
    fn get(&self, id: &str) -> Option<ConversationThread>;
    fn create(&self, new: NewConversation) -> Conversation;
    fn append_message(&self, conversation_id: &str, msg: NewMessage) -> Option<Message>;
    fn assign(&self, id: &str, volunteer_id: Option<String>) -> Option<Conversation>;
    fn link_case(
        &self,
        id: &str,
        case_id: Option<String>,
        contact_id: Option<String>,
    ) -> Option<Conversation>;
    fn set_status(&self, id: &str, status: ConversationStatus) -> Option<Conversation>;
    fn conversations_for_case(&self, case_id: &str) -> Vec<Conversation>;
    fn conversations_for_contact(&self, contact_id: &str) -> Vec<Conversation>;
    /// Clear ownership from every conversation a volunteer holds and mark them
    /// `New` again, so their work stays with the org when they leave. Returns
    /// how many conversations were reassigned.
    fn unassign_volunteer(&self, volunteer_id: &str) -> usize;
}

/// Process-wide singleton, seeded with a few demo threads on first use.
static COMMS: OnceLock<InMemoryComms> = OnceLock::new();

/// The active repository. Returned as a trait object so a different backend can
/// be substituted here without changing a single caller.
pub fn repo() -> &'static dyn CommRepository {
    COMMS.get_or_init(InMemoryComms::seeded)
}

/// In-memory [`CommRepository`] backed by plain `Vec`s under an `RwLock`.
pub struct InMemoryComms {
    inner: RwLock<Inner>,
}

struct Inner {
    /// Conversations paired with a recency key (higher = more recently active).
    conversations: Vec<Record>,
    messages: Vec<Message>,
    seq: u64,
}

struct Record {
    conv: Conversation,
    order: u64,
}

impl InMemoryComms {
    fn new() -> Self {
        Self {
            inner: RwLock::new(Inner {
                conversations: Vec::new(),
                messages: Vec::new(),
                seq: 0,
            }),
        }
    }

    /// Build the store pre-populated with representative demo conversations that
    /// reference the seeded contacts (`data.rs`) and cases (`mockdata.rs`).
    fn seeded() -> Self {
        let store = Self::new();

        // A web-chat lead already folded into a contact + case record.
        let c1 = store.create(NewConversation {
            channel: Channel::WebChat,
            subject: "Website chat — housing help".into(),
            status: ConversationStatus::Assigned,
            contact_id: Some("2".into()),
            case_id: Some("c-1001".into()),
            assigned_volunteer_id: Some("v-1".into()),
        });
        store.append_message(
            &c1.id,
            NewMessage {
                channel: Channel::WebChat,
                direction: MessageDirection::Inbound,
                author_kind: AuthorKind::Visitor,
                author_id: None,
                author_label: "Website visitor".into(),
                body: "Hi, I need help finding emergency housing for me and my kids.".into(),
                sources: Vec::new(),
            },
        );
        store.append_message(
            &c1.id,
            NewMessage {
                channel: Channel::WebChat,
                direction: MessageDirection::Outbound,
                author_kind: AuthorKind::Staff,
                author_id: Some("v-1".into()),
                author_label: "Priya Nair".into(),
                body: "You're in the right place — I'm opening a case and will call you today."
                    .into(),
                sources: Vec::new(),
            },
        );

        // A brand-new, unclaimed web chat waiting in the shared inbox.
        let c2 = store.create(NewConversation {
            channel: Channel::WebChat,
            subject: "Website chat — new inquiry".into(),
            status: ConversationStatus::New,
            contact_id: None,
            case_id: None,
            assigned_volunteer_id: None,
        });
        store.append_message(
            &c2.id,
            NewMessage {
                channel: Channel::WebChat,
                direction: MessageDirection::Inbound,
                author_kind: AuthorKind::Visitor,
                author_id: None,
                author_label: "Website visitor".into(),
                body: "Do you offer help with public benefits applications?".into(),
                sources: Vec::new(),
            },
        );

        store
    }
}

impl Inner {
    fn next_id(&mut self, prefix: &str) -> String {
        self.seq += 1;
        format!("{prefix}-{}", self.seq)
    }

    fn find(&self, id: &str) -> Option<&Record> {
        self.conversations.iter().find(|r| r.conv.id == id)
    }

    fn find_mut(&mut self, id: &str) -> Option<&mut Record> {
        self.conversations.iter_mut().find(|r| r.conv.id == id)
    }

    /// Bump a conversation to the front of the recency ordering.
    fn touch(&mut self, id: &str) {
        self.seq += 1;
        let order = self.seq;
        if let Some(rec) = self.find_mut(id) {
            rec.order = order;
            rec.conv.updated_at = now();
        }
    }
}

impl CommRepository for InMemoryComms {
    fn list(&self, filter: &ConversationFilter) -> Vec<Conversation> {
        let guard = self.inner.read().unwrap();
        let mut recs: Vec<&Record> = guard
            .conversations
            .iter()
            .filter(|r| {
                let c = &r.conv;
                filter.status.is_none_or(|s| s == c.status)
                    && filter.channel.is_none_or(|ch| ch == c.channel)
                    && filter
                        .assigned_volunteer_id
                        .as_ref()
                        .is_none_or(|v| c.assigned_volunteer_id.as_deref() == Some(v))
                    && filter
                        .case_id
                        .as_ref()
                        .is_none_or(|cid| c.case_id.as_deref() == Some(cid))
                    && filter
                        .contact_id
                        .as_ref()
                        .is_none_or(|cid| c.contact_id.as_deref() == Some(cid))
            })
            .collect();
        // Most recently active first.
        recs.sort_by_key(|r| std::cmp::Reverse(r.order));
        recs.into_iter().map(|r| r.conv.clone()).collect()
    }

    fn get(&self, id: &str) -> Option<ConversationThread> {
        let guard = self.inner.read().unwrap();
        let conversation = guard.find(id)?.conv.clone();
        let messages = guard
            .messages
            .iter()
            .filter(|m| m.conversation_id == id)
            .cloned()
            .collect();
        Some(ConversationThread {
            conversation,
            messages,
        })
    }

    fn create(&self, new: NewConversation) -> Conversation {
        let mut guard = self.inner.write().unwrap();
        let id = guard.next_id("conv");
        guard.seq += 1;
        let order = guard.seq;
        let conv = Conversation {
            id,
            channel: new.channel,
            subject: new.subject,
            status: new.status,
            contact_id: new.contact_id,
            case_id: new.case_id,
            assigned_volunteer_id: new.assigned_volunteer_id,
            created_at: now(),
            updated_at: now(),
            last_message_preview: String::new(),
            message_count: 0,
        };
        let clone = conv.clone();
        guard.conversations.push(Record { conv, order });
        clone
    }

    fn append_message(&self, conversation_id: &str, msg: NewMessage) -> Option<Message> {
        let mut guard = self.inner.write().unwrap();
        // Confirm the conversation exists before mutating.
        guard.find(conversation_id)?;
        let id = guard.next_id("msg");
        let message = Message {
            id,
            conversation_id: conversation_id.to_string(),
            channel: msg.channel,
            direction: msg.direction,
            author_kind: msg.author_kind,
            author_id: msg.author_id,
            author_label: msg.author_label,
            body: msg.body,
            sources: msg.sources,
            created_at: now(),
        };
        let preview = preview_of(&message.body);
        guard.messages.push(message.clone());
        guard.touch(conversation_id);
        if let Some(rec) = guard.find_mut(conversation_id) {
            rec.conv.last_message_preview = preview;
            rec.conv.message_count += 1;
        }
        Some(message)
    }

    fn assign(&self, id: &str, volunteer_id: Option<String>) -> Option<Conversation> {
        let mut guard = self.inner.write().unwrap();
        guard.find(id)?;
        guard.touch(id);
        let rec = guard.find_mut(id)?;
        rec.conv.assigned_volunteer_id = volunteer_id.clone();
        // Keep lifecycle consistent with ownership.
        if volunteer_id.is_some() {
            if rec.conv.status == ConversationStatus::New {
                rec.conv.status = ConversationStatus::Assigned;
            }
        } else if rec.conv.status == ConversationStatus::Assigned {
            rec.conv.status = ConversationStatus::New;
        }
        Some(rec.conv.clone())
    }

    fn link_case(
        &self,
        id: &str,
        case_id: Option<String>,
        contact_id: Option<String>,
    ) -> Option<Conversation> {
        let mut guard = self.inner.write().unwrap();
        guard.find(id)?;
        guard.touch(id);
        let rec = guard.find_mut(id)?;
        if case_id.is_some() {
            rec.conv.case_id = case_id;
        }
        if contact_id.is_some() {
            rec.conv.contact_id = contact_id;
        }
        Some(rec.conv.clone())
    }

    fn set_status(&self, id: &str, status: ConversationStatus) -> Option<Conversation> {
        let mut guard = self.inner.write().unwrap();
        guard.find(id)?;
        guard.touch(id);
        let rec = guard.find_mut(id)?;
        rec.conv.status = status;
        Some(rec.conv.clone())
    }

    fn conversations_for_case(&self, case_id: &str) -> Vec<Conversation> {
        self.list(&ConversationFilter {
            case_id: Some(case_id.to_string()),
            ..Default::default()
        })
    }

    fn conversations_for_contact(&self, contact_id: &str) -> Vec<Conversation> {
        self.list(&ConversationFilter {
            contact_id: Some(contact_id.to_string()),
            ..Default::default()
        })
    }

    fn unassign_volunteer(&self, volunteer_id: &str) -> usize {
        let mut guard = self.inner.write().unwrap();
        let ids: Vec<String> = guard
            .conversations
            .iter()
            .filter(|r| r.conv.assigned_volunteer_id.as_deref() == Some(volunteer_id))
            .map(|r| r.conv.id.clone())
            .collect();
        for id in &ids {
            guard.touch(id);
            if let Some(rec) = guard.find_mut(id) {
                rec.conv.assigned_volunteer_id = None;
                if rec.conv.status != ConversationStatus::Closed {
                    rec.conv.status = ConversationStatus::New;
                }
            }
        }
        ids.len()
    }
}

/// First line / trimmed excerpt of a message body for list previews.
fn preview_of(body: &str) -> String {
    let one_line = body.replace('\n', " ");
    if one_line.chars().count() > 120 {
        let truncated: String = one_line.chars().take(120).collect();
        format!("{truncated}…")
    } else {
        one_line
    }
}

/// A coarse timestamp string. The POC has no `chrono` dependency, so this is a
/// simple epoch-seconds marker; a real backend would store proper timestamps.
fn now() -> String {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => format!("{}", d.as_secs()),
        Err(_) => "0".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> InMemoryComms {
        InMemoryComms::new()
    }

    #[test]
    fn create_append_and_get_roundtrip() {
        let s = empty();
        let c = s.create(NewConversation {
            channel: Channel::WebChat,
            subject: "Test".into(),
            status: ConversationStatus::New,
            contact_id: None,
            case_id: None,
            assigned_volunteer_id: None,
        });
        s.append_message(
            &c.id,
            NewMessage {
                channel: Channel::WebChat,
                direction: MessageDirection::Inbound,
                author_kind: AuthorKind::Visitor,
                author_id: None,
                author_label: "Visitor".into(),
                body: "Hello there".into(),
                sources: Vec::new(),
            },
        );
        let thread = s.get(&c.id).unwrap();
        assert_eq!(thread.messages.len(), 1);
        assert_eq!(thread.conversation.message_count, 1);
        assert_eq!(thread.conversation.last_message_preview, "Hello there");
    }

    #[test]
    fn assigning_updates_status_and_unassigning_reverts() {
        let s = empty();
        let c = s.create(NewConversation {
            channel: Channel::WebChat,
            subject: "Test".into(),
            status: ConversationStatus::New,
            contact_id: None,
            case_id: None,
            assigned_volunteer_id: None,
        });
        let assigned = s.assign(&c.id, Some("v-1".into())).unwrap();
        assert_eq!(assigned.status, ConversationStatus::Assigned);
        assert_eq!(assigned.assigned_volunteer_id.as_deref(), Some("v-1"));
        let unassigned = s.assign(&c.id, None).unwrap();
        assert_eq!(unassigned.status, ConversationStatus::New);
        assert!(unassigned.assigned_volunteer_id.is_none());
    }

    #[test]
    fn unassign_volunteer_preserves_history_and_reassigns() {
        let s = empty();
        let c = s.create(NewConversation {
            channel: Channel::WebChat,
            subject: "Test".into(),
            status: ConversationStatus::Assigned,
            contact_id: None,
            case_id: None,
            assigned_volunteer_id: Some("v-9".into()),
        });
        s.append_message(
            &c.id,
            NewMessage {
                channel: Channel::WebChat,
                direction: MessageDirection::Inbound,
                author_kind: AuthorKind::Visitor,
                author_id: None,
                author_label: "Visitor".into(),
                body: "Keep this".into(),
                sources: Vec::new(),
            },
        );
        let moved = s.unassign_volunteer("v-9");
        assert_eq!(moved, 1);
        let thread = s.get(&c.id).unwrap();
        // History survives; ownership is cleared and the thread is back in the queue.
        assert_eq!(thread.messages.len(), 1);
        assert!(thread.conversation.assigned_volunteer_id.is_none());
        assert_eq!(thread.conversation.status, ConversationStatus::New);
    }

    #[test]
    fn list_filters_by_case() {
        let s = empty();
        s.create(NewConversation {
            channel: Channel::WebChat,
            subject: "A".into(),
            status: ConversationStatus::New,
            contact_id: None,
            case_id: Some("c-1".into()),
            assigned_volunteer_id: None,
        });
        s.create(NewConversation {
            channel: Channel::Email,
            subject: "B".into(),
            status: ConversationStatus::New,
            contact_id: None,
            case_id: Some("c-2".into()),
            assigned_volunteer_id: None,
        });
        assert_eq!(s.conversations_for_case("c-1").len(), 1);
        assert_eq!(s.conversations_for_case("c-2").len(), 1);
        assert_eq!(s.conversations_for_case("c-3").len(), 0);
    }
}
