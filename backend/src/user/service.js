import { userResponse } from './models.js';
import { UserRepository } from './repository.js';

export class UserService {
  constructor(db) {
    this.repo = new UserRepository(db);
  }

  async create(input) {
    return userResponse(await this.repo.create(input));
  }

  // Returns the full user (including password_hash) — intentional, needed by
  // auth for login.
  findByEmail(email) {
    return this.repo.findByEmail(email);
  }

  // Returns the full user (including password_hash) — intentional, needed by
  // auth to verify the current password before an email change.
  findFullById(id) {
    return this.repo.findById(id);
  }

  async updateProfile(id, name, email) {
    const user = await this.repo.updateProfile(id, name, email);
    return user ? userResponse(user) : null;
  }

  updatePasswordHash(id, passwordHash) {
    return this.repo.updatePasswordHash(id, passwordHash);
  }

  async findById(id) {
    const user = await this.repo.findById(id);
    return user ? userResponse(user) : null;
  }

  async listAll(search) {
    return (await this.repo.listAll(search)).map(userResponse);
  }

  delete(id) {
    return this.repo.delete(id);
  }

  updateGlobalRole(id, globalRole) {
    return this.repo.updateGlobalRole(id, globalRole);
  }

  clearGlobalRole(id) {
    return this.repo.clearGlobalRole(id);
  }

  countSystemAdmins() {
    return this.repo.countSystemAdmins();
  }
}
