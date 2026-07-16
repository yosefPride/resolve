use actix_web::cookie::Cookie as ParsedCookie;
use actix_web::{App, test as actix_test, web};
use mongodb::{
    Database, IndexModel,
    bson::{doc, oid::ObjectId},
    options::IndexOptions,
};
use resolve::auth::models::{
    AuthResponse, LoginRequest, RefreshResponse, RegisterRequest, UpdateMeRequest,
};
use resolve::auth::refresh_token::REFRESH_TOKEN_COOKIE;
use resolve::auth::service::AuthService;
use resolve::config::Config;
use resolve::errors::ApiError;
use resolve::server::routes;
use resolve::state::AppState;
use resolve::user::models::UserResponse;
use resolve::user::repository::UserRepository;

const TEST_JWT_SECRET: &str = "test-secret";

mod support;

async fn setup() -> AuthService {
    let db = support::shared_client().await.database("resolve_test");

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
    let db = support::shared_client().await.database("resolve_test");
    // Only needed to populate Config below (never used to open a connection
    // at request time — see src/config.rs); cheap env lookup, no network.
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
#[test]
fn test_register_success() {
    support::runtime().block_on(async {
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
    });
}

// 2. Registering the same email twice returns DuplicateEmail, not a panic.
#[test]
fn test_register_duplicate_email() {
    support::runtime().block_on(async {
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
    });
}

// 3. Login with correct credentials succeeds and issues a jwt.
#[test]
fn test_login_success() {
    support::runtime().block_on(async {
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
    });
}

// 4. Login with the wrong password returns InvalidCredentials.
#[test]
fn test_login_wrong_password() {
    support::runtime().block_on(async {
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
    });
}

// 5. Login with an unknown email returns InvalidCredentials (not a distinguishable 404).
#[test]
fn test_login_unknown_email() {
    support::runtime().block_on(async {
        let auth = setup().await;
        let err = auth
            .login(LoginRequest {
                email: "nobody@test.com".to_string(),
                password: "whatever123".to_string(),
            })
            .await
            .expect_err("expected invalid credentials error");

        assert!(matches!(err, ApiError::InvalidCredentials));
    });
}

// 6. GET /auth/me, exercised through the real HTTP routing (not just the service
// layer), returns the current user for a valid token and rejects missing/invalid ones.
// Also asserts register sets a refresh-token cookie.
#[test]
fn test_me_endpoint() {
    support::runtime().block_on(async {
        const EMAIL: &str = "http_me@test.com";
        let (db, uri) = setup_db().await;
        let repo = UserRepository::new(&db);

        if let Some(existing) = repo.find_by_email(EMAIL).await.expect("find failed") {
            repo.delete(existing.id.unwrap())
                .await
                .expect("cleanup delete failed");
        }

        let app_state = build_app_state(db, uri);
        let app = actix_test::init_service(
            App::new()
                .app_data(app_state)
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let register_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(&RegisterRequest {
                email: EMAIL.to_string(),
                password: "password123".to_string(),
                name: "HTTP Me Test".to_string(),
            })
            .to_request();
        let register_resp = actix_test::call_service(&app, register_req).await;
        assert_eq!(register_resp.status(), 201);
        assert!(
            refresh_cookie_value(&register_resp).is_some(),
            "register should set a refresh_token cookie"
        );
        let register_body: AuthResponse = actix_test::read_body_json(register_resp).await;
        let jwt = register_body.jwt;

        // Valid token returns the current user.
        let me_req = actix_test::TestRequest::get()
            .uri("/api/v1/auth/me")
            .insert_header(("Authorization", format!("Bearer {jwt}")))
            .to_request();
        let me_resp = actix_test::call_service(&app, me_req).await;
        assert_eq!(me_resp.status(), 200);
        let me_body: UserResponse = actix_test::read_body_json(me_resp).await;
        assert_eq!(me_body.email, EMAIL);

        // Missing token is rejected.
        let no_token_req = actix_test::TestRequest::get()
            .uri("/api/v1/auth/me")
            .to_request();
        let no_token_resp = actix_test::call_service(&app, no_token_req).await;
        assert_eq!(no_token_resp.status(), 401);

        // Invalid token is rejected.
        let bad_token_req = actix_test::TestRequest::get()
            .uri("/api/v1/auth/me")
            .insert_header(("Authorization", "Bearer not-a-real-token"))
            .to_request();
        let bad_token_resp = actix_test::call_service(&app, bad_token_req).await;
        assert_eq!(bad_token_resp.status(), 401);

        let cleanup_id = ObjectId::parse_str(&me_body.id).unwrap();
        repo.delete(cleanup_id).await.ok();
    });
}

// 7. GET /auth/me rejects a well-formed but expired token (401, not a 500).
#[test]
fn test_me_endpoint_rejects_expired_token() {
    support::runtime().block_on(async {
        const EMAIL: &str = "http_me_expired@test.com";
        let (db, uri) = setup_db().await;
        let repo = UserRepository::new(&db);

        if let Some(existing) = repo.find_by_email(EMAIL).await.expect("find failed") {
            repo.delete(existing.id.unwrap())
                .await
                .expect("cleanup delete failed");
        }

        let app_state = build_app_state(db, uri);
        let app = actix_test::init_service(
            App::new()
                .app_data(app_state)
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let register_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(&RegisterRequest {
                email: EMAIL.to_string(),
                password: "password123".to_string(),
                name: "HTTP Me Expired Test".to_string(),
            })
            .to_request();
        let register_resp = actix_test::call_service(&app, register_req).await;
        assert_eq!(register_resp.status(), 201);
        let register_body: AuthResponse = actix_test::read_body_json(register_resp).await;
        let user_id = register_body.user.id.clone();

        let expired_exp = (chrono::Utc::now() - chrono::Duration::hours(1)).timestamp() as usize;
        let expired_token =
            resolve::auth::jwt::issue_token_with_exp(&user_id, TEST_JWT_SECRET, expired_exp)
                .expect("failed to issue expired token");

        let me_req = actix_test::TestRequest::get()
            .uri("/api/v1/auth/me")
            .insert_header(("Authorization", format!("Bearer {expired_token}")))
            .to_request();
        let me_resp = actix_test::call_service(&app, me_req).await;
        assert_eq!(me_resp.status(), 401);

        let cleanup_id = ObjectId::parse_str(&user_id).unwrap();
        repo.delete(cleanup_id).await.ok();
    });
}

// 8. POST /auth/refresh exchanges a valid refresh cookie for a new access
// token and rotates the refresh token — the old one no longer works afterward.
#[test]
fn test_refresh_rotates_token() {
    support::runtime().block_on(async {
        const EMAIL: &str = "http_refresh@test.com";
        let (db, uri) = setup_db().await;
        let repo = UserRepository::new(&db);

        if let Some(existing) = repo.find_by_email(EMAIL).await.expect("find failed") {
            repo.delete(existing.id.unwrap())
                .await
                .expect("cleanup delete failed");
        }

        let app_state = build_app_state(db, uri);
        let app = actix_test::init_service(
            App::new()
                .app_data(app_state)
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let register_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(&RegisterRequest {
                email: EMAIL.to_string(),
                password: "password123".to_string(),
                name: "HTTP Refresh Test".to_string(),
            })
            .to_request();
        let register_resp = actix_test::call_service(&app, register_req).await;
        let original_refresh_token =
            refresh_cookie_value(&register_resp).expect("register should set a refresh cookie");
        let register_body: AuthResponse = actix_test::read_body_json(register_resp).await;
        let user_id = register_body.user.id.clone();

        // First refresh succeeds and rotates to a new refresh token.
        let refresh_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/refresh")
            .cookie(ParsedCookie::new(
                REFRESH_TOKEN_COOKIE,
                original_refresh_token.clone(),
            ))
            .to_request();
        let refresh_resp = actix_test::call_service(&app, refresh_req).await;
        assert_eq!(refresh_resp.status(), 200);
        let rotated_refresh_token =
            refresh_cookie_value(&refresh_resp).expect("refresh should rotate the cookie");
        assert_ne!(rotated_refresh_token, original_refresh_token);
        let refresh_body: RefreshResponse = actix_test::read_body_json(refresh_resp).await;
        assert!(!refresh_body.jwt.is_empty());

        // The new access token authenticates.
        let me_req = actix_test::TestRequest::get()
            .uri("/api/v1/auth/me")
            .insert_header(("Authorization", format!("Bearer {}", refresh_body.jwt)))
            .to_request();
        let me_resp = actix_test::call_service(&app, me_req).await;
        assert_eq!(me_resp.status(), 200);

        // Reusing the original (already-rotated) refresh token fails.
        let reuse_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/refresh")
            .cookie(ParsedCookie::new(
                REFRESH_TOKEN_COOKIE,
                original_refresh_token,
            ))
            .to_request();
        let reuse_resp = actix_test::call_service(&app, reuse_req).await;
        assert_eq!(reuse_resp.status(), 401);

        let cleanup_id = ObjectId::parse_str(&user_id).unwrap();
        repo.delete(cleanup_id).await.ok();
    });
}

// 9. POST /auth/refresh without a cookie, or with a garbage one, is rejected.
#[test]
fn test_refresh_rejects_missing_or_invalid_token() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let app_state = build_app_state(db, uri);
        let app = actix_test::init_service(
            App::new()
                .app_data(app_state)
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let no_cookie_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/refresh")
            .to_request();
        let no_cookie_resp = actix_test::call_service(&app, no_cookie_req).await;
        assert_eq!(no_cookie_resp.status(), 401);

        let bad_cookie_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/refresh")
            .cookie(ParsedCookie::new(REFRESH_TOKEN_COOKIE, "not-a-real-token"))
            .to_request();
        let bad_cookie_resp = actix_test::call_service(&app, bad_cookie_req).await;
        assert_eq!(bad_cookie_resp.status(), 401);
    });
}

// 10. POST /auth/logout revokes only the refresh token for this session — a
// refresh attempt with it afterward fails — but does not invalidate the
// still-unexpired access token already issued (that's the accepted tradeoff
// of stateless, short-lived access tokens: revocation lives at the refresh
// layer, not the access-token layer).
#[test]
fn test_logout_revokes_refresh_token_only() {
    support::runtime().block_on(async {
        const EMAIL: &str = "http_logout@test.com";
        let (db, uri) = setup_db().await;
        let repo = UserRepository::new(&db);

        if let Some(existing) = repo.find_by_email(EMAIL).await.expect("find failed") {
            repo.delete(existing.id.unwrap())
                .await
                .expect("cleanup delete failed");
        }

        let app_state = build_app_state(db, uri);
        let app = actix_test::init_service(
            App::new()
                .app_data(app_state)
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let register_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(&RegisterRequest {
                email: EMAIL.to_string(),
                password: "password123".to_string(),
                name: "HTTP Logout Test".to_string(),
            })
            .to_request();
        let register_resp = actix_test::call_service(&app, register_req).await;
        let raw_refresh_token =
            refresh_cookie_value(&register_resp).expect("register should set a refresh cookie");
        let register_body: AuthResponse = actix_test::read_body_json(register_resp).await;
        let access_token = register_body.jwt;
        let user_id = register_body.user.id.clone();

        // Log out with that session's refresh token.
        let logout_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/logout")
            .cookie(ParsedCookie::new(
                REFRESH_TOKEN_COOKIE,
                raw_refresh_token.clone(),
            ))
            .to_request();
        let logout_resp = actix_test::call_service(&app, logout_req).await;
        assert_eq!(logout_resp.status(), 200);

        // The refresh token no longer works.
        let refresh_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/refresh")
            .cookie(ParsedCookie::new(REFRESH_TOKEN_COOKIE, raw_refresh_token))
            .to_request();
        let refresh_resp = actix_test::call_service(&app, refresh_req).await;
        assert_eq!(refresh_resp.status(), 401);

        // The access token issued before logout is still valid until it expires
        // on its own — logout does not retroactively kill it.
        let me_req = actix_test::TestRequest::get()
            .uri("/api/v1/auth/me")
            .insert_header(("Authorization", format!("Bearer {access_token}")))
            .to_request();
        let me_resp = actix_test::call_service(&app, me_req).await;
        assert_eq!(me_resp.status(), 200);

        let cleanup_id = ObjectId::parse_str(&user_id).unwrap();
        repo.delete(cleanup_id).await.ok();
    });
}

// 11. Logout with no refresh cookie present is a no-op, not an error.
#[test]
fn test_logout_without_cookie_is_noop() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let app_state = build_app_state(db, uri);
        let app = actix_test::init_service(
            App::new()
                .app_data(app_state)
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let logout_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/logout")
            .to_request();
        let logout_resp = actix_test::call_service(&app, logout_req).await;
        assert_eq!(logout_resp.status(), 200);
    });
}

// Removes any leftover row for `email` from a previous run of these tests
// (the HTTP tests share "resolve_test" and only clean their own emails).
async fn delete_user_by_email(repo: &UserRepository, email: &str) {
    if let Some(existing) = repo.find_by_email(email).await.expect("find failed") {
        repo.delete(existing.id.unwrap())
            .await
            .expect("cleanup delete failed");
    }
}

fn register_json(email: &str, name: &str) -> RegisterRequest {
    RegisterRequest {
        email: email.to_string(),
        password: "password123".to_string(),
        name: name.to_string(),
    }
}

// 12. PATCH /auth/me updates the name alone; email is untouched and no
// password is demanded.
#[test]
fn test_update_me_name_only() {
    support::runtime().block_on(async {
        const EMAIL: &str = "http_update_name@test.com";
        let (db, uri) = setup_db().await;
        let repo = UserRepository::new(&db);
        let app_state = build_app_state(db, uri);
        let app = actix_test::init_service(
            App::new()
                .app_data(app_state)
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        delete_user_by_email(&repo, EMAIL).await;
        let register_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(register_json(EMAIL, "Before Rename"))
            .to_request();
        let register_resp = actix_test::call_service(&app, register_req).await;
        assert_eq!(register_resp.status(), 201);
        let register_body: AuthResponse = actix_test::read_body_json(register_resp).await;
        let jwt = register_body.jwt;
        let user_id = register_body.user.id;

        let update_req = actix_test::TestRequest::patch()
            .uri("/api/v1/auth/me")
            .insert_header(("Authorization", format!("Bearer {jwt}")))
            .set_json(UpdateMeRequest {
                name: Some("After Rename".to_string()),
                email: None,
                current_password: None,
            })
            .to_request();
        let update_resp = actix_test::call_service(&app, update_req).await;
        assert_eq!(update_resp.status(), 200);
        let updated: UserResponse = actix_test::read_body_json(update_resp).await;
        assert_eq!(updated.name, "After Rename");
        assert_eq!(updated.email, EMAIL);

        // The change is persisted, not just echoed back.
        let me_req = actix_test::TestRequest::get()
            .uri("/api/v1/auth/me")
            .insert_header(("Authorization", format!("Bearer {jwt}")))
            .to_request();
        let me_body: UserResponse =
            actix_test::read_body_json(actix_test::call_service(&app, me_req).await).await;
        assert_eq!(me_body.name, "After Rename");

        repo.delete(ObjectId::parse_str(&user_id).unwrap()).await.ok();
    });
}

// 13. Changing the email requires the current password: missing → 400,
// wrong → 401, correct → 200 and login works with the new email.
#[test]
fn test_update_me_email_requires_current_password() {
    support::runtime().block_on(async {
        const EMAIL: &str = "http_update_email@test.com";
        const NEW_EMAIL: &str = "http_update_email_new@test.com";
        let (db, uri) = setup_db().await;
        let repo = UserRepository::new(&db);
        delete_user_by_email(&repo, EMAIL).await;
        // The target email may also linger from a previous run of this test.
        delete_user_by_email(&repo, NEW_EMAIL).await;

        let app_state = build_app_state(db, uri);
        let app = actix_test::init_service(
            App::new()
                .app_data(app_state)
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let register_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(register_json(EMAIL, "Email Changer"))
            .to_request();
        let register_resp = actix_test::call_service(&app, register_req).await;
        assert_eq!(register_resp.status(), 201);
        let register_body: AuthResponse = actix_test::read_body_json(register_resp).await;
        let jwt = register_body.jwt;
        let user_id = register_body.user.id;

        let no_password_req = actix_test::TestRequest::patch()
            .uri("/api/v1/auth/me")
            .insert_header(("Authorization", format!("Bearer {jwt}")))
            .set_json(UpdateMeRequest {
                name: None,
                email: Some(NEW_EMAIL.to_string()),
                current_password: None,
            })
            .to_request();
        let no_password_resp = actix_test::call_service(&app, no_password_req).await;
        assert_eq!(no_password_resp.status(), 400);

        let wrong_password_req = actix_test::TestRequest::patch()
            .uri("/api/v1/auth/me")
            .insert_header(("Authorization", format!("Bearer {jwt}")))
            .set_json(UpdateMeRequest {
                name: None,
                email: Some(NEW_EMAIL.to_string()),
                current_password: Some("not-the-password".to_string()),
            })
            .to_request();
        let wrong_password_resp = actix_test::call_service(&app, wrong_password_req).await;
        assert_eq!(wrong_password_resp.status(), 401);

        let update_req = actix_test::TestRequest::patch()
            .uri("/api/v1/auth/me")
            .insert_header(("Authorization", format!("Bearer {jwt}")))
            .set_json(UpdateMeRequest {
                name: None,
                email: Some(NEW_EMAIL.to_string()),
                current_password: Some("password123".to_string()),
            })
            .to_request();
        let update_resp = actix_test::call_service(&app, update_req).await;
        assert_eq!(update_resp.status(), 200);
        let updated: UserResponse = actix_test::read_body_json(update_resp).await;
        assert_eq!(updated.email, NEW_EMAIL);

        // The new email is now the login identity.
        let login_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .set_json(&LoginRequest {
                email: NEW_EMAIL.to_string(),
                password: "password123".to_string(),
            })
            .to_request();
        let login_resp = actix_test::call_service(&app, login_req).await;
        assert_eq!(login_resp.status(), 200);

        repo.delete(ObjectId::parse_str(&user_id).unwrap()).await.ok();
    });
}

// 14. Taking an email that belongs to another user is a 409, same as register.
#[test]
fn test_update_me_duplicate_email() {
    support::runtime().block_on(async {
        const EMAIL_A: &str = "http_update_dup_a@test.com";
        const EMAIL_B: &str = "http_update_dup_b@test.com";
        let (db, uri) = setup_db().await;
        let repo = UserRepository::new(&db);
        let app_state = build_app_state(db, uri);
        let app = actix_test::init_service(
            App::new()
                .app_data(app_state)
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        delete_user_by_email(&repo, EMAIL_A).await;
        delete_user_by_email(&repo, EMAIL_B).await;

        let register_a_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(register_json(EMAIL_A, "Dup A"))
            .to_request();
        let register_a_resp = actix_test::call_service(&app, register_a_req).await;
        assert_eq!(register_a_resp.status(), 201);
        let register_a_body: AuthResponse = actix_test::read_body_json(register_a_resp).await;
        let user_a_id = register_a_body.user.id;

        let register_b_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(register_json(EMAIL_B, "Dup B"))
            .to_request();
        let register_b_resp = actix_test::call_service(&app, register_b_req).await;
        assert_eq!(register_b_resp.status(), 201);
        let register_b_body: AuthResponse = actix_test::read_body_json(register_b_resp).await;
        let jwt_b = register_b_body.jwt;
        let user_b_id = register_b_body.user.id;

        let update_req = actix_test::TestRequest::patch()
            .uri("/api/v1/auth/me")
            .insert_header(("Authorization", format!("Bearer {jwt_b}")))
            .set_json(UpdateMeRequest {
                name: None,
                email: Some(EMAIL_A.to_string()),
                current_password: Some("password123".to_string()),
            })
            .to_request();
        let update_resp = actix_test::call_service(&app, update_req).await;
        assert_eq!(update_resp.status(), 409);

        repo.delete(ObjectId::parse_str(&user_a_id).unwrap()).await.ok();
        repo.delete(ObjectId::parse_str(&user_b_id).unwrap()).await.ok();
    });
}

// 15. PATCH /auth/me input validation: empty body, blank name, and malformed
// email are 400s; no token at all is a 401.
#[test]
fn test_update_me_rejects_invalid_input() {
    support::runtime().block_on(async {
        const EMAIL: &str = "http_update_invalid@test.com";
        let (db, uri) = setup_db().await;
        let repo = UserRepository::new(&db);
        let app_state = build_app_state(db, uri);
        let app = actix_test::init_service(
            App::new()
                .app_data(app_state)
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        delete_user_by_email(&repo, EMAIL).await;
        let register_req = actix_test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(register_json(EMAIL, "Invalid Input"))
            .to_request();
        let register_resp = actix_test::call_service(&app, register_req).await;
        assert_eq!(register_resp.status(), 201);
        let register_body: AuthResponse = actix_test::read_body_json(register_resp).await;
        let jwt = register_body.jwt;
        let user_id = register_body.user.id;

        let invalid_bodies = [
            // Nothing to update.
            UpdateMeRequest {
                name: None,
                email: None,
                current_password: None,
            },
            // Blank name.
            UpdateMeRequest {
                name: Some("   ".to_string()),
                email: None,
                current_password: None,
            },
            // Malformed email.
            UpdateMeRequest {
                name: None,
                email: Some("not-an-email".to_string()),
                current_password: Some("password123".to_string()),
            },
        ];
        for (i, body) in invalid_bodies.into_iter().enumerate() {
            let req = actix_test::TestRequest::patch()
                .uri("/api/v1/auth/me")
                .insert_header(("Authorization", format!("Bearer {jwt}")))
                .set_json(body)
                .to_request();
            let resp = actix_test::call_service(&app, req).await;
            assert_eq!(resp.status(), 400, "expected 400 for invalid body #{i}");
        }

        let no_token_req = actix_test::TestRequest::patch()
            .uri("/api/v1/auth/me")
            .set_json(UpdateMeRequest {
                name: Some("New Name".to_string()),
                email: None,
                current_password: None,
            })
            .to_request();
        let no_token_resp = actix_test::call_service(&app, no_token_req).await;
        assert_eq!(no_token_resp.status(), 401);

        repo.delete(ObjectId::parse_str(&user_id).unwrap()).await.ok();
    });
}
