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

    // Serves list_by_ticket (every comment fetch is scoped to group_id +
    // ticket_id) and both bulk cascade deletes (delete_by_ticket on ticket_id
    // alone, delete_by_group on group_id alone — both are index prefixes here).
    db.collection::<Document>("comments")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "group_id": 1, "ticket_id": 1 })
                .build(),
        )
        .await?;

    // Serves has_replies (CommentService::delete_comment's hard-vs-soft-delete
    // check).
    db.collection::<Document>("comments")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "parent_comment_id": 1 })
                .build(),
        )
        .await?;

    // One insight document per ticket (AiRepository::upsert_summary /
    // upsert_analysis upsert against this pair), so it's unique as well as
    // the lookup path for find_insight.
    db.collection::<Document>("ai_ticket_insights")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "group_id": 1, "ticket_id": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await?;

    // Serves find_latest_report's per-group "most recent" query (sorted
    // descending on generated_at).
    db.collection::<Document>("ai_group_reports")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "group_id": 1, "generated_at": -1 })
                .build(),
        )
        .await?;

    // TTL index: auto-expires a report 30 days after it was generated, same
    // reaper mechanism as refresh_tokens (mongodb's background process drops
    // it once expired), but relative to generated_at rather than an
    // absolute expires_at field. Reports are insert-only, one new document
    // per regeneration (docs/database.md: groups -> ai_group_reports is
    // 1-to-many) — without this, an actively-used group regenerating hourly
    // accumulates history that nothing ever reads (find_latest_report only
    // ever wants the newest one), unbounded. Must be a separate single-field
    // index from the compound one above: MongoDB TTL indexes can't be
    // compound.
    db.collection::<Document>("ai_group_reports")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "generated_at": 1 })
                .options(
                    IndexOptions::builder()
                        .expire_after(std::time::Duration::from_secs(30 * 24 * 60 * 60))
                        .build(),
                )
                .build(),
        )
        .await?;

    // Serves list_chat_messages (every read is scoped to group_id + ticket_id,
    // oldest-first) — same shape as the comments index above.
    db.collection::<Document>("ai_chat_messages")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "group_id": 1, "ticket_id": 1, "created_at": 1 })
                .build(),
        )
        .await?;

    // Serves count_recent_user_messages, the chat rate-limit check: equality
    // on role + user_id, range on created_at — this compound index covers
    // that query directly instead of scanning every message ever sent.
    db.collection::<Document>("ai_chat_messages")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "role": 1, "user_id": 1, "created_at": 1 })
                .build(),
        )
        .await?;

    Ok(())
}

// One-time backfill for tickets created before content_updated_at existed
// (see Ticket's doc comment in ticket/models.rs) — without this, those
// documents have no such field and fail to deserialize entirely, 500ing any
// list/get that touches them. A plain `$set` can't copy another field's
// value, so this needs a pipeline update. Idempotent: the filter only
// matches documents still missing the field, so re-running at the next boot
// is a no-op.
pub async fn backfill_ticket_content_updated_at(db: &Database) -> Result<(), Error> {
    db.collection::<Document>("tickets")
        .update_many(
            doc! { "content_updated_at": { "$exists": false } },
            vec![doc! { "$set": { "content_updated_at": "$updated_at" } }],
        )
        .await?;
    Ok(())
}
