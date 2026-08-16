use std::fmt;

use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{self, DateTime as BsonDateTime, doc, oid::ObjectId},
    options::ReturnDocument,
};

use crate::ai::models::{AiConversation, AiTicketInsight, ChatMessage, ChatRole};

#[derive(Debug)]
pub enum AiRepoError {
    Database(mongodb::error::Error),
}

impl fmt::Display for AiRepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiRepoError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for AiRepoError {}

impl From<mongodb::error::Error> for AiRepoError {
    fn from(err: mongodb::error::Error) -> Self {
        AiRepoError::Database(err)
    }
}

pub struct AiRepository {
    insights: Collection<AiTicketInsight>,
    chat_messages: Collection<ChatMessage>,
    conversations: Collection<AiConversation>,
}

impl AiRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            insights: db.collection("ai_ticket_insights"),
            chat_messages: db.collection("ai_chat_messages"),
            conversations: db.collection("ai_conversations"),
        }
    }

    // Filtered on group_id as well as ticket_id, same multi-tenancy pattern as
    // TicketRepository::find_by_id: a ticket_id from another group simply
    // matches nothing.
    pub async fn find_insight(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<Option<AiTicketInsight>, AiRepoError> {
        Ok(self
            .insights
            .find_one(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .await?)
    }

    // Upserts just the summary field group, leaving any existing analysis
    // fields on the same document untouched (see AiTicketInsight's doc
    // comment on why the two field groups are independently tracked).
    pub async fn upsert_summary(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        summary: &str,
        source_updated_at: BsonDateTime,
    ) -> Result<AiTicketInsight, AiRepoError> {
        let now = BsonDateTime::now();
        let insight = self
            .insights
            .find_one_and_update(
                doc! { "group_id": group_id, "ticket_id": ticket_id },
                doc! {
                    "$set": {
                        "summary": summary,
                        "summary_source_updated_at": source_updated_at,
                        "updated_at": now,
                    },
                    "$setOnInsert": {
                        "group_id": group_id,
                        "ticket_id": ticket_id,
                        "created_at": now,
                    },
                },
            )
            .upsert(true)
            .return_document(ReturnDocument::After)
            .await?
            .expect("upsert always returns a document");
        Ok(insight)
    }

    // Upserts just the analysis field group (severity/fix/classification),
    // leaving any existing summary untouched.
    pub async fn upsert_analysis(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        severity_prediction: &str,
        suggested_fix: &str,
        classification: &str,
        source_updated_at: BsonDateTime,
    ) -> Result<AiTicketInsight, AiRepoError> {
        let now = BsonDateTime::now();
        let insight = self
            .insights
            .find_one_and_update(
                doc! { "group_id": group_id, "ticket_id": ticket_id },
                doc! {
                    "$set": {
                        "severity_prediction": severity_prediction,
                        "suggested_fix": suggested_fix,
                        "classification": classification,
                        "analysis_source_updated_at": source_updated_at,
                        "updated_at": now,
                    },
                    "$setOnInsert": {
                        "group_id": group_id,
                        "ticket_id": ticket_id,
                        "created_at": now,
                    },
                },
            )
            .upsert(true)
            .return_document(ReturnDocument::After)
            .await?
            .expect("upsert always returns a document");
        Ok(insight)
    }

    // Cascade target for ticket deletion, mirroring
    // CommentRepository::delete_by_ticket. Called from
    // TicketService::delete_ticket so an insight (or a user's conversations)
    // can't outlive the ticket it describes. chat_messages/conversations are
    // both filtered by group_id+ticket_id directly (not by looking up
    // conversation ids first) so a conversation belonging to a deleted user
    // still gets cleaned up here.
    pub async fn delete_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<u64, AiRepoError> {
        let insights_deleted = self
            .insights
            .delete_many(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .await?
            .deleted_count;
        let messages_deleted = self
            .chat_messages
            .delete_many(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .await?
            .deleted_count;
        let conversations_deleted = self
            .conversations
            .delete_many(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .await?
            .deleted_count;
        Ok(insights_deleted + messages_deleted + conversations_deleted)
    }

    // Cascade target for group deletion, mirroring
    // CommentRepository::delete_by_group. Called from purge_group_data
    // (group/service.rs). Returns the combined count across all three
    // collections — insights, chat messages, and conversations are different
    // documents but the same cascade concern, so callers get one number
    // rather than needing to track three.
    pub async fn delete_by_group(&self, group_id: ObjectId) -> Result<u64, AiRepoError> {
        let insights_deleted = self
            .insights
            .delete_many(doc! { "group_id": group_id })
            .await?
            .deleted_count;
        let messages_deleted = self
            .chat_messages
            .delete_many(doc! { "group_id": group_id })
            .await?
            .deleted_count;
        let conversations_deleted = self
            .conversations
            .delete_many(doc! { "group_id": group_id })
            .await?
            .deleted_count;
        Ok(insights_deleted + messages_deleted + conversations_deleted)
    }

    // Ownership isn't checked here — the service layer already fetched and
    // verified the conversation before calling this, so this is a plain
    // insert scoped by conversation_id/group_id/ticket_id.
    pub async fn create_conversation(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        user_id: ObjectId,
    ) -> Result<AiConversation, AiRepoError> {
        let now = BsonDateTime::now();
        let conversation = AiConversation {
            id: None,
            group_id,
            ticket_id,
            user_id,
            title: None,
            created_at: now,
            updated_at: now,
        };
        let result = self.conversations.insert_one(&conversation).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .expect("insert_one always returns an ObjectId");
        Ok(AiConversation {
            id: Some(id),
            ..conversation
        })
    }

    // Filtered on user_id as well as group_id/ticket_id — ownership is
    // enforced by this query itself, not a post-hoc check, since a
    // conversation list is inherently "my conversations", not "the
    // conversations on this ticket". Most-recently-active first.
    pub async fn list_conversations(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        user_id: ObjectId,
    ) -> Result<Vec<AiConversation>, AiRepoError> {
        let cursor = self
            .conversations
            .find(doc! { "group_id": group_id, "ticket_id": ticket_id, "user_id": user_id })
            .sort(doc! { "updated_at": -1 })
            .await?;
        cursor.try_collect().await.map_err(Into::into)
    }

    // No ownership filter here — this is a single-document lookup by id; the
    // caller (AiService) checks group_id/ticket_id/user_id against the
    // returned conversation itself, the same "fetch then compare" shape as
    // CommentService::delete_comment.
    pub async fn find_conversation(
        &self,
        conversation_id: ObjectId,
    ) -> Result<Option<AiConversation>, AiRepoError> {
        Ok(self
            .conversations
            .find_one(doc! { "_id": conversation_id })
            .await?)
    }

    // Bumps updated_at (drives list_conversations' sort) on every message,
    // and fills in title only the first time — $currentDate/$set on a
    // possibly-null field would overwrite a title on every message otherwise,
    // so the "only if still None" guard lives in the query filter itself.
    pub async fn touch_conversation(
        &self,
        conversation_id: ObjectId,
        title_if_unset: &str,
    ) -> Result<(), AiRepoError> {
        self.conversations
            .update_one(
                doc! { "_id": conversation_id, "title": null },
                doc! { "$set": { "title": title_if_unset } },
            )
            .await?;
        self.conversations
            .update_one(
                doc! { "_id": conversation_id },
                doc! { "$set": { "updated_at": BsonDateTime::now() } },
            )
            .await?;
        Ok(())
    }

    // Oldest-first, same shape as CommentRepository::list_by_ticket — the
    // service takes the tail of this for the transcript it sends to Gemini
    // (see AiService::CHAT_HISTORY_LIMIT), and the frontend renders the whole
    // thing top-to-bottom as-is.
    pub async fn list_conversation_messages(
        &self,
        conversation_id: ObjectId,
    ) -> Result<Vec<ChatMessage>, AiRepoError> {
        let cursor = self
            .chat_messages
            .find(doc! { "conversation_id": conversation_id })
            .sort(doc! { "created_at": 1 })
            .await?;
        cursor.try_collect().await.map_err(Into::into)
    }

    pub async fn insert_chat_message(
        &self,
        conversation_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
        role: ChatRole,
        user_id: Option<ObjectId>,
        content: &str,
    ) -> Result<ChatMessage, AiRepoError> {
        let message = ChatMessage {
            id: None,
            conversation_id,
            group_id,
            ticket_id,
            role,
            user_id,
            content: content.to_string(),
            created_at: BsonDateTime::now(),
        };
        let result = self.chat_messages.insert_one(&message).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .expect("insert_one always returns an ObjectId");
        Ok(ChatMessage {
            id: Some(id),
            ..message
        })
    }

    // Deletes one conversation and its messages — unlike the old
    // clear_chat_messages (which wiped every message on a ticket for
    // everyone), this only ever touches a single conversation_id, already
    // ownership-checked by the service layer before this is called.
    pub async fn delete_conversation(
        &self,
        conversation_id: ObjectId,
    ) -> Result<u64, AiRepoError> {
        let conversation_deleted = self
            .conversations
            .delete_one(doc! { "_id": conversation_id })
            .await?
            .deleted_count;
        let messages_deleted = self
            .chat_messages
            .delete_many(doc! { "conversation_id": conversation_id })
            .await?
            .deleted_count;
        Ok(conversation_deleted + messages_deleted)
    }

    // Counts this user's own chat messages sent since `since`, across every
    // ticket — the rate limit (AiService::CHAT_RATE_LIMIT) is scoped per
    // user, not per ticket, so one person can't dodge it by spreading
    // messages across tickets. role is filtered explicitly rather than
    // relying on user_id alone being absent on assistant messages, so the
    // query stays correct even if that invariant ever changes.
    pub async fn count_recent_user_messages(
        &self,
        user_id: ObjectId,
        since: BsonDateTime,
    ) -> Result<u64, AiRepoError> {
        let role = bson::to_bson(&ChatRole::User).expect("ChatRole always serializes");
        Ok(self
            .chat_messages
            .count_documents(doc! {
                "role": role,
                "user_id": user_id,
                "created_at": { "$gte": since },
            })
            .await?)
    }
}
