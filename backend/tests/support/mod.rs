use mongodb::Client;
use resolve::config::Config;
use std::sync::OnceLock;
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;

// Shared across every *_api_tests.rs / auth_tests.rs file — some also use it
// directly for minting tokens outside of a Config (e.g. rbac_api_tests.rs's
// jwt::issue_token, auth_tests.rs's issue_token_with_exp), not just via
// test_config below.
pub const TEST_JWT_SECRET: &str = "test-secret";

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

// `#[tokio::test]`/`#[actix_web::test]` each build and tear down their own
// runtime per test function. The mongodb driver spawns its SDAM background
// monitor tasks (see mongodb crate's sdam/monitor.rs) onto whichever runtime
// is current when the `Client` is built, so those tasks die the moment that
// first test's runtime is dropped — leaving a `shared_client()` with no live
// topology monitor for every test after the first. Driving every test on one
// process-wide runtime keeps those tasks alive for the whole binary, the same
// as production (which also runs on a single long-lived runtime).
pub fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build shared test runtime")
    })
}

static CLIENT: OnceCell<Client> = OnceCell::const_new();

// MONGO_URI is a `mongodb+srv://` Atlas connection string, so every fresh
// `Client` does a DNS SRV lookup plus a TLS handshake to each replica before
// it's usable. Each test file's `setup()` used to open one per test (20+ per
// binary), which is slow and trips Atlas's throttling on rapid new-connection
// bursts, surfacing as flaky timeouts on whatever the first operation on that
// connection happened to be. One shared client (and its pool) per test binary
// fixes both.
pub async fn shared_client() -> &'static Client {
    CLIENT
        .get_or_init(|| async {
            dotenvy::dotenv().ok();
            let uri = std::env::var("MONGO_URI").expect("MONGO_URI must be set");
            Client::with_uri_str(&uri)
                .await
                .expect("failed to connect to MongoDB")
        })
        .await
}

// Every *_api_tests.rs file was hand-building this same Config literal
// (field-for-field identical apart from mongo_uri), which meant every new
// required Config field needed editing in 7+ places — this is that one
// place instead. gemini_api_key comes from the real environment (`.ok()`,
// so it's None rather than a panic if unset) rather than a placeholder
// string: only ai_api_tests.rs's #[ignore]-gated live tests actually need a
// working key, and reading the real one here means they get it for free
// without a separate code path.
pub fn test_config(mongo_uri: String) -> Config {
    dotenvy::dotenv().ok();
    Config {
        mongo_uri,
        jwt_secret: TEST_JWT_SECRET.to_string(),
        cookie_secure: false,
        frontend_origin: "http://localhost:5173".to_string(),
        gemini_api_key: std::env::var("GEMINI_API_KEY").ok(),
    }
}
