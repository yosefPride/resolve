use mongodb::{Client, Database, error::Error};

use crate::config::Config;

pub async fn connect(config: &Config) -> Result<Client, Error> {
    Client::with_uri_str(&config.mongo_uri).await
}

pub fn database(client: &Client, _config: &Config) -> Database {
    client.database("resolve")
}
