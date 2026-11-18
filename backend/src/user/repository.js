import { ApiError } from '../errors.js';
import { insertWithId, isDuplicateKey, substringRegex } from '../utils.js';
import { GlobalRole, decodeUser } from './models.js';

// Any duplicate-key failure here is the unique email index (UserRepoError).
async function guard(promise) {
  try {
    return await promise;
  } catch (err) {
    if (isDuplicateKey(err)) throw ApiError.duplicateEmail();
    throw err;
  }
}

export class UserRepository {
  constructor(db) {
    this.collection = db.collection('users');
  }

  async create({ email, name, passwordHash }) {
    const user = {
      email,
      password_hash: passwordHash,
      name,
      global_role: null,
      created_at: new Date(),
    };
    const id = await guard(insertWithId(this.collection, user));
    return { _id: id, ...user };
  }

  async updateProfile(id, name, email) {
    const doc = await guard(
      this.collection.findOneAndUpdate(
        { _id: id },
        { $set: { name, email } },
        { returnDocument: 'after' },
      ),
    );
    return doc ? decodeUser(doc) : null;
  }

  async updatePasswordHash(id, passwordHash) {
    const result = await guard(
      this.collection.updateOne({ _id: id }, { $set: { password_hash: passwordHash } }),
    );
    return result.matchedCount > 0;
  }

  async updateGlobalRole(id, globalRole) {
    const result = await guard(this.collection.updateOne({ _id: id }, { $set: { global_role: globalRole } }));
    return result.matchedCount > 0;
  }

  async clearGlobalRole(id) {
    const result = await guard(this.collection.updateOne({ _id: id }, { $unset: { global_role: '' } }));
    return result.matchedCount > 0;
  }

  // Used to guard demotion: refuses to demote the last System Admin.
  async countSystemAdmins() {
    return guard(this.collection.countDocuments({ global_role: GlobalRole.SystemAdmin }));
  }

  async findByEmail(email) {
    const doc = await guard(this.collection.findOne({ email }));
    return doc ? decodeUser(doc) : null;
  }

  async findById(id) {
    const doc = await guard(this.collection.findOne({ _id: id }));
    return doc ? decodeUser(doc) : null;
  }

  // `search`, when present and non-empty, filters to users whose name or email
  // contains it (case-insensitive substring). Absent/blank returns every user.
  async listAll(search) {
    let filter = {};
    if (search) {
      const rx = substringRegex(search);
      filter = { $or: [{ name: rx }, { email: rx }] };
    }
    const docs = await guard(this.collection.find(filter).toArray());
    return docs.map(decodeUser);
  }

  async delete(id) {
    const result = await guard(this.collection.deleteOne({ _id: id }));
    return result.deletedCount > 0;
  }
}
