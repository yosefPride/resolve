use std::future::Future;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::errors::ApiError;

// Pinned, NOT a "-latest" alias. This used to be gemini-flash-latest, chosen
// because the then-current gemini-2.0-flash had a free-tier quota of 0. That
// alias has since drifted onto gemini-3.7-flash, whose free tier allows only
// 5 requests/minute and which frequently returns 503 UNAVAILABLE under load —
// measured at roughly a 75% failure rate, every one of which surfaced as a
// 500 here. An alias silently changing the quota and latency profile out from
// under us is exactly the failure this pin prevents; gemini-3.5-flash is a
// full (non-lite) flash model with ~10 RPM on the free tier. Re-pin
// deliberately when upgrading, and re-check the quota when you do.
const GEMINI_MODEL: &str = "gemini-3.5-flash";
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
            .map_err(|error| {
                log::error!("Gemini request failed to send: {error}");
                ApiError::Internal
            })?;

        // Gemini's error body is still never surfaced to the client raw
        // (docs/api.md forbids leaking internal errors), but it is logged
        // before being discarded. Previously every failure here collapsed to
        // a bare Internal with nothing written anywhere, which made a 500
        // impossible to tell apart from a missing API key (AiService::new) or
        // a malformed-response parse failure further down.
        //
        // 429 (free-tier quota exhausted) and 5xx (model overloaded) are both
        // transient and worth retrying, so they map to AiUnavailable rather
        // than Internal. Deliberately not ApiError::RateLimited: that means
        // the caller hit *our* per-user chat quota (AiService::CHAT_RATE_LIMIT),
        // and the frontend renders the message verbatim
        // (frontend/src/utils/errors.js), so reusing it would make "you have
        // sent too many messages" and "Google is throttling us" read as the
        // same thing to the user.
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            log::error!("Gemini returned {status} for {GEMINI_MODEL}: {body}");
            return Err(match status.as_u16() {
                429 | 500 | 502 | 503 | 504 => ApiError::AiUnavailable,
                _ => ApiError::Internal,
            });
        }
        response.text().await.map_err(|error| {
            log::error!("Gemini response body could not be read: {error}");
            ApiError::Internal
        })
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
    let value: Value = serde_json::from_str(body).map_err(|error| {
        log::error!("Gemini response was not valid JSON: {error}");
        ApiError::Internal
    })?;
    value["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| {
            // Reached when the envelope parses but holds no text part — e.g. a
            // response blocked by a safety filter, or truncated by MAX_TOKENS.
            log::error!("Gemini response had no text part: {body}");
            ApiError::Internal
        })
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
    let raw: RawAnalysis = serde_json::from_str(&text).map_err(|error| {
        log::error!("Gemini analysis JSON did not match the schema: {error}; text was {text}");
        ApiError::Internal
    })?;
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
