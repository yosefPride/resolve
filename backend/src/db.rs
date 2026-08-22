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
    // first operation) — ping so "connected" below is actually proven.
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

enum IndexKind {
    Plain,
    // Unique indexes double as race guards: the app-level pre-checks
    // (duplicate email/member/link) just avoid hitting them in the common
    // case; the index is what actually rejects a concurrent duplicate.
    Unique,
    // TTL: MongoDB's background reaper drops a document once the indexed
    // field's time is in the past — no application-level cron job.
    Ttl,
}

// One row per index: (collection, keys, kind). The comments say what query
// each index serves; the loop in ensure_indexes builds them all uniformly.
fn index_table() -> Vec<(&'static str, Document, IndexKind)> {
    use IndexKind::*;
    vec![
        // Duplicate registrations rejected regardless of any race between the
        // app-level email check and the insert.
        ("users", doc! { "email": 1 }, Unique),
        ("refresh_tokens", doc! { "token_hash": 1 }, Unique),
        // Spent/expired refresh tokens clean themselves up.
        ("refresh_tokens", doc! { "expires_at": 1 }, Ttl),
        // At most one group_members row per (group, user) — add_member's
        // DuplicateMember rejection, without a check-then-insert race.
        ("group_members", doc! { "group_id": 1, "user_id": 1 }, Unique),
        // "List my groups" lookups filter on user_id alone — the compound
        // index above can't serve them, user_id isn't its prefix.
        ("group_members", doc! { "user_id": 1 }, Plain),
        // The audit-log viewer's two independent filters (?group_id / ?user_id).
        ("admin_audit_log", doc! { "group_id": 1 }, Plain),
        ("admin_audit_log", doc! { "deleted_user_id": 1 }, Plain),
        // Every group-scoped ticket query filters on group_id
        // (docs/database.md, "Multi-Tenancy Rule"), plus the list filters.
        ("tickets", doc! { "group_id": 1 }, Plain),
        ("tickets", doc! { "group_id": 1, "status": 1 }, Plain),
        ("tickets", doc! { "group_id": 1, "created_by": 1 }, Plain),
        // The per-group ticket_number sequence stays unique, in addition to
        // the atomic counter that allocates it (TicketRepository::next_ticket_number).
        ("tickets", doc! { "group_id": 1, "ticket_number": 1 }, Unique),
        // list_by_ticket, plus both cascade deletes (each filter is a prefix).
        ("comments", doc! { "group_id": 1, "ticket_id": 1 }, Plain),
        // has_replies (CommentService::delete_comment's hard-vs-soft check).
        ("comments", doc! { "parent_comment_id": 1 }, Plain),
        // ActivityRepository::list_by_ticket (newest-first) + cascade deletes.
        ("ticket_activity", doc! { "group_id": 1, "ticket_id": 1, "occurred_at": -1 }, Plain),
        // find_latest_for_group — the compound index above can't serve a sort
        // on occurred_at without constraining ticket_id, which sits between.
        ("ticket_activity", doc! { "group_id": 1, "occurred_at": -1 }, Plain),
        // At most one link per (group, source, target, relation); its
        // (group_id, source_ticket_id) prefix also serves list/delete.
        (
            "ticket_links",
            doc! { "group_id": 1, "source_ticket_id": 1, "target_ticket_id": 1, "relation_type": 1 },
            Unique,
        ),
        // The target-side branch of list_by_ticket/delete_by_ticket (a ticket
        // can appear on either side of a link).
        ("ticket_links", doc! { "group_id": 1, "target_ticket_id": 1 }, Plain),
        // At most one reaction per (comment, user) — the guard behind
        // set_reaction's upsert; its prefix serves list/delete_by_comment.
        ("comment_reactions", doc! { "comment_id": 1, "user_id": 1 }, Unique),
        // Cascade deletes (group_id alone is a valid prefix).
        ("comment_reactions", doc! { "group_id": 1, "ticket_id": 1 }, Plain),
        // ReferenceRepository::list_by_ticket + cascade deletes.
        ("ticket_references", doc! { "group_id": 1, "ticket_id": 1 }, Plain),
        // One insight document per ticket (upsert_summary/upsert_analysis key
        // on this pair); also find_insight's lookup path.
        ("ai_ticket_insights", doc! { "group_id": 1, "ticket_id": 1 }, Unique),
        // list_conversation_messages (single conversation, oldest-first).
        ("ai_chat_messages", doc! { "conversation_id": 1, "created_at": 1 }, Plain),
        // Cascade deletes filter by group_id(+ticket_id) directly, not via
        // conversation ids, so a deleted user's conversations still clean up.
        ("ai_chat_messages", doc! { "group_id": 1, "ticket_id": 1 }, Plain),
        // count_recent_user_messages, the chat rate-limit check — covers the
        // equality-on-role/user_id + range-on-created_at query directly.
        ("ai_chat_messages", doc! { "role": 1, "user_id": 1, "created_at": 1 }, Plain),
        // list_conversations (equality prefix, sort field last); also covers
        // the cascade deletes since group_id(+ticket_id) is a prefix.
        (
            "ai_conversations",
            doc! { "group_id": 1, "ticket_id": 1, "user_id": 1, "updated_at": -1 },
            Plain,
        ),
    ]
}

pub async fn ensure_indexes(db: &Database) -> Result<(), Error> {
    for (collection, keys, kind) in index_table() {
        let options = match kind {
            IndexKind::Plain => IndexOptions::builder().build(),
            IndexKind::Unique => IndexOptions::builder().unique(true).build(),
            IndexKind::Ttl => IndexOptions::builder()
                .expire_after(std::time::Duration::from_secs(0))
                .build(),
        };
        db.collection::<Document>(collection)
            .create_index(IndexModel::builder().keys(keys).options(options).build())
            .await?;
    }
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

// One-time wipe of chat messages from the old single-shared-thread-per-ticket
// model (no conversation_id, multiple users' messages interleaved with no
// owner). There's no principled way to split that into per-user
// conversations after the fact (confirmed with user), so rather than migrate
// it, this just drops it — new messages always carry conversation_id, so the
// filter only ever matches leftover pre-migration documents. Idempotent: a
// no-op on every boot after the first.
pub async fn wipe_legacy_chat_messages(db: &Database) -> Result<(), Error> {
    db.collection::<Document>("ai_chat_messages")
        .delete_many(doc! { "conversation_id": { "$exists": false } })
        .await?;
    Ok(())
}
