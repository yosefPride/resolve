use actix_web::{App, test, web};
use mongodb::{
    Database, IndexModel,
    bson::{doc, oid::ObjectId},
    options::IndexOptions,
};
use resolve::auth::models::{AuthResponse, LoginRequest, RegisterRequest};
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

// Used by the full-HTTP test below, which needs the raw `Database` (to build
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

// 1. Register creates a user and issues a jwt.
#[tokio::test]
async fn test_register_success() {
    let auth = setup().await;
    let response = auth
        .register(RegisterRequest {
            email: "register@test.com".to_string(),
            password: "password123".to_string(),
            name: "Register Test".to_string(),
        })
        .await
        .expect("register failed");

    assert_eq!(response.user.email, "register@test.com");
    assert!(!response.jwt.is_empty());
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

    let response = auth
        .login(LoginRequest {
            email: "login@test.com".to_string(),
            password: "correct-password".to_string(),
        })
        .await
        .expect("login failed");

    assert_eq!(response.user.email, "login@test.com");
    assert!(!response.jwt.is_empty());
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

    let app_state = web::Data::new(AppState {
        db,
        config: Config {
            mongo_uri: uri,
            jwt_secret: TEST_JWT_SECRET.to_string(),
        },
    });

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

    let app_state = web::Data::new(AppState {
        db,
        config: Config {
            mongo_uri: uri,
            jwt_secret: TEST_JWT_SECRET.to_string(),
        },
    });

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
        resolve::auth::jwt::issue_token_with_exp(&user_id, 0, TEST_JWT_SECRET, expired_exp)
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

// 8. POST /auth/logout invalidates every token issued before it, while a fresh
// login afterward issues a new, valid token.
#[actix_web::test]
async fn test_logout_invalidates_existing_tokens() {
    const EMAIL: &str = "http_logout@test.com";
    let (db, uri) = setup_db().await;
    let repo = UserRepository::new(&db);

    if let Some(existing) = repo.find_by_email(EMAIL).await.expect("find failed") {
        repo.delete(existing.id.unwrap())
            .await
            .expect("cleanup delete failed");
    }

    let app_state = web::Data::new(AppState {
        db,
        config: Config {
            mongo_uri: uri,
            jwt_secret: TEST_JWT_SECRET.to_string(),
        },
    });

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
    assert_eq!(register_resp.status(), 201);
    let register_body: AuthResponse = test::read_body_json(register_resp).await;
    let old_jwt = register_body.jwt;
    let user_id = register_body.user.id.clone();

    // The token works before logout.
    let me_before_req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Authorization", format!("Bearer {old_jwt}")))
        .to_request();
    let me_before_resp = test::call_service(&app, me_before_req).await;
    assert_eq!(me_before_resp.status(), 200);

    // Log out with that same token.
    let logout_req = test::TestRequest::post()
        .uri("/api/v1/auth/logout")
        .insert_header(("Authorization", format!("Bearer {old_jwt}")))
        .to_request();
    let logout_resp = test::call_service(&app, logout_req).await;
    assert_eq!(logout_resp.status(), 200);

    // The same token is now rejected.
    let me_after_req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Authorization", format!("Bearer {old_jwt}")))
        .to_request();
    let me_after_resp = test::call_service(&app, me_after_req).await;
    assert_eq!(me_after_resp.status(), 401);

    // A fresh login issues a new, valid token.
    let login_req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(&LoginRequest {
            email: EMAIL.to_string(),
            password: "password123".to_string(),
        })
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    assert_eq!(login_resp.status(), 200);
    let login_body: AuthResponse = test::read_body_json(login_resp).await;
    let new_jwt = login_body.jwt;

    let me_new_req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Authorization", format!("Bearer {new_jwt}")))
        .to_request();
    let me_new_resp = test::call_service(&app, me_new_req).await;
    assert_eq!(me_new_resp.status(), 200);

    let cleanup_id = ObjectId::parse_str(&user_id).unwrap();
    repo.delete(cleanup_id).await.ok();
}
