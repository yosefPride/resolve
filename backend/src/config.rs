pub struct Config {
    pub mongo_uri: String,
    pub jwt_secret: String,
    // Whether the refresh-token cookie gets the `Secure` attribute. Defaults
    // to true (required in production); set COOKIE_SECURE=false for local
    // HTTP development, where a real browser would otherwise silently refuse
    // to store a Secure cookie at all.
    pub cookie_secure: bool,
    // Origin the frontend is served from, for CORS. Needed explicitly
    // (rather than a wildcard) because the refresh-token cookie requires
    // credentialed cross-origin requests, which the CORS spec forbids
    // combining with `Access-Control-Allow-Origin: *`.
    pub frontend_origin: String,
    // Gemini API key (docs/ai-integration.md). Optional, unlike mongo_uri/
    // jwt_secret: CLAUDE.md's Core Rules state "AI is advisory (not required
    // for correctness)", so a deployment (or a dev running only
    // auth/group/ticket work) shouldn't be unable to boot at all just
    // because AI isn't configured. Missing key means AiService::new fails
    // per-request (ApiError::Internal) instead — the AI endpoints are down,
    // not the whole backend.
    pub gemini_api_key: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, std::env::VarError> {
        dotenvy::dotenv().ok();
        Ok(Config {
            mongo_uri: std::env::var("MONGO_URI")?,
            jwt_secret: std::env::var("JWT_SECRET")?,
            cookie_secure: std::env::var("COOKIE_SECURE")
                .map(|value| value != "false")
                .unwrap_or(true),
            frontend_origin: std::env::var("FRONTEND_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
            gemini_api_key: std::env::var("GEMINI_API_KEY").ok(),
        })
    }

    pub fn bind_address(&self) -> String {
        "127.0.0.1:8080".to_string()
    }
}
