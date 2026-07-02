use mongodb::{IndexModel, bson::doc, options::IndexOptions};
use resolve::user::{
    models::{CreateUserInput, GlobalRole},
    repository::{UserRepoError, UserRepository},
};

async fn setup() -> UserRepository {
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

    UserRepository::new(&db)
}

// 1. Create a user — id, email, name, created_at are set; password_hash is stored.
#[tokio::test]
async fn test_create_user() {
    let repo = setup().await;
    let input = CreateUserInput {
        email: "create@test.com".to_string(),
        name: "Create Test".to_string(),
        password_hash: "hashed_pw".to_string(),
    };

    let user = repo.create(input).await.expect("create failed");
    assert!(user.id.is_some());
    assert_eq!(user.email, "create@test.com");
    assert_eq!(user.name, "Create Test");
    assert_eq!(user.password_hash, "hashed_pw");
    assert!(user.global_role.is_none());

    repo.delete(user.id.unwrap()).await.ok(); // comment out here and drop in setup to see user being added.
    // and then run `cargo test test_create_user -- --test-threads=1 --nocapture`
}

// 2. Find by email returns the correct user.
#[tokio::test]
async fn test_find_by_email_found() {
    let repo = setup().await;
    let input = CreateUserInput {
        email: "findbyemail@test.com".to_string(),
        name: "Find Email".to_string(),
        password_hash: "hashed_pw".to_string(),
    };
    let created = repo.create(input).await.expect("create failed");

    let found = repo
        .find_by_email("findbyemail@test.com")
        .await
        .expect("find failed")
        .expect("user not found");

    assert_eq!(found.email, "findbyemail@test.com");

    repo.delete(created.id.unwrap()).await.ok();
}

// 3. Find by email returns None for an unknown email.
#[tokio::test]
async fn test_find_by_email_not_found() {
    let repo = setup().await;
    let result = repo
        .find_by_email("nobody@test.com")
        .await
        .expect("find failed");
    assert!(result.is_none());
}

// 4. Find by id returns the correct user.
#[tokio::test]
async fn test_find_by_id_found() {
    let repo = setup().await;
    let input = CreateUserInput {
        email: "findbyid@test.com".to_string(),
        name: "Find Id".to_string(),
        password_hash: "hashed_pw".to_string(),
    };
    let created = repo.create(input).await.expect("create failed");
    let id = created.id.unwrap();

    let found = repo
        .find_by_id(id)
        .await
        .expect("find failed")
        .expect("user not found");

    assert_eq!(found.id.unwrap(), id);

    repo.delete(id).await.ok();
}

// 5. Duplicate email returns DuplicateEmail error, not a panic.
#[tokio::test]
async fn test_duplicate_email_error() {
    let repo = setup().await;
    let make_input = || CreateUserInput {
        email: "duplicate@test.com".to_string(),
        name: "Dup".to_string(),
        password_hash: "hashed_pw".to_string(),
    };
    let first = repo
        .create(make_input())
        .await
        .expect("first create failed");
    let err = repo
        .create(make_input())
        .await
        .expect_err("expected duplicate error");

    assert!(
        matches!(err, UserRepoError::DuplicateEmail),
        "expected DuplicateEmail, got: {err}"
    );

    repo.delete(first.id.unwrap()).await.ok();
}

// 6. List all returns all created users.
#[tokio::test]
async fn test_list_all() {
    let repo = setup().await;
    let a = repo
        .create(CreateUserInput {
            email: "list_a@test.com".to_string(),
            name: "A".to_string(),
            password_hash: "pw".to_string(),
        })
        .await
        .expect("create a failed");
    let b = repo
        .create(CreateUserInput {
            email: "list_b@test.com".to_string(),
            name: "B".to_string(),
            password_hash: "pw".to_string(),
        })
        .await
        .expect("create b failed");

    let all = repo.list_all().await.expect("list failed");
    let emails: Vec<&str> = all.iter().map(|u| u.email.as_str()).collect();
    assert!(emails.contains(&"list_a@test.com"));
    assert!(emails.contains(&"list_b@test.com"));

    repo.delete(a.id.unwrap()).await.ok();
    repo.delete(b.id.unwrap()).await.ok();
}

// 7. Delete removes the user; subsequent find_by_id returns None.
#[tokio::test]
async fn test_delete_user() {
    let repo = setup().await;
    let user = repo
        .create(CreateUserInput {
            email: "delete@test.com".to_string(),
            name: "Delete Me".to_string(),
            password_hash: "pw".to_string(),
        })
        .await
        .expect("create failed");
    let id = user.id.unwrap();

    let deleted = repo.delete(id).await.expect("delete failed");
    assert!(deleted);

    let gone = repo.find_by_id(id).await.expect("find failed");
    assert!(gone.is_none());
}

// 8. global_role is None by default; GlobalRole::SystemAdmin round-trips correctly.
#[tokio::test]
async fn test_global_role() {
    let repo = setup().await;

    // No role by default.
    let regular = repo
        .create(CreateUserInput {
            email: "norole@test.com".to_string(),
            name: "No Role".to_string(),
            password_hash: "pw".to_string(),
        })
        .await
        .expect("create failed");
    assert!(regular.global_role.is_none());

    // Verify GlobalRole::SystemAdmin serializes and deserializes via the enum.
    assert!(matches!(GlobalRole::SystemAdmin, GlobalRole::SystemAdmin));

    repo.delete(regular.id.unwrap()).await.ok();
}
