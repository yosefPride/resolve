import { MongoClient } from 'mongodb';

export async function connect(config) {
  const client = new MongoClient(config.mongoUri);
  // The driver connects lazily on first operation — ping so "connected"
  // below is actually proven.
  await client.db(config.mongoDb).command({ ping: 1 });
  console.log(`\nSuccessfully connected to MongoDB database '${config.mongoDb}'`);
  return client;
}

export function database(client, config) {
  return client.db(config.mongoDb);
}

// One row per index: [collection, keys, kind]. The comments say what query
// each index serves; ensureIndexes builds them all uniformly.
//   plain  — ordinary index
//   unique — doubles as a race guard: the app-level pre-checks (duplicate
//            email/member/link) just avoid hitting it in the common case
//   ttl    — MongoDB's background reaper drops a document once the indexed
//            field's time is in the past
const INDEXES = [
  // Duplicate registrations rejected regardless of any race between the
  // app-level email check and the insert.
  ['users', { email: 1 }, 'unique'],
  ['refresh_tokens', { token_hash: 1 }, 'unique'],
  // Spent/expired refresh tokens clean themselves up.
  ['refresh_tokens', { expires_at: 1 }, 'ttl'],
  // At most one group_members row per (group, user).
  ['group_members', { group_id: 1, user_id: 1 }, 'unique'],
  // "List my groups" lookups filter on user_id alone.
  ['group_members', { user_id: 1 }, 'plain'],
  // The audit-log viewer's two independent filters (?group_id / ?user_id).
  ['admin_audit_log', { group_id: 1 }, 'plain'],
  ['admin_audit_log', { deleted_user_id: 1 }, 'plain'],
  // Every group-scoped ticket query filters on group_id, plus the list filters.
  ['tickets', { group_id: 1 }, 'plain'],
  ['tickets', { group_id: 1, status: 1 }, 'plain'],
  ['tickets', { group_id: 1, created_by: 1 }, 'plain'],
  // The per-group ticket_number sequence stays unique, in addition to the
  // atomic counter that allocates it.
  ['tickets', { group_id: 1, ticket_number: 1 }, 'unique'],
  // listByTicket, plus both cascade deletes (each filter is a prefix).
  ['comments', { group_id: 1, ticket_id: 1 }, 'plain'],
  // hasReplies (CommentService.deleteComment's hard-vs-soft check).
  ['comments', { parent_comment_id: 1 }, 'plain'],
  // ActivityRepository.listByTicket (newest-first) + cascade deletes.
  ['ticket_activity', { group_id: 1, ticket_id: 1, occurred_at: -1 }, 'plain'],
  // findLatestForGroup — the compound index above can't serve a sort on
  // occurred_at without constraining ticket_id, which sits between.
  ['ticket_activity', { group_id: 1, occurred_at: -1 }, 'plain'],
  // At most one link per (group, source, target, relation); its
  // (group_id, source_ticket_id) prefix also serves list/delete.
  [
    'ticket_links',
    { group_id: 1, source_ticket_id: 1, target_ticket_id: 1, relation_type: 1 },
    'unique',
  ],
  // The target-side branch of listByTicket/deleteByTicket.
  ['ticket_links', { group_id: 1, target_ticket_id: 1 }, 'plain'],
  // At most one reaction per (comment, user); its prefix serves list/delete.
  ['comment_reactions', { comment_id: 1, user_id: 1 }, 'unique'],
  // Cascade deletes (group_id alone is a valid prefix).
  ['comment_reactions', { group_id: 1, ticket_id: 1 }, 'plain'],
  // ReferenceRepository.listByTicket + cascade deletes.
  ['ticket_references', { group_id: 1, ticket_id: 1 }, 'plain'],
  // One insight document per ticket (upsertSummary/upsertAnalysis key on this
  // pair); also findInsight's lookup path.
  ['ai_ticket_insights', { group_id: 1, ticket_id: 1 }, 'unique'],
  // listConversationMessages (single conversation, oldest-first).
  ['ai_chat_messages', { conversation_id: 1, created_at: 1 }, 'plain'],
  // Cascade deletes filter by group_id(+ticket_id) directly, not via
  // conversation ids, so a deleted user's conversations still clean up.
  ['ai_chat_messages', { group_id: 1, ticket_id: 1 }, 'plain'],
  // countRecentUserMessages, the chat rate-limit check.
  ['ai_chat_messages', { role: 1, user_id: 1, created_at: 1 }, 'plain'],
  // listConversations (equality prefix, sort field last); also covers the
  // cascade deletes since group_id(+ticket_id) is a prefix.
  ['ai_conversations', { group_id: 1, ticket_id: 1, user_id: 1, updated_at: -1 }, 'plain'],
];

const INDEX_OPTIONS = {
  plain: {},
  unique: { unique: true },
  ttl: { expireAfterSeconds: 0 },
};

export async function ensureIndexes(db) {
  for (const [collection, keys, kind] of INDEXES) {
    await db.collection(collection).createIndex(keys, INDEX_OPTIONS[kind]);
  }
}

// One-time backfill for tickets created before content_updated_at existed —
// without it, those documents fail to decode, 500ing any list/get that
// touches them. A plain `$set` can't copy another field's value, so this
// needs a pipeline update. Idempotent: a no-op once every ticket has it.
export async function backfillTicketContentUpdatedAt(db) {
  await db
    .collection('tickets')
    .updateMany({ content_updated_at: { $exists: false } }, [
      { $set: { content_updated_at: '$updated_at' } },
    ]);
}

// One-time wipe of chat messages from the old single-shared-thread-per-ticket
// model (no conversation_id). New messages always carry conversation_id, so
// the filter only ever matches leftover pre-migration documents.
export async function wipeLegacyChatMessages(db) {
  await db.collection('ai_chat_messages').deleteMany({ conversation_id: { $exists: false } });
}
