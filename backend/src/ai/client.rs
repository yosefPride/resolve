use std::future::Future;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::errors::ApiError;

// "-latest" alias rather than a pinned version: gemini-2.0-flash returned 429
// RESOURCE_EXHAUSTED with quota limit 0 on this project (superseded, no free-
// tier quota), while gemini-flash-latest resolves to Google's current flash
// model and works. Using the alias also avoids re-pinning every time Google
// retires a dated model name.
const GEMINI_MODEL: &str = "gemini-flash-latest";
const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisResult {
    pub severity_prediction: String,
    pub suggested_fix: String,
    pub classification: String,
}

// Abstraction over "call an LLM with a ticket and get structured output
// back" so AiService can be tested against a fake instead of hitting the
// real network — same reason TicketService takes a Database handle rather
// than talking to Mongo's wire protocol itself.
// Desugared to `-> impl Future<...> + Send` (rather than plain `async fn`) so
// the returned future is Send — required for it to cross the .await points
// inside an actix-web handler running on Actix's multi-threaded runtime.
pub trait AiProvider {
    fn summarize(
        &self,
        title: &str,
        description: &str,
    ) -> impl Future<Output = Result<String, ApiError>> + Send;

    fn analyze(
        &self,
        title: &str,
        description: &str,
    ) -> impl Future<Output = Result<AnalysisResult, ApiError>> + Send;

    // transcript is a pre-formatted "User: ...\nAssistant: ...\n..." string
    // built by AiService from the last CHAT_HISTORY_LIMIT stored messages
    // (empty string if this is the first message) — same reasoning as
    // stats_summary above: the service builds plain text from its own data,
    // this trait only ever takes/returns strings.
    fn chat(
        &self,
        title: &str,
        description: &str,
        transcript: &str,
        message: &str,
    ) -> impl Future<Output = Result<String, ApiError>> + Send;
}

pub struct GeminiClient {
    http: reqwest::Client,
    api_key: String,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key,
        }
    }

    // response_schema, when set, asks Gemini to return application/json
    // matching that schema instead of free text — used by analyze() to get
    // reliably-parseable structured output instead of scraping prose.
    async fn generate(
        &self,
        prompt: &str,
        response_schema: Option<Value>,
    ) -> Result<String, ApiError> {
        let mut generation_config = json!({});
        if let Some(schema) = response_schema {
            generation_config = json!({
                "responseMimeType": "application/json",
                "responseSchema": schema,
            });
        }
        let request_body = json!({
            "contents": [{ "parts": [{ "text": prompt }] }],
            "generationConfig": generation_config,
        });

        let url = format!("{GEMINI_API_BASE}/{GEMINI_MODEL}:generateContent");
        let response = self
            .http
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .json(&request_body)
            .send()
            .await
            .map_err(|_| ApiError::Internal)?;

        // Gemini error responses (bad key, quota, invalid request) are never
        // surfaced to the client raw (docs/api.md forbids leaking internal
        // errors) — they all collapse to Internal, same as every other
        // external-boundary failure in this codebase (see ApiError's From
        // impls for repo errors).
        if !response.status().is_success() {
            return Err(ApiError::Internal);
        }
        response.text().await.map_err(|_| ApiError::Internal)
    }
}

impl AiProvider for GeminiClient {
    async fn summarize(&self, title: &str, description: &str) -> Result<String, ApiError> {
        let body = self
            .generate(&summarize_prompt(title, description), None)
            .await?;
        parse_summary_response(&body)
    }

    async fn analyze(&self, title: &str, description: &str) -> Result<AnalysisResult, ApiError> {
        let body = self
            .generate(
                &analyze_prompt(title, description),
                Some(analysis_response_schema()),
            )
            .await?;
        parse_analysis_response(&body)
    }

    async fn chat(
        &self,
        title: &str,
        description: &str,
        transcript: &str,
        message: &str,
    ) -> Result<String, ApiError> {
        let body = self
            .generate(&chat_prompt(title, description, transcript, message), None)
            .await?;
        parse_summary_response(&body)
    }
}

fn summarize_prompt(title: &str, description: &str) -> String {
    format!(
        "Summarize the following bug ticket in 1-2 concise, plain-language \
         sentences for a support engineer skimming a queue. Do not repeat \
         the title verbatim.\n\nTitle: {title}\nDescription: {description}"
    )
}

fn analyze_prompt(title: &str, description: &str) -> String {
    format!(
        "You are triaging a bug ticket. Based on the title and description \
         below, provide: a severity estimate (one of \"low\", \"medium\", \
         \"high\", \"critical\"), a short actionable suggested fix, and a \
         short classification label (e.g. \"bug\", \"feature request\", \
         \"performance\", \"security\").\n\nTitle: {title}\nDescription: {description}"
    )
}

// Confirmed with user: the chat is scoped to this ticket (and bug-tracking
// work generally) — not a general-purpose assistant. This instruction is the
// entire enforcement mechanism (there's no separate classifier/filter step),
// so it's stated up front, before the ticket context, and repeated as the
// last line right before the new message so it isn't lost to recency bias
// on a long transcript.
fn chat_prompt(title: &str, description: &str, transcript: &str, message: &str) -> String {
    let history_section = if transcript.is_empty() {
        String::new()
    } else {
        format!("\n\nConversation so far:\n{transcript}")
    };
    format!(
        "You are a support assistant helping with ONE bug ticket in a bug-\
         tracking tool. Only answer questions about this ticket or about \
         triaging/resolving it. If the new message is unrelated to this \
         ticket or to bug-tracking work in general, do not answer it — \
         instead reply that you're here to help with this issue and can't \
         assist with unrelated requests. Use the ticket's title and \
         description as context, and the conversation so far (if any) for \
         continuity. Keep the reply short and conversational, a few \
         sentences at most.\n\n\
         Title: {title}\nDescription: {description}{history_section}\n\n\
         New message (stay in scope): {message}"
    )
}

fn analysis_response_schema() -> Value {
    json!({
        "type": "OBJECT",
        "properties": {
            "severity": { "type": "STRING" },
            "suggested_fix": { "type": "STRING" },
            "classification": { "type": "STRING" },
        },
        "required": ["severity", "suggested_fix", "classification"],
    })
}

// Pulls the model's text out of Gemini's response envelope. Kept separate
// from the two parse_* functions below since both summarize and analyze
// responses share this same outer shape (candidates[0].content.parts[0].text)
// — only what's inside that text differs (plain prose vs. a JSON string).
fn extract_text(body: &str) -> Result<String, ApiError> {
    let value: Value = serde_json::from_str(body).map_err(|_| ApiError::Internal)?;
    value["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(str::to_string)
        .ok_or(ApiError::Internal)
}

fn parse_summary_response(body: &str) -> Result<String, ApiError> {
    let text = extract_text(body)?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(ApiError::Internal);
    }
    Ok(trimmed.to_string())
}

#[derive(Deserialize)]
struct RawAnalysis {
    severity: String,
    suggested_fix: String,
    classification: String,
}

fn parse_analysis_response(body: &str) -> Result<AnalysisResult, ApiError> {
    let text = extract_text(body)?;
    let raw: RawAnalysis = serde_json::from_str(&text).map_err(|_| ApiError::Internal)?;
    if raw.severity.trim().is_empty()
        || raw.suggested_fix.trim().is_empty()
        || raw.classification.trim().is_empty()
    {
        return Err(ApiError::Internal);
    }
    Ok(AnalysisResult {
        severity_prediction: raw.severity,
        suggested_fix: raw.suggested_fix,
        classification: raw.classification,
    })
}
