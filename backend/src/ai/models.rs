use mongodb::bson::{DateTime as BsonDateTime, Document, oid::ObjectId};
use serde::{Deserialize, Serialize};

// API response shapes (ai::service::AiService). `cached` isn't in
// docs/api.md's field list but is cheap to include and directly useful: it's
// what the service-layer tests assert on to prove a cache hit skipped the
// Gemini call, and it doubles as a signal the frontend can show later ("from
// cache" vs. freshly generated) without adding any new capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketSummaryResponse {
    pub summary: String,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketAnalysisResponse {
    pub severity_prediction: String,
    pub suggested_fix: String,
    pub classification: String,
    pub cached: bool,
}

// One document per ticket, upserted in place by summarize/analyze
// (ai::repository::AiRepository::upsert_summary / upsert_analysis) rather than
// inserted fresh each call — docs/database.md's "tickets -> ai_ticket_insights
// (1-to-1 or 1-to-many over time)" resolves to 1-to-1 here, with "over time"
// being this document's fields filling in / refreshing as each action runs.
//
// summary_source_updated_at / analysis_source_updated_at each snapshot the
// ticket's updated_at at the moment that field group was last generated, and
// are tracked separately rather than as one shared fingerprint: summarize and
// analyze are independent calls, so an edit can invalidate one without the
// other having ever run, or vice versa. A field group is fresh iff its own
// value is present AND its source timestamp still matches the ticket's
// current updated_at (see is_summary_fresh / is_analysis_fresh below) — this
// is the mechanism behind docs/ai-integration.md's "cached per ticket... if
// ticket does not change, AI is not re-run".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTicketInsight {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: ObjectId,
    pub ticket_id: ObjectId,
    pub summary: Option<String>,
    pub summary_source_updated_at: Option<BsonDateTime>,
    pub severity_prediction: Option<String>,
    pub suggested_fix: Option<String>,
    pub classification: Option<String>,
    pub analysis_source_updated_at: Option<BsonDateTime>,
    pub created_at: BsonDateTime,
    pub updated_at: BsonDateTime,
}

impl AiTicketInsight {
    pub fn is_summary_fresh(&self, ticket_updated_at: BsonDateTime) -> bool {
        self.summary.is_some() && self.summary_source_updated_at == Some(ticket_updated_at)
    }

    pub fn is_analysis_fresh(&self, ticket_updated_at: BsonDateTime) -> bool {
        self.severity_prediction.is_some()
            && self.suggested_fix.is_some()
            && self.classification.is_some()
            && self.analysis_source_updated_at == Some(ticket_updated_at)
    }
}

// Aggregated group-wide analytics (docs/api.md, "POST /ai/groups/{id}/report").
// report_data is a raw Document rather than a typed struct: its shape is
// decided when the report generator is built (later stage), so this stays
// whatever the service hands it in the meantime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiGroupReport {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: ObjectId,
    pub report_data: Document,
    pub generated_at: BsonDateTime,
    pub generated_by: ObjectId,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insight_at(ts: BsonDateTime) -> AiTicketInsight {
        AiTicketInsight {
            id: Some(ObjectId::new()),
            group_id: ObjectId::new(),
            ticket_id: ObjectId::new(),
            summary: Some("summary text".to_string()),
            summary_source_updated_at: Some(ts),
            severity_prediction: Some("high".to_string()),
            suggested_fix: Some("fix text".to_string()),
            classification: Some("bug".to_string()),
            analysis_source_updated_at: Some(ts),
            created_at: ts,
            updated_at: ts,
        }
    }

    #[test]
    fn summary_fresh_when_timestamp_matches() {
        let ts = BsonDateTime::now();
        let insight = insight_at(ts);
        assert!(insight.is_summary_fresh(ts));
        assert!(insight.is_analysis_fresh(ts));
    }

    #[test]
    fn summary_stale_when_ticket_changed_since() {
        let generated_at = BsonDateTime::from_millis(1_000);
        let ticket_now = BsonDateTime::from_millis(2_000);
        let insight = insight_at(generated_at);
        assert!(!insight.is_summary_fresh(ticket_now));
        assert!(!insight.is_analysis_fresh(ticket_now));
    }

    #[test]
    fn summary_not_fresh_when_never_generated() {
        let ts = BsonDateTime::now();
        let mut insight = insight_at(ts);
        insight.summary = None;
        insight.summary_source_updated_at = None;
        assert!(!insight.is_summary_fresh(ts));
        // Analysis fields are independent of summary being absent.
        assert!(insight.is_analysis_fresh(ts));
    }

    #[test]
    fn analysis_not_fresh_when_only_partially_generated() {
        let ts = BsonDateTime::now();
        let mut insight = insight_at(ts);
        insight.suggested_fix = None;
        assert!(!insight.is_analysis_fresh(ts));
    }
}
