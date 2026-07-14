use mongodb::{
    Client, Database, IndexModel,
    bson::{Document, doc},
    error::Error,
    options::IndexOptions,
};

use crate::config::Config;

pub async fn connect(config: &Config) -> Result<Client, Error> {
    Client::with_uri_str(&config.mongo_uri).await
}

pub fn database(client: &Client, _config: &Config) -> Database {
    client.database("resolve")
}

// Enforces uniqueness at the database level so duplicate registrations are
// rejected regardless of any race between the app-level email check and insert.
pub async fn ensure_indexes(db: &Database) -> Result<(), Error> {
    db.collection::<Document>("users")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "email": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await?;

    db.collection::<Document>("refresh_tokens")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "token_hash": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await?;

    // Enforces at most one group_members row per (group, user) so add_member's
    // duplicate-membership rejection (GroupRepoError::DuplicateMember) is
    // atomic, without a separate check-then-insert race.
    db.collection::<Document>("group_members")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "group_id": 1, "user_id": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await?;

    // TTL index: MongoDB's background reaper drops a document once its
    // `expires_at` is in the past, so spent/expired refresh tokens are
    // cleaned up automatically without any application-level cron job.
    db.collection::<Document>("refresh_tokens")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "expires_at": 1 })
                .options(
                    IndexOptions::builder()
                        .expire_after(std::time::Duration::from_secs(0))
                        .build(),
                )
                .build(),
        )
        .await?;

    Ok(())
}
