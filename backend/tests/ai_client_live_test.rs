// Live smoke test against the real Gemini API — proves GEMINI_API_KEY and
// the request/response shapes in ai::client actually work end to end, which
// the unit tests (canned response fixtures, no network) can't prove on their
// own. Excluded from the default `cargo test` run since it costs a real API
// call and needs network access; run explicitly with:
//   cargo test --test ai_client_live_test -- --ignored --nocapture
use resolve::ai::client::{AiProvider, GeminiClient};

#[test]
#[ignore]
fn summarize_and_analyze_hit_the_real_gemini_api() {
    dotenvy::dotenv().ok();
    let api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
    let client = GeminiClient::new(api_key);

    let runtime = tokio::runtime::Runtime::new().expect("failed to build runtime");
    runtime.block_on(async {
        let summary = client
            .summarize(
                "Login button unresponsive",
                "Clicking the login button on mobile Safari does nothing; no network request fires.",
            )
            .await
            .expect("summarize call failed");
        println!("summary: {summary}");
        assert!(!summary.trim().is_empty());

        let analysis = client
            .analyze(
                "Login button unresponsive",
                "Clicking the login button on mobile Safari does nothing; no network request fires.",
            )
            .await
            .expect("analyze call failed");
        println!("analysis: {analysis:?}");
        assert!(!analysis.severity_prediction.trim().is_empty());
        assert!(!analysis.suggested_fix.trim().is_empty());
        assert!(!analysis.classification.trim().is_empty());
    });
}
