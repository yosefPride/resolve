use std::fmt;

use mongodb::{
    Collection, Database,
    bson::{DateTime as BsonDateTime, Document, doc, oid::ObjectId},
    options::ReturnDocument,
};

use crate::ai::models::{AiGroupReport, AiTicketInsight};

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
}

impl AiRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            insights: db.collection("ai_ticket_insights"),
            reports: db.collection("ai_group_reports"),
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

    // Cascade target for ticket/group deletion, mirroring
    // CommentRepository::delete_by_ticket. Not yet wired into
    // TicketService::delete_ticket — that's cross-module plumbing left for
    // when AiService exists, so an insight can't outlive the ticket it
    // describes.
    pub async fn delete_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<u64, AiRepoError> {
        let result = self
            .insights
            .delete_many(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .await?;
        Ok(result.deleted_count)
    }

    // Cascade target for group deletion, mirroring
    // CommentRepository::delete_by_group. Not yet wired into
    // purge_group_data — same follow-up as delete_by_ticket above.
    pub async fn delete_by_group(&self, group_id: ObjectId) -> Result<u64, AiRepoError> {
        let insights_deleted = self
            .insights
            .delete_many(doc! { "group_id": group_id })
            .await?
            .deleted_count;
        self.reports.delete_many(doc! { "group_id": group_id }).await?;
        Ok(insights_deleted)
    }

    // Most recent report for the group, or None if one has never been
    // generated — used by the report cache's time-window check (later
    // stage).
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
