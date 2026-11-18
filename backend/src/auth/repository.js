import { insertWithId } from '../utils.js';
import { decodeRefreshToken } from './models.js';

export class AuthRepository {
  constructor(db) {
    this.collection = db.collection('refresh_tokens');
  }

  async insert(userId, tokenHash, expiresAt) {
    await insertWithId(this.collection, {
      user_id: userId,
      token_hash: tokenHash,
      created_at: new Date(),
      expires_at: expiresAt,
      revoked_at: null,
    });
  }

  // Matches only a token that hasn't been revoked and hasn't expired — a
  // stolen-then-replayed token (already revoked by the legitimate rotation)
  // or a stale one simply won't be found, no separate reuse handling needed.
  async findActiveByHash(tokenHash) {
    const doc = await this.collection.findOne({
      token_hash: tokenHash,
      revoked_at: null,
      expires_at: { $gt: new Date() },
    });
    return doc ? decodeRefreshToken(doc) : null;
  }

  async revokeById(id) {
    await this.collection.updateOne({ _id: id }, { $set: { revoked_at: new Date() } });
  }

  // Revokes every outstanding refresh token for a user except the one whose
  // hash is given — password change logs out all other devices while the
  // session that made the change stays alive. `exceptHash` null (no refresh
  // cookie on the request) revokes them all.
  async revokeAllForUserExcept(userId, exceptHash) {
    const filter = { user_id: userId, revoked_at: null };
    if (exceptHash !== null) filter.token_hash = { $ne: exceptHash };
    await this.collection.updateMany(filter, { $set: { revoked_at: new Date() } });
  }

  async revokeByHash(tokenHash) {
    await this.collection.updateOne({ token_hash: tokenHash }, { $set: { revoked_at: new Date() } });
  }
}
