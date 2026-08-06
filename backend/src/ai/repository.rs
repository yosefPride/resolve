use std::fmt;

use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{self, DateTime as BsonDateTime, Document, doc, oid::ObjectId},
    options::ReturnDocument,
};

use crate::ai::models::{AiGroupReport, AiTicketInsight, ChatMessage, ChatRole};

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
    reports: Collection<AiGroupReport>,
    chat_messages: Collection<ChatMessage>,
}

impl AiRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            insights: db.collection("ai_ticket_insights"),
            reports: db.collection("ai_group_reports"),
            chat_messages: db.collection("ai_chat_messages"),
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
    // TicketService::delete_ticket so an insight (or a chat thread) can't
    // outlive the ticket it describes.
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
        Ok(insights_deleted + messages_deleted)
    }

    // Cascade target for group deletion, mirroring
    // CommentRepository::delete_by_group. Called from purge_group_data
    // (group/service.rs). Returns the combined count across all three
    // collections — insights, reports, and chat messages are different
    // documents but the same cascade concern, so callers get one number
    // rather than needing to track three.
    pub async fn delete_by_group(&self, group_id: ObjectId) -> Result<u64, AiRepoError> {
        let insights_deleted = self
            .insights
            .delete_many(doc! { "group_id": group_id })
            .await?
            .deleted_count;
        let reports_deleted = self
            .reports
            .delete_many(doc! { "group_id": group_id })
            .await?
            .deleted_count;
        let messages_deleted = self
            .chat_messages
            .delete_many(doc! { "group_id": group_id })
            .await?
            .deleted_count;
        Ok(insights_deleted + reports_deleted + messages_deleted)
    }

    // Oldest-first, same shape as CommentRepository::list_by_ticket — the
    // service takes the tail of this for the transcript it sends to Gemini
    // (see AiService::CHAT_HISTORY_LIMIT), and the frontend renders the whole
    // thing top-to-bottom as-is.
    pub async fn list_chat_messages(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<Vec<ChatMessage>, AiRepoError> {
        let cursor = self
            .chat_messages
            .find(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .sort(doc! { "created_at": 1 })
            .await?;
        cursor.try_collect().await.map_err(Into::into)
    }

    pub async fn insert_chat_message(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        role: ChatRole,
        user_id: Option<ObjectId>,
        content: &str,
    ) -> Result<ChatMessage, AiRepoError> {
        let message = ChatMessage {
            id: None,
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

    // "New chat": unconditional hard delete of every message on this ticket's
    // thread. There's only ever one ongoing conversation per ticket (no
    // thread/conversation id to scope by), so clearing it is just emptying
    // the whole collection for this (group_id, ticket_id) pair.
    pub async fn clear_chat_messages(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<u64, AiRepoError> {
        let result = self
            .chat_messages
            .delete_many(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .await?;
        Ok(result.deleted_count)
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

    // Most recent report for the group, or None if one has never been
    // generated — used by AiService::generate_group_report's TTL check
    // (AiGroupReport::is_fresh).
    pub async fn find_latest_report(
        &self,
        group_id: ObjectId,
    ) -> Result<Option<AiGroupReport>, AiRepoError> {
        Ok(self
            .reports
            .find_one(doc! { "group_id": group_id })
            .sort(doc! { "generated_at": -1 })
            .await?)
    }

    pub async fn insert_report(
        &self,
        group_id: ObjectId,
        report_data: Document,
        generated_by: ObjectId,
    ) -> Result<AiGroupReport, AiRepoError> {
        let report = AiGroupReport {
            id: None,
            group_id,
            report_data,
            generated_at: BsonDateTime::now(),
            generated_by,
        };
        let result = self.reports.insert_one(&report).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .expect("insert_one always returns an ObjectId");
        Ok(AiGroupReport {
            id: Some(id),
            ..report
        })
    }
}
