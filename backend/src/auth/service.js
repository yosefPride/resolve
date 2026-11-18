import { ApiError } from '../errors.js';
import { trim } from '../rust.js';
import { userResponse } from '../user/models.js';
import { UserService } from '../user/service.js';
import { parseObjectId } from '../utils.js';
import { issueToken } from './jwt.js';
import { hashPassword, verifyPassword } from './password.js';
import * as refreshToken from './refreshToken.js';
import { AuthRepository } from './repository.js';

const DAY_MS = 24 * 60 * 60 * 1000;

// bcrypt failures (a malformed stored hash) are Internal, not a mismatch.
async function checkPassword(password, hash) {
  try {
    return await verifyPassword(password, hash);
  } catch {
    throw ApiError.internal();
  }
}

export class AuthService {
  constructor(db, jwtSecret) {
    this.userService = new UserService(db);
    this.authRepo = new AuthRepository(db);
    this.jwtSecret = jwtSecret;
  }

  // Mints a fresh session for a user: a short-lived access token plus a new
  // refresh-token row. Shared by register, login, and refresh (rotation) so
  // all three issue sessions the exact same way.
  async issueSession(userId) {
    const jwt = issueToken(userId, this.jwtSecret);
    const [rawRefreshToken, tokenHash] = refreshToken.generate();
    const expiresAt = new Date(Date.now() + refreshToken.REFRESH_TOKEN_TTL_DAYS * DAY_MS);
    const userObjectId = parseObjectId(userId);
    if (!userObjectId) throw ApiError.internal();
    await this.authRepo.insert(userObjectId, tokenHash, expiresAt);
    return [jwt, rawRefreshToken];
  }

  // Returns the JSON body (user + access token) alongside the raw refresh
  // token, which the handler sets as an httpOnly cookie rather than
  // returning in the body.
  async register(input) {
    const passwordHash = await hashPassword(input.password);
    const user = await this.userService.create({
      email: input.email,
      name: input.name,
      passwordHash,
    });
    const [jwt, rawRefreshToken] = await this.issueSession(user.id);
    return [{ user, jwt }, rawRefreshToken];
  }

  async login(input) {
    const found = await this.userService.findByEmail(input.email);
    if (!found) throw ApiError.invalidCredentials();
    if (!(await checkPassword(input.password, found.password_hash))) {
      throw ApiError.invalidCredentials();
    }
    const user = userResponse(found);
    const [jwt, rawRefreshToken] = await this.issueSession(user.id);
    return [{ user, jwt }, rawRefreshToken];
  }

  // Updates the caller's own name/email. Changing the email requires the
  // current password; a name-only change does not.
  async updateMe(userId, input) {
    const user = await this.userService.findFullById(userId);
    if (!user) throw ApiError.unauthenticated();

    const name = input.name ?? user.name;
    const email = input.email ?? user.email;

    if (email !== user.email) {
      if (input.current_password === null) {
        throw ApiError.validation('current password is required to change email');
      }
      if (!(await checkPassword(input.current_password, user.password_hash))) {
        throw ApiError.invalidCredentials();
      }
    }

    const updated = await this.userService.updateProfile(userId, trim(name), trim(email));
    if (!updated) throw ApiError.unauthenticated();
    return updated;
  }

  // Changes the caller's password after verifying the current one, then
  // revokes every other outstanding refresh token for the user — other
  // devices are logged out, but the session that made the change (the
  // refresh-token hash passed in, from this request's cookie) keeps working.
  // null — no cookie on the request — revokes all of them.
  async changePassword(userId, input, currentTokenHash) {
    const user = await this.userService.findFullById(userId);
    if (!user) throw ApiError.unauthenticated();

    if (!(await checkPassword(input.current_password, user.password_hash))) {
      throw ApiError.invalidCredentials();
    }

    const newHash = await hashPassword(input.new_password);
    await this.userService.updatePasswordHash(userId, newHash);
    await this.authRepo.revokeAllForUserExcept(userId, currentTokenHash);
  }

  // Exchanges a valid, unexpired, not-yet-used refresh token for a new
  // session. The presented token is revoked first (single-use rotation) — a
  // stolen copy stops working the moment the legitimate client refreshes.
  async refresh(rawRefreshToken) {
    const tokenHash = refreshToken.hashToken(rawRefreshToken);
    const record = await this.authRepo.findActiveByHash(tokenHash);
    if (!record) throw ApiError.unauthenticated();
    if (!record._id) throw ApiError.internal();
    await this.authRepo.revokeById(record._id);
    return this.issueSession(record.user_id.toHexString());
  }

  // Revokes a single session's refresh token — logout is per-device, not
  // global. A missing/unknown token is a no-op rather than an error, since
  // the end state ("this token no longer works") already holds.
  async logout(rawRefreshToken) {
    await this.authRepo.revokeByHash(refreshToken.hashToken(rawRefreshToken));
  }
}
