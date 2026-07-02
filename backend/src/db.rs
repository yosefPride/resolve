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
    Ok(())
}
