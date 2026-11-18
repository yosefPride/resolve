import { ApiError } from '../errors.js';
import { byteLength, trim } from '../rust.js';
import { getCookie } from '../server/cookies.js';
import { json, request } from '../server/extract.js';
import { handler } from '../server/handler.js';
import { authenticatedUser } from '../server/middleware.js';
import { HttpResponse } from '../server/response.js';
import { UserService } from '../user/service.js';
import { ChangePasswordRequest, LoginRequest, RegisterRequest, UpdateMeRequest } from './models.js';
import { REFRESH_TOKEN_COOKIE, REFRESH_TOKEN_TTL_DAYS, hashToken } from './refreshToken.js';
import { AuthService } from './service.js';

// Scoped to /auth so the refresh token is never sent on unrelated API calls.
// httpOnly keeps it out of reach of JS; `secure` is config-driven since a
// real browser refuses to store a Secure cookie at all over plain HTTP.
//
// SameSite tracks `secure`: production (secure) deploys frontend and API on
// unrelated sites (separate *.onrender.com hosts are distinct registrable
// domains), so the cookie needs SameSite=None — which browsers only honor
// alongside Secure. Local dev (plain HTTP) can't use None at all, but doesn't
// need it either — a different port on localhost is still same-site.
function cookieAttributes(secure) {
  return secure ? 'HttpOnly; SameSite=None; Secure' : 'HttpOnly; SameSite=Strict';
}

function refreshCookie(rawToken, secure) {
  const maxAge = REFRESH_TOKEN_TTL_DAYS * 24 * 60 * 60;
  return `${REFRESH_TOKEN_COOKIE}=${rawToken}; ${cookieAttributes(secure)}; Path=/api/v1/auth; Max-Age=${maxAge}`;
}

function expiredRefreshCookie(secure) {
  return `${REFRESH_TOKEN_COOKIE}=; ${cookieAttributes(secure)}; Path=/api/v1/auth; Max-Age=0`;
}

function validateRegister(input) {
  if (trim(input.email) === '' || !input.email.includes('@')) {
    throw ApiError.validation('a valid email is required');
  }
  if (byteLength(input.password) < 8) {
    throw ApiError.validation('password must be at least 8 characters');
  }
  if (trim(input.name) === '') throw ApiError.validation('name is required');
}

function validateUpdateMe(input) {
  if (input.name === null && input.email === null) throw ApiError.validation('nothing to update');
  if (input.email !== null && (trim(input.email) === '' || !input.email.includes('@'))) {
    throw ApiError.validation('a valid email is required');
  }
  if (input.name !== null && trim(input.name) === '') throw ApiError.validation('name is required');
}

function validateChangePassword(input) {
  if (input.current_password === '') throw ApiError.validation('current password is required');
  if (byteLength(input.new_password) < 8) {
    throw ApiError.validation('password must be at least 8 characters');
  }
}

function validateLogin(input) {
  if (trim(input.email) === '' || input.password === '') {
    throw ApiError.validation('email and password are required');
  }
}

function authService(state) {
  return new AuthService(state.db, state.config.jwtSecret);
}

export const register = handler([json(RegisterRequest)], async ({ state }, input) => {
  validateRegister(input);
  const [response, rawRefreshToken] = await authService(state).register(input);
  return HttpResponse.created()
    .cookie(refreshCookie(rawRefreshToken, state.config.cookieSecure))
    .json(response);
});

export const login = handler([json(LoginRequest)], async ({ state }, input) => {
  validateLogin(input);
  const [response, rawRefreshToken] = await authService(state).login(input);
  return HttpResponse.ok()
    .cookie(refreshCookie(rawRefreshToken, state.config.cookieSecure))
    .json(response);
});

export const me = handler([authenticatedUser], async ({ state }, user) => {
  const response = await new UserService(state.db).findById(user.userId);
  if (!response) throw ApiError.unauthenticated();
  return HttpResponse.ok().json(response);
});

export const updateMe = handler(
  [authenticatedUser, json(UpdateMeRequest)],
  async ({ state }, user, input) => {
    validateUpdateMe(input);
    const response = await authService(state).updateMe(user.userId, input);
    return HttpResponse.ok().json(response);
  },
);

// The request's own refresh cookie (if present) identifies the session to
// spare from revocation, so changing the password logs out every other device
// without ending the session that made the change.
export const changePassword = handler(
  [authenticatedUser, request, json(ChangePasswordRequest)],
  async ({ state }, user, req, input) => {
    validateChangePassword(input);
    const cookie = getCookie(req, REFRESH_TOKEN_COOKIE);
    const currentTokenHash = cookie === null ? null : hashToken(cookie);
    await authService(state).changePassword(user.userId, input, currentTokenHash);
    return HttpResponse.ok().finish();
  },
);

// Does not require an access token: the refresh token *is* the session
// identifier here — the access token may well have expired by the time a
// client needs to refresh.
export const refresh = handler([request], async ({ state }, req) => {
  const rawRefreshToken = getCookie(req, REFRESH_TOKEN_COOKIE);
  if (rawRefreshToken === null) throw ApiError.unauthenticated();
  const [jwt, newRawRefreshToken] = await authService(state).refresh(rawRefreshToken);
  return HttpResponse.ok()
    .cookie(refreshCookie(newRawRefreshToken, state.config.cookieSecure))
    .json({ jwt });
});

// Per-device logout: revokes only the refresh token in this request's cookie,
// so other logged-in devices are unaffected. Requires no access token either
// — an absent/already-invalid cookie is treated as already logged out.
export const logout = handler([request], async ({ state }, req) => {
  const cookie = getCookie(req, REFRESH_TOKEN_COOKIE);
  if (cookie !== null) await authService(state).logout(cookie);
  return HttpResponse.ok().cookie(expiredRefreshCookie(state.config.cookieSecure)).finish();
});
