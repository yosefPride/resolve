use mongodb::{Database, bson::oid::ObjectId};

use crate::ai::client::{AiProvider, GeminiClient};
use crate::ai::models::{TicketAnalysisResponse, TicketSummaryResponse};
use crate::ai::repository::AiRepository;
use crate::config::Config;
use crate::errors::ApiError;
use crate::rbac::service::RbacService;
use crate::ticket::repository::TicketRepository;

// Generic over the provider (default GeminiClient) rather than a concrete
// GeminiClient field: production call sites (AiService::new) are unaffected
// by the default type parameter, while tests can build an
// AiService<FakeProvider> via with_provider to exercise the cache/RBAC logic
// below without ever touching the network — same reasoning as AiProvider's
// doc comment on why it exists at all.
pub struct AiService<P: AiProvider = GeminiClient> {
    repo: AiRepository,
    ticket_repo: TicketRepository,
    rbac: RbacService,
    provider: P,
}

impl AiService<GeminiClient> {
    pub fn new(db: &Database, config: &Config) -> Self {
        Self::with_provider(db, GeminiClient::new(config.gemini_api_key.clone()))
    }
}

impl<P: AiProvider> AiService<P> {
    pub fn with_provider(db: &Database, provider: P) -> Self {
        Self {
            repo: AiRepository::new(db),
            ticket_repo: TicketRepository::new(db),
            rbac: RbacService::new(db),
            provider,
        }
    }

    // docs/api.md: "POST /ai/groups/{id}/tickets/{ticket_id}/summarize —
    // Group-scoped (member of {id} required)".
    pub async fn summarize_ticket(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<TicketSummaryResponse, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        let ticket = self
            .ticket_repo
            .find_by_id(group_id, ticket_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        let cached = self.repo.find_insight(group_id, ticket_id).await?;
        if let Some(insight) = &cached
            && insight.is_summary_fresh(ticket.content_updated_at)
        {
            return Ok(TicketSummaryResponse {
                summary: insight.summary.clone().expect("is_summary_fresh implies Some"),
                cached: true,
            });
        }

        let summary = self
            .provider
            .summarize(&ticket.title, &ticket.description)
            .await?;
        self.repo
            .upsert_summary(group_id, ticket_id, &summary, ticket.content_updated_at)
            .await?;
        Ok(TicketSummaryResponse {
            summary,
            cached: false,
        })
    }

    // docs/api.md: "POST /ai/groups/{id}/tickets/{ticket_id}/analyze —
    // Group-scoped (member of {id} required)".
    pub async fn analyze_ticket(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<TicketAnalysisResponse, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        let ticket = self
            .ticket_repo
            .find_by_id(group_id, ticket_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        let cached = self.repo.find_insight(group_id, ticket_id).await?;
        if let Some(insight) = &cached
            && insight.is_analysis_fresh(ticket.content_updated_at)
        {
            return Ok(TicketAnalysisResponse {
                severity_prediction: insight
                    .severity_prediction
                    .clone()
                    .expect("is_analysis_fresh implies Some"),
                suggested_fix: insight
                    .suggested_fix
                    .clone()
                    .expect("is_analysis_fresh implies Some"),
                classification: insight
                    .classification
                    .clone()
                    .expect("is_analysis_fresh implies Some"),
                cached: true,
            });
        }

        let analysis = self
            .provider
            .analyze(&ticket.title, &ticket.description)
            .await?;
        self.repo
            .upsert_analysis(
                group_id,
                ticket_id,
                &analysis.severity_prediction,
                &analysis.suggested_fix,
                &analysis.classification,
                ticket.content_updated_at,
            )
            .await?;
        Ok(TicketAnalysisResponse {
            severity_prediction: analysis.severity_prediction,
            suggested_fix: analysis.suggested_fix,
            classification: analysis.classification,
            cached: false,
        })
    }
}
