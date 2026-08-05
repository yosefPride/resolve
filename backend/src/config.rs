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
    // Gemini API key (docs/ai-integration.md). Required like mongo_uri/
    // jwt_secret rather than optional-with-fallback: AI is a core feature
    // (CLAUDE.md), not a best-effort extra, so a missing key should fail
    // startup loudly instead of letting AI endpoints silently 500 later.
    pub gemini_api_key: String,
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
            gemini_api_key: std::env::var("GEMINI_API_KEY")?,
        })
    }

    pub fn bind_address(&self) -> String {
        "127.0.0.1:8080".to_string()
    }
}
