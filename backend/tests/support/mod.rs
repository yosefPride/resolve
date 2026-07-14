use mongodb::Client;
use std::sync::OnceLock;
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;

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
