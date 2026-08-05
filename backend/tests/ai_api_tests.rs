// Full-HTTP-stack tests for the /ai routes. Every test here is #[ignore]'d:
// unlike every other *_api_tests.rs file, the happy-path tests
// (summarize/analyze/report actually succeeding) go through the real
// GeminiClient — real network calls, dependent on GEMINI_API_KEY and
// Gemini's own quota/rate limits — so this file can't run as part of the
// default `cargo test` without making the whole suite's success depend on
// external network state. ai_service_tests.rs already covers the RBAC/cache
// logic against a fake provider for the default run; this file is what
// actually proves the real stack (routing, extractor, RBAC, service,
// network call, response shape) works end to end. Run explicitly with:
//   cargo test --test ai_api_tests -- --ignored --test-threads=1
use actix_web::{App, test as actix_test, web};
use mongodb::{Database, IndexModel, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::ai::models::{GroupReportResponse, TicketAnalysisResponse, TicketSummaryResponse};
use resolve::auth::models::{AuthResponse, RegisterRequest};
use resolve::group::models::{AddMemberRequest, CreateGroupRequest, GroupResponse, Role};
use resolve::group::repository::GroupRepository;
use resolve::server::routes;
use resolve::state::AppState;
use resolve::ticket::models::{CreateTicketRequest, TicketPriority, TicketResponse};
use resolve::user::repository::UserRepository;

mod support;

// Same shared-db convention as tests/ticket_api_tests.rs's setup_db().
async fn setup_db() -> (Database, String) {
    let db = support::shared_client().await.database("resolve_test");
    let uri = std::env::var("MONGO_URI").expect("MONGO_URI must be set");

    db.collection::<mongodb::bson::Document>("users")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "email": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await
        .expect("failed to create email index");

    db.collection::<mongodb::bson::Document>("group_members")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "group_id": 1, "user_id": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await
        .expect("failed to create group_members compound index");

    (db, uri)
}

// Unlike the other *_api_tests.rs files, this one needs a real GEMINI_API_KEY
// for its happy-path tests (member calls summarize/analyze/report and
// actually gets a result), same as ai_client_live_test.rs. support::test_config
// reads the real key from the environment, so nothing special is needed here
// — if it's unset, those specific tests fail with a 500 rather than this
// function panicking at setup.
fn build_app_state(db: Database, uri: String) -> web::Data<AppState> {
    web::Data::new(AppState {
        db,
        config: support::test_config(uri),
    })
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@test.com", ObjectId::new())
}

fn register_request(prefix: &str) -> actix_web::test::TestRequest {
    actix_test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(&RegisterRequest {
            email: unique_email(prefix),
            password: "password123".to_string(),
            name: prefix.to_string(),
        })
}

fn auth_header(jwt: &str) -> (&'static str, String) {
    ("Authorization", format!("Bearer {jwt}"))
}

// Cleans up a group's members/doc and the given users, same tolerance
// ticket_api_tests.rs/group_api_tests.rs already have for the shared db.
async fn cleanup(group_repo: &GroupRepository, user_repo: &UserRepository, group_id: ObjectId, users: &[&AuthResponse]) {
    group_repo.delete_members_by_group(group_id).await.ok();
    group_repo.delete_group(group_id).await.ok();
    for user in users {
        user_repo
            .delete(ObjectId::parse_str(&user.user.id).unwrap())
            .await
            .ok();
    }
}

macro_rules! test_app {
    ($state:expr) => {
        actix_test::init_service(
            App::new()
                .app_data($state)
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await
    };
}

// Registers an owner (Group Admin) and a Contributor, creates a group, adds
// the Contributor, and seeds one ticket — the common fixture every test below
// needs. Returns (group_id, ticket_id, owner, contributor).
macro_rules! seed {
    ($app:expr) => {{
        let owner: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&$app, register_request("ai-owner").to_request()).await,
        )
        .await;
        let contributor: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&$app, register_request("ai-contributor").to_request()).await,
        )
        .await;

        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &$app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "AI Test Group".to_string(),
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        assert_eq!(
            actix_test::call_service(
                &$app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/users", group.id))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&AddMemberRequest {
                        user_id: contributor.user.id.clone(),
                        role: Role::Contributor,
                    })
                    .to_request(),
            )
            .await
            .status(),
            201
        );

        let ticket: TicketResponse = actix_test::read_body_json(
            actix_test::call_service(
                &$app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/tickets", group.id))
                    .insert_header(auth_header(&contributor.jwt))
                    .set_json(&CreateTicketRequest {
                        title: "Login button unresponsive".to_string(),
                        description: "Tapping login on mobile Safari does nothing.".to_string(),
                        priority: TicketPriority::High,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        (group.id, ticket.id, owner, contributor)
    }};
}

// 1. A member (Contributor) can summarize a ticket; the real Gemini call
// succeeds and the response is a non-empty, uncached summary. A second call
// for the same unchanged ticket is served from cache.
#[test]
#[ignore]
fn test_summarize_member_succeeds_then_caches() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = test_app!(build_app_state(db, uri));

        let (group_id, ticket_id, owner, contributor) = seed!(app);

        let resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!(
                    "/api/v1/ai/groups/{group_id}/tickets/{ticket_id}/summarize"
                ))
                .insert_header(auth_header(&contributor.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 200);
        let body: TicketSummaryResponse = actix_test::read_body_json(resp).await;
        assert!(!body.summary.trim().is_empty());
        assert!(!body.cached);

        let second = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!(
                    "/api/v1/ai/groups/{group_id}/tickets/{ticket_id}/summarize"
                ))
                .insert_header(auth_header(&contributor.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(second.status(), 200);
        let second_body: TicketSummaryResponse = actix_test::read_body_json(second).await;
        assert!(second_body.cached);
        assert_eq!(second_body.summary, body.summary);

        cleanup(
            &group_repo,
            &user_repo,
            ObjectId::parse_str(&group_id).unwrap(),
            &[&owner, &contributor],
        )
        .await;
    });
}

// 2. A member can analyze a ticket; the real Gemini call succeeds and every
// field comes back non-empty.
#[test]
#[ignore]
fn test_analyze_member_succeeds() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = test_app!(build_app_state(db, uri));

        let (group_id, ticket_id, owner, contributor) = seed!(app);

        let resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!(
                    "/api/v1/ai/groups/{group_id}/tickets/{ticket_id}/analyze"
                ))
                .insert_header(auth_header(&contributor.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 200);
        let body: TicketAnalysisResponse = actix_test::read_body_json(resp).await;
        assert!(!body.severity_prediction.trim().is_empty());
        assert!(!body.suggested_fix.trim().is_empty());
        assert!(!body.classification.trim().is_empty());
        assert!(!body.cached);

        cleanup(
            &group_repo,
            &user_repo,
            ObjectId::parse_str(&group_id).unwrap(),
            &[&owner, &contributor],
        )
        .await;
    });
}

// 3. A non-member is forbidden, and never reaches the Gemini call (so this
// doesn't depend on the API key at all).
#[test]
#[ignore]
fn test_summarize_non_member_forbidden() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = test_app!(build_app_state(db, uri));

        let (group_id, ticket_id, owner, contributor) = seed!(app);
        let outsider: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("ai-outsider").to_request()).await,
        )
        .await;

        let resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!(
                    "/api/v1/ai/groups/{group_id}/tickets/{ticket_id}/summarize"
                ))
                .insert_header(auth_header(&outsider.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 403);

        cleanup(
            &group_repo,
            &user_repo,
            ObjectId::parse_str(&group_id).unwrap(),
            &[&owner, &contributor, &outsider],
        )
        .await;
    });
}

// 4. No Authorization header at all is rejected before any RBAC or provider
// logic runs.
#[test]
#[ignore]
fn test_summarize_requires_authentication() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = test_app!(build_app_state(db, uri));

        let (group_id, ticket_id, owner, contributor) = seed!(app);

        let resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!(
                    "/api/v1/ai/groups/{group_id}/tickets/{ticket_id}/summarize"
                ))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 401);

        cleanup(
            &group_repo,
            &user_repo,
            ObjectId::parse_str(&group_id).unwrap(),
            &[&owner, &contributor],
        )
        .await;
    });
}

// 5. A ticket_id that doesn't belong to the group (or doesn't exist at all)
// 404s rather than leaking whether it exists elsewhere.
#[test]
#[ignore]
fn test_summarize_unknown_ticket_not_found() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = test_app!(build_app_state(db, uri));

        let (group_id, _ticket_id, owner, contributor) = seed!(app);

        let resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!(
                    "/api/v1/ai/groups/{group_id}/tickets/{}/summarize",
                    ObjectId::new()
                ))
                .insert_header(auth_header(&contributor.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 404);

        cleanup(
            &group_repo,
            &user_repo,
            ObjectId::parse_str(&group_id).unwrap(),
            &[&owner, &contributor],
        )
        .await;
    });
}

// 6. An invalid (non-ObjectId) ticket_id is a 400, not a 500 or a panic.
#[test]
#[ignore]
fn test_summarize_invalid_ticket_id_is_bad_request() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = test_app!(build_app_state(db, uri));

        let (group_id, _ticket_id, owner, contributor) = seed!(app);

        let resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!(
                    "/api/v1/ai/groups/{group_id}/tickets/not-an-id/summarize"
                ))
                .insert_header(auth_header(&contributor.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 400);

        cleanup(
            &group_repo,
            &user_repo,
            ObjectId::parse_str(&group_id).unwrap(),
            &[&owner, &contributor],
        )
        .await;
    });
}

// 7. A Group Admin can generate a group report; the real Gemini call
// succeeds and a second call within the TTL is served from cache.
#[test]
#[ignore]
fn test_report_group_admin_succeeds_then_caches() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = test_app!(build_app_state(db, uri));

        let (group_id, _ticket_id, owner, contributor) = seed!(app);

        let resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!("/api/v1/ai/groups/{group_id}/report"))
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 200);
        let body: GroupReportResponse = actix_test::read_body_json(resp).await;
        assert_eq!(body.data.total_tickets, 1);
        assert!(!body.data.narrative.trim().is_empty());
        assert!(!body.cached);

        let second = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!("/api/v1/ai/groups/{group_id}/report"))
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(second.status(), 200);
        let second_body: GroupReportResponse = actix_test::read_body_json(second).await;
        assert!(second_body.cached);
        assert_eq!(second_body.data, body.data);

        cleanup(
            &group_repo,
            &user_repo,
            ObjectId::parse_str(&group_id).unwrap(),
            &[&owner, &contributor],
        )
        .await;
    });
}

// 8. A Contributor is forbidden from generating a group report.
#[test]
#[ignore]
fn test_report_contributor_forbidden() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = test_app!(build_app_state(db, uri));

        let (group_id, _ticket_id, owner, contributor) = seed!(app);

        let resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!("/api/v1/ai/groups/{group_id}/report"))
                .insert_header(auth_header(&contributor.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 403);

        cleanup(
            &group_repo,
            &user_repo,
            ObjectId::parse_str(&group_id).unwrap(),
            &[&owner, &contributor],
        )
        .await;
    });
}
