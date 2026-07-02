use mongodb::{IndexModel, bson::doc, options::IndexOptions};
use resolve::auth::models::{LoginRequest, RegisterRequest};
use resolve::auth::service::AuthService;
use resolve::errors::ApiError;

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
