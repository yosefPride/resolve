use mongodb::{
    Client, Database, IndexModel,
    bson::{Document, doc},
    error::Error,
    options::IndexOptions,
};

use crate::config::Config;

pub async fn connect(config: &Config) -> Result<Client, Error> {
    let client = Client::with_uri_str(&config.mongo_uri).await?;
    // with_uri_str doesn't open a connection (the driver connects lazily on
    // first operation) â ping so "connected" below is actually proven.
    client
        .database("resolve")
        .run_command(doc! { "ping": 1 })
        .await?;
    println!("\nSuccessfully connected to MongoDB database 'resolve'");
    Ok(client)
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

    // Serves the "list my groups" lookups (list_memberships_for_user /
    // list_groups_for_user), which filter on user_id alone — the compound
    // (group_id, user_id) index above can't, since user_id isn't its prefix.
    db.collection::<Document>("group_members")
        .create_index(IndexModel::builder().keys(doc! { "user_id": 1 }).build())
        .await?;

    // Serve the audit-log viewer's two filters (GET /admin/audit-log?group_id
    // / ?user_id) — each query hits admin_audit_log on one of these fields.
    // Separate single-field indexes, since the two filters are independent and
    // either may be used alone.
    db.collection::<Document>("admin_audit_log")
        .create_index(IndexModel::builder().keys(doc! { "group_id": 1 }).build())
        .await?;

    db.collection::<Document>("admin_audit_log")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "deleted_user_id": 1 })
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

    // Serves every group-scoped ticket query (docs/database.md, "Multi-Tenancy
    // Rule") — every ticket read/write filters on group_id.
    db.collection::<Document>("tickets")
        .create_index(IndexModel::builder().keys(doc! { "group_id": 1 }).build())
        .await?;

    db.collection::<Document>("tickets")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "group_id": 1, "status": 1 })
                .build(),
        )
        .await?;

    db.collection::<Document>("tickets")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "group_id": 1, "created_by": 1 })
                .build(),
        )
        .await?;

    // Enforces the per-group ticket_number sequence stays unique, in addition
    // to the atomic counter that allocates it (TicketRepository::next_ticket_number).
    db.collection::<Document>("tickets")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "group_id": 1, "ticket_number": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await?;

    Ok(())
}
