import { ApiError } from '../errors.js';
import { log } from '../logging.js';
import { trim } from '../rust.js';
import { fromSlice } from '../serde/json.js';
import * as t from '../serde/types.js';

// Pinned, NOT a "-latest" alias: an alias silently changing the quota and
// latency profile out from under us is exactly what this pin prevents.
// gemini-3.5-flash is a full (non-lite) flash model with ~10 RPM on the free
// tier. Re-pin deliberately when upgrading, and re-check the quota.
const GEMINI_MODEL = 'gemini-3.5-flash';
const GEMINI_API_BASE = 'https://generativelanguage.googleapis.com/v1beta/models';
const TARGET = 'resolve::ai::client';

export class GeminiClient {
  constructor(apiKey) {
    this.apiKey = apiKey;
  }

  // responseSchema, when set, asks Gemini for application/json matching that
  // schema instead of free text — used by analyze() for reliably-parseable
  // structured output.
  async generate(prompt, responseSchema) {
    const generationConfig = responseSchema
      ? { responseMimeType: 'application/json', responseSchema }
      : {};
    const requestBody = { contents: [{ parts: [{ text: prompt }] }], generationConfig };

    let response;
    try {
      response = await fetch(`${GEMINI_API_BASE}/${GEMINI_MODEL}:generateContent`, {
        method: 'POST',
        headers: { 'x-goog-api-key': this.apiKey, 'content-type': 'application/json' },
        body: JSON.stringify(requestBody),
      });
    } catch (error) {
      log.error(TARGET, `Gemini request failed to send: ${error}`);
      throw ApiError.internal();
    }

    // Gemini's error body is never surfaced to the client (docs/api.md), but
    // it is logged. 429 (quota exhausted) and 5xx (model overloaded) are
    // transient and worth retrying, so they map to AiUnavailable rather than
    // Internal — and deliberately not RateLimited, which means the caller hit
    // *our* per-user chat quota and is shown verbatim by the frontend.
    if (!response.ok) {
      const body = await response.text().catch(() => '');
      log.error(TARGET, `Gemini returned ${response.status} ${response.statusText} for ${GEMINI_MODEL}: ${body}`);
      throw [429, 500, 502, 503, 504].includes(response.status)
        ? ApiError.aiUnavailable()
        : ApiError.internal();
    }
    try {
      return await response.text();
    } catch (error) {
      log.error(TARGET, `Gemini response body could not be read: ${error}`);
      throw ApiError.internal();
    }
  }

  async summarize(title, description) {
    return parseSummaryResponse(await this.generate(summarizePrompt(title, description), null));
  }

  async analyze(title, description) {
    const body = await this.generate(analyzePrompt(title, description), analysisResponseSchema());
    return parseAnalysisResponse(body);
  }

  // transcript is a pre-formatted "User: ...\nAssistant: ..." string of the
  // last stored messages (empty for the first message).
  async chat(title, description, transcript, message) {
    const body = await this.generate(chatPrompt(title, description, transcript, message), null);
    return parseSummaryResponse(body);
  }
}

function summarizePrompt(title, description) {
  return (
    'Summarize the following bug ticket in 1-2 concise, plain-language ' +
    'sentences for a support engineer skimming a queue. Do not repeat ' +
    `the title verbatim.\n\nTitle: ${title}\nDescription: ${description}`
  );
}

function analyzePrompt(title, description) {
  return (
    'You are triaging a bug ticket. Based on the title and description ' +
    'below, provide: a severity estimate (one of "low", "medium", ' +
    '"high", "critical"), a short actionable suggested fix, and a ' +
    'short classification label (e.g. "bug", "feature request", ' +
    `"performance", "security").\n\nTitle: ${title}\nDescription: ${description}`
  );
}

// The chat is scoped to this ticket (and bug-tracking work generally) — not
// a general-purpose assistant. This instruction is the entire enforcement
// mechanism, so it's stated up front and repeated right before the new
// message so it isn't lost to recency bias on a long transcript.
function chatPrompt(title, description, transcript, message) {
  const historySection = transcript === '' ? '' : `\n\nConversation so far:\n${transcript}`;
  return (
    'You are a support assistant helping with ONE bug ticket in a bug-' +
    'tracking tool. Only answer questions about this ticket or about ' +
    'triaging/resolving it. If the new message is unrelated to this ' +
    'ticket or to bug-tracking work in general, do not answer it — ' +
    "instead reply that you're here to help with this issue and can't " +
    "assist with unrelated requests. Use the ticket's title and " +
    'description as context, and the conversation so far (if any) for ' +
    'continuity. Keep the reply short and conversational, a few ' +
    'sentences at most.\n\n' +
    `Title: ${title}\nDescription: ${description}${historySection}\n\n` +
    `New message (stay in scope): ${message}`
  );
}

function analysisResponseSchema() {
  return {
    type: 'OBJECT',
    properties: {
      severity: { type: 'STRING' },
      suggested_fix: { type: 'STRING' },
      classification: { type: 'STRING' },
    },
    required: ['severity', 'suggested_fix', 'classification'],
  };
}

const isObject = (v) => typeof v === 'object' && v !== null && !Array.isArray(v);
const field = (v, key) => (isObject(v) && Object.hasOwn(v, key) ? v[key] : undefined);
const first = (v) => (Array.isArray(v) ? v[0] : undefined);

// Pulls the model's text out of Gemini's response envelope
// (candidates[0].content.parts[0].text), shared by every call.
function extractText(body) {
  let value;
  try {
    value = JSON.parse(body);
  } catch (error) {
    log.error(TARGET, `Gemini response was not valid JSON: ${error.message}`);
    throw ApiError.internal();
  }
  const text = field(first(field(field(first(field(value, 'candidates')), 'content'), 'parts')), 'text');
  if (typeof text !== 'string') {
    // e.g. a response blocked by a safety filter, or truncated by MAX_TOKENS.
    log.error(TARGET, `Gemini response had no text part: ${body}`);
    throw ApiError.internal();
  }
  return text;
}

function parseSummaryResponse(body) {
  const trimmed = trim(extractText(body));
  if (trimmed === '') throw ApiError.internal();
  return trimmed;
}

const RawAnalysis = t.struct('RawAnalysis', {
  severity: t.string,
  suggested_fix: t.string,
  classification: t.string,
});

function parseAnalysisResponse(body) {
  const text = extractText(body);
  let raw;
  try {
    raw = fromSlice(Buffer.from(text, 'utf8'), RawAnalysis);
  } catch (error) {
    log.error(TARGET, `Gemini analysis JSON did not match the schema: ${error.message}; text was ${text}`);
    throw ApiError.internal();
  }
  if (trim(raw.severity) === '' || trim(raw.suggested_fix) === '' || trim(raw.classification) === '') {
    throw ApiError.internal();
  }
  return {
    severityPrediction: raw.severity,
    suggestedFix: raw.suggested_fix,
    classification: raw.classification,
  };
}
