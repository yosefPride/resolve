use actix_web::cookie::Cookie as ParsedCookie;
use actix_web::{App, test, web};
use mongodb::{
    Database, IndexModel,
    bson::{doc, oid::ObjectId},
    options::IndexOptions,
};
use resolve::auth::models::{AuthResponse, LoginRequest, RefreshResponse, RegisterRequest};
use resolve::auth::refresh_token::REFRESH_TOKEN_COOKIE;
use resolve::auth::service::AuthService;
use resolve::config::Config;
use resolve::errors::ApiError;
use resolve::server::routes;
use resolve::state::AppState;
use resolve::user::models::UserResponse;
use resolve::user::repository::UserRepository;

const TEST_JWT_SECRET: &str = "test-secret";

async fn setup() -> AuthService {
    dotenvy::dotenv().ok();
    let uri = std::env::var("MONGO_URI").expect("MONGO_URI must be set");
    let client = mongodb::Client::with_uri_str(&uri)
        .await
        .expect("failed to connect to MongoDB");
    let db = client.database("resolve_test");

    // Drop and recreate so each run starts from a known clean state.
    db.collection::<mongodb::bson::Document>("users")
        .drop()
        .await
        .expect("failed to drop users collection");
    db.collection::<mongodb::bson::Document>("refresh_tokens")
        .drop()
        .await
        .expect("failed to drop refresh_tokens collection");

    db.collection::<mongodb::bson::Document>("users")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "email": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await
        .expect("failed to create email index");

    AuthService::new(&db, TEST_JWT_SECRET.to_string())
}

// Used by the full-HTTP tests below, which need the raw `Database` (to build
// `AppState`) rather than an `AuthService`. Deliberately does not drop the
// collection like `setup()` does, since it shares "resolve_test" with the
// other tests in this file — it only cleans up its own dedicated email.
async fn setup_db() -> (Database, String) {
    dotenvy::dotenv().ok();
    let uri = std::env::var("MONGO_URI").expect("MONGO_URI must be set");
    let client = mongodb::Client::with_uri_str(&uri)
        .await
        .expect("failed to connect to MongoDB");
    let db = client.database("resolve_test");

    db.collection::<mongodb::bson::Document>("users")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "email": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await
        .expect("failed to create email index");

    (db, uri)
}

fn build_app_state(db: Database, uri: String) -> web::Data<AppState> {
    web::Data::new(AppState {
        db,
        config: Config {
            mongo_uri: uri,
            jwt_secret: TEST_JWT_SECRET.to_string(),
            cookie_secure: false,
            frontend_origin: "http://localhost:5173".to_string(),
        },
    })
}

// Pulls the raw refresh token out of a response's Set-Cookie header, the way
// a real browser would before sending it back on the next request.
fn refresh_cookie_value(
    resp: &actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
) -> Option<String> {
    resp.response()
        .headers()
        .get_all(actix_web::http::header::SET_COOKIE)
        .find_map(|header_value| {
            let raw = header_value.to_str().ok()?;
            let cookie = ParsedCookie::parse(raw.to_string()).ok()?;
            (cookie.name() == REFRESH_TOKEN_COOKIE).then(|| cookie.value().to_string())
        })
}

// 1. Register creates a user and issues a jwt.
#[tokio::test]
async fn test_register_success() {
    let auth = setup().await;
    let (response, raw_refresh_token) = auth
        .register(RegisterRequest {
            email: "register@test.com".to_string(),
            password: "password123".to_string(),
            name: "Register Test".to_string(),
        })
        .await
        .expect("register failed");

    assert_eq!(response.user.email, "register@test.com");
    assert!(!response.jwt.is_empty());
    assert!(!raw_refresh_token.is_empty());
}

// 2. Registering the same email twice returns DuplicateEmail, not a panic.
#[tokio::test]
async fn test_register_duplicate_email() {
    let auth = setup().await;
    let make_input = || RegisterRequest {
        email: "duplicate@test.com".to_string(),
        password: "password123".to_string(),
        name: "Dup".to_string(),
    };

    auth.register(make_input())
        .await
        .expect("first register failed");
    let err = auth
        .register(make_input())
        .await
        .expect_err("expected duplicate email error");

    assert!(matches!(err, ApiError::DuplicateEmail));
}

// 3. Login with correct credentials succeeds and issues a jwt.
#[tokio::test]
async fn test_login_success() {
    let auth = setup().await;
    auth.register(RegisterRequest {
        email: "login@test.com".to_string(),
        password: "correct-password".to_string(),
        name: "Login Test".to_string(),
    })
    .await
    .expect("register failed");

    let (response, raw_refresh_token) = auth
        .login(LoginRequest {
            email: "login@test.com".to_string(),
            password: "correct-password".to_string(),
        })
        .await
        .expect("login failed");

    assert_eq!(response.user.email, "login@test.com");
    assert!(!response.jwt.is_empty());
    assert!(!raw_refresh_token.is_empty());
}

// 4. Login with the wrong password returns InvalidCredentials.
#[tokio::test]
async fn test_login_wrong_password() {
    let auth = setup().await;
    auth.register(RegisterRequest {
        email: "wrongpw@test.com".to_string(),
        password: "correct-password".to_string(),
        name: "Wrong Password".to_string(),
    })
    .await
    .expect("register failed");

    let err = auth
        .login(LoginRequest {
            email: "wrongpw@test.com".to_string(),
            password: "incorrect-password".to_string(),
        })
        .await
        .expect_err("expected invalid credentials error");

    assert!(matches!(err, ApiError::InvalidCredentials));
}

// 5. Login with an unknown email returns InvalidCredentials (not a distinguishable 404).
#[tokio::test]
async fn test_login_unknown_email() {
    let auth = setup().await;
    let err = auth
        .login(LoginRequest {
            email: "nobody@test.com".to_string(),
            password: "whatever123".to_string(),
        })
        .await
        .expect_err("expected invalid credentials error");

    assert!(matches!(err, ApiError::InvalidCredentials));
}

// 6. GET /auth/me, exercised through the real HTTP routing (not just the service
// layer), returns the current user for a valid token and rejects missing/invalid ones.
// Also asserts register sets a refresh-token cookie.
#[actix_web::test]
async fn test_me_endpoint() {
    const EMAIL: &str = "http_me@test.com";
    let (db, uri) = setup_db().await;
    let repo = UserRepository::new(&db);

    if let Some(existing) = repo.find_by_email(EMAIL).await.expect("find failed") {
        repo.delete(existing.id.unwrap())
            .await
            .expect("cleanup delete failed");
    }

    let app_state = build_app_state(db, uri);
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .service(web::scope("/api/v1").configure(routes::configure)),
    )
    .await;

    let register_req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(&RegisterRequest {
            email: EMAIL.to_string(),
            password: "password123".to_string(),
            name: "HTTP Me Test".to_string(),
        })
        .to_request();
    let register_resp = test::call_service(&app, register_req).await;
    assert_eq!(register_resp.status(), 201);
    assert!(
        refresh_cookie_value(&register_resp).is_some(),
        "register should set a refresh_token cookie"
    );
    let register_body: AuthResponse = test::read_body_json(register_resp).await;
    let jwt = register_body.jwt;

    // Valid token returns the current user.
    let me_req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
        .to_request();
    let me_resp = test::call_service(&app, me_req).await;
    assert_eq!(me_resp.status(), 200);
    let me_body: UserResponse = test::read_body_json(me_resp).await;
    assert_eq!(me_body.email, EMAIL);

    // Missing token is rejected.
    let no_token_req = test::TestRequest::get().uri("/api/v1/auth/me").to_request();
    let no_token_resp = test::call_service(&app, no_token_req).await;
    assert_eq!(no_token_resp.status(), 401);

    // Invalid token is rejected.
    let bad_token_req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Authorization", "Bearer not-a-real-token"))
        .to_request();
    let bad_token_resp = test::call_service(&app, bad_token_req).await;
    assert_eq!(bad_token_resp.status(), 401);

    let cleanup_id = ObjectId::parse_str(&me_body.id).unwrap();
    repo.delete(cleanup_id).await.ok();
}

// 7. GET /auth/me rejects a well-formed but expired token (401, not a 500).
#[actix_web::test]
async fn test_me_endpoint_rejects_expired_token() {
    const EMAIL: &str = "http_me_expired@test.com";
    let (db, uri) = setup_db().await;
    let repo = UserRepository::new(&db);

    if let Some(existing) = repo.find_by_email(EMAIL).await.expect("find failed") {
        repo.delete(existing.id.unwrap())
            .await
            .expect("cleanup delete failed");
    }

    let app_state = build_app_state(db, uri);
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .service(web::scope("/api/v1").configure(routes::configure)),
    )
    .await;

    let register_req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(&RegisterRequest {
            email: EMAIL.to_string(),
            password: "password123".to_string(),
            name: "HTTP Me Expired Test".to_string(),
        })
        .to_request();
    let register_resp = test::call_service(&app, register_req).await;
    assert_eq!(register_resp.status(), 201);
    let register_body: AuthResponse = test::read_body_json(register_resp).await;
    let user_id = register_body.user.id.clone();

    let expired_exp = (chrono::Utc::now() - chrono::Duration::hours(1)).timestamp() as usize;
    let expired_token =
        resolve::auth::jwt::issue_token_with_exp(&user_id, TEST_JWT_SECRET, expired_exp)
            .expect("failed to issue expired token");

    let me_req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Authorization", format!("Bearer {expired_token}")))
        .to_request();
    let me_resp = test::call_service(&app, me_req).await;
    assert_eq!(me_resp.status(), 401);

    let cleanup_id = ObjectId::parse_str(&user_id).unwrap();
    repo.delete(cleanup_id).await.ok();
}

// 8. POST /auth/refresh exchanges a valid refresh cookie for a new access
// token and rotates the refresh token — the old one no longer works afterward.
#[actix_web::test]
async fn test_refresh_rotates_token() {
    const EMAIL: &str = "http_refresh@test.com";
    let (db, uri) = setup_db().await;
    let repo = UserRepository::new(&db);

    if let Some(existing) = repo.find_by_email(EMAIL).await.expect("find failed") {
        repo.delete(existing.id.unwrap())
            .await
            .expect("cleanup delete failed");
    }

    let app_state = build_app_state(db, uri);
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .service(web::scope("/api/v1").configure(routes::configure)),
    )
    .await;

    let register_req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(&RegisterRequest {
            email: EMAIL.to_string(),
            password: "password123".to_string(),
            name: "HTTP Refresh Test".to_string(),
        })
        .to_request();
    let register_resp = test::call_service(&app, register_req).await;
    let original_refresh_token =
        refresh_cookie_value(&register_resp).expect("register should set a refresh cookie");
    let register_body: AuthResponse = test::read_body_json(register_resp).await;
    let user_id = register_body.user.id.clone();

    // First refresh succeeds and rotates to a new refresh token.
    let refresh_req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .cookie(ParsedCookie::new(
            REFRESH_TOKEN_COOKIE,
            original_refresh_token.clone(),
        ))
        .to_request();
    let refresh_resp = test::call_service(&app, refresh_req).await;
    assert_eq!(refresh_resp.status(), 200);
    let rotated_refresh_token =
        refresh_cookie_value(&refresh_resp).expect("refresh should rotate the cookie");
    assert_ne!(rotated_refresh_token, original_refresh_token);
    let refresh_body: RefreshResponse = test::read_body_json(refresh_resp).await;
    assert!(!refresh_body.jwt.is_empty());

    // The new access token authenticates.
    let me_req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Authorization", format!("Bearer {}", refresh_body.jwt)))
        .to_request();
    let me_resp = test::call_service(&app, me_req).await;
    assert_eq!(me_resp.status(), 200);

    // Reusing the original (already-rotated) refresh token fails.
    let reuse_req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .cookie(ParsedCookie::new(
            REFRESH_TOKEN_COOKIE,
            original_refresh_token,
        ))
        .to_request();
    let reuse_resp = test::call_service(&app, reuse_req).await;
    assert_eq!(reuse_resp.status(), 401);

    let cleanup_id = ObjectId::parse_str(&user_id).unwrap();
    repo.delete(cleanup_id).await.ok();
}

// 9. POST /auth/refresh without a cookie, or with a garbage one, is rejected.
#[actix_web::test]
async fn test_refresh_rejects_missing_or_invalid_token() {
    let (db, uri) = setup_db().await;
    let app_state = build_app_state(db, uri);
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .service(web::scope("/api/v1").configure(routes::configure)),
    )
    .await;

    let no_cookie_req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .to_request();
    let no_cookie_resp = test::call_service(&app, no_cookie_req).await;
    assert_eq!(no_cookie_resp.status(), 401);

    let bad_cookie_req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .cookie(ParsedCookie::new(REFRESH_TOKEN_COOKIE, "not-a-real-token"))
        .to_request();
    let bad_cookie_resp = test::call_service(&app, bad_cookie_req).await;
    assert_eq!(bad_cookie_resp.status(), 401);
}

// 10. POST /auth/logout revokes only the refresh token for this session — a
// refresh attempt with it afterward fails — but does not invalidate the
// still-unexpired access token already issued (that's the accepted tradeoff
// of stateless, short-lived access tokens: revocation lives at the refresh
// layer, not the access-token layer).
#[actix_web::test]
async fn test_logout_revokes_refresh_token_only() {
    const EMAIL: &str = "http_logout@test.com";
    let (db, uri) = setup_db().await;
    let repo = UserRepository::new(&db);

    if let Some(existing) = repo.find_by_email(EMAIL).await.expect("find failed") {
        repo.delete(existing.id.unwrap())
            .await
            .expect("cleanup delete failed");
    }

    let app_state = build_app_state(db, uri);
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .service(web::scope("/api/v1").configure(routes::configure)),
    )
    .await;

    let register_req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(&RegisterRequest {
            email: EMAIL.to_string(),
            password: "password123".to_string(),
            name: "HTTP Logout Test".to_string(),
        })
        .to_request();
    let register_resp = test::call_service(&app, register_req).await;
    let raw_refresh_token =
        refresh_cookie_value(&register_resp).expect("register should set a refresh cookie");
    let register_body: AuthResponse = test::read_body_json(register_resp).await;
    let access_token = register_body.jwt;
    let user_id = register_body.user.id.clone();

    // Log out with that session's refresh token.
    let logout_req = test::TestRequest::post()
        .uri("/api/v1/auth/logout")
        .cookie(ParsedCookie::new(
            REFRESH_TOKEN_COOKIE,
            raw_refresh_token.clone(),
        ))
        .to_request();
    let logout_resp = test::call_service(&app, logout_req).await;
    assert_eq!(logout_resp.status(), 200);

    // The refresh token no longer works.
    let refresh_req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .cookie(ParsedCookie::new(REFRESH_TOKEN_COOKIE, raw_refresh_token))
        .to_request();
    let refresh_resp = test::call_service(&app, refresh_req).await;
    assert_eq!(refresh_resp.status(), 401);

    // The access token issued before logout is still valid until it expires
    // on its own — logout does not retroactively kill it.
    let me_req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Authorization", format!("Bearer {access_token}")))
        .to_request();
    let me_resp = test::call_service(&app, me_req).await;
    assert_eq!(me_resp.status(), 200);

    let cleanup_id = ObjectId::parse_str(&user_id).unwrap();
    repo.delete(cleanup_id).await.ok();
}

// 11. Logout with no refresh cookie present is a no-op, not an error.
#[actix_web::test]
async fn test_logout_without_cookie_is_noop() {
    let (db, uri) = setup_db().await;
    let app_state = build_app_state(db, uri);
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .service(web::scope("/api/v1").configure(routes::configure)),
    )
    .await;

    let logout_req = test::TestRequest::post()
        .uri("/api/v1/auth/logout")
        .to_request();
    let logout_resp = test::call_service(&app, logout_req).await;
    assert_eq!(logout_resp.status(), 200);
}
