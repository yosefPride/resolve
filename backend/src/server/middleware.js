// Request guards — the Node equivalents of server/middleware.rs's extractors.

import { decodeToken } from '../auth/jwt.js';
import { ApiError } from '../errors.js';
import { RbacService } from '../rbac/service.js';
import { parseObjectId } from '../utils.js';
import { headerStr } from './headers.js';

// Header check is kept separate from (and ahead of) any state access so the
// "no/!Bearer token" paths resolve to Unauthenticated first.
function bearerToken(req) {
  const header = headerStr(req, 'authorization');
  if (header === null || !header.startsWith('Bearer ')) throw ApiError.unauthenticated();
  return header.slice('Bearer '.length);
}

function userIdFromToken(token, secret) {
  const claims = decodeToken(token, secret);
  const userId = claims && parseObjectId(claims.sub);
  if (!userId) throw ApiError.unauthenticated();
  return userId;
}

// Fully stateless: verified by signature + exp alone, no DB lookup. This is
// safe specifically because access tokens are short-lived — revocation happens
// at the refresh token layer.
//
// Use this on routes where authentication alone is the requirement (auth
// endpoints, cross-group group management). Data routes scoped to one tenant
// take groupScoped instead, and system routes systemAdminUser.
export const authenticatedUser = {
  prepare(ctx) {
    const token = bearerToken(ctx.req);
    return { userId: userIdFromToken(token, ctx.state.config.jwtSecret) };
  },
};

// The tenant-scoped request context: caller identity plus the group named by
// the `{id}` path segment. One membership lookup happens here so membership is
// always current — a user removed mid-token is rejected on their very next
// request rather than at token expiry. The caller's role is not carried: the
// service layer re-derives it where a role check is needed.
export const groupScoped = {
  prepare(ctx) {
    const token = bearerToken(ctx.req);
    const userId = userIdFromToken(token, ctx.state.config.jwtSecret);
    // Absent segment => mounted on a route without one (server
    // misconfiguration, Internal), distinct from a malformed id (Validation).
    const raw = ctx.params.find(([name]) => name === 'id');
    if (!raw) throw ApiError.internal();
    const groupId = parseObjectId(raw[1]);
    if (!groupId) throw ApiError.validation('invalid id');
    return { userId, groupId };
  },
  async resolve(ctx, scoped) {
    await new RbacService(ctx.state.db).requireMember(scoped.groupId, scoped.userId);
    return scoped;
  },
};

// Global-scope guard for /admin routes: authenticates, then confirms the caller
// holds the System Admin global role. A non-admin or unknown caller resolves to
// Forbidden. Service-layer requireSystemAdmin still runs underneath.
export const systemAdminUser = {
  prepare(ctx) {
    const token = bearerToken(ctx.req);
    return { userId: userIdFromToken(token, ctx.state.config.jwtSecret) };
  },
  async resolve(ctx, user) {
    await new RbacService(ctx.state.db).requireSystemAdmin(user.userId);
    return user;
  },
};
