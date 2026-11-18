import { createHmac } from 'node:crypto';

// Short-lived on purpose: the access token is verified statelessly (signature +
// exp only, no DB lookup), so a stolen token's exposure window is bounded by
// this TTL rather than by an explicit revocation check. Session longevity comes
// from the refresh token instead (see auth/refreshToken.js).
const ACCESS_TOKEN_TTL_MINUTES = 15;

// jsonwebtoken's Validation::default(): HS256 only, `exp` required and checked
// with 60 seconds of leeway, and any `aud` claim rejected.
const LEEWAY_SECONDS = 60;

// The exact bytes jsonwebtoken writes for Header::default(): {"typ":"JWT","alg":"HS256"}.
const ENCODED_HEADER = Buffer.from('{"typ":"JWT","alg":"HS256"}').toString('base64url');

function sign(message, secret) {
  return createHmac('sha256', Buffer.from(secret, 'utf8')).update(message).digest('base64url');
}

// URL-safe base64 without padding, rejecting anything non-canonical (the
// base64 crate's URL_SAFE_NO_PAD engine).
function decodeBase64Url(text) {
  if (!/^[A-Za-z0-9_-]*$/.test(text) || text.length % 4 === 1) return null;
  const bytes = Buffer.from(text, 'base64url');
  return bytes.toString('base64url') === text ? bytes : null;
}

function parseJson(bytes) {
  try {
    return JSON.parse(bytes.toString('utf8'));
  } catch {
    return null;
  }
}

function nowSeconds() {
  return Math.floor(Date.now() / 1000);
}

export function issueToken(userId, secret) {
  const exp = Math.floor((Date.now() + ACCESS_TOKEN_TTL_MINUTES * 60 * 1000) / 1000);
  return issueTokenWithExp(userId, secret, exp);
}

export function issueTokenWithExp(userId, secret, exp) {
  const claims = Buffer.from(JSON.stringify({ sub: userId, exp })).toString('base64url');
  const message = `${ENCODED_HEADER}.${claims}`;
  return `${message}.${sign(message, secret)}`;
}

// Returns the claims ({ sub, exp }) of a valid token, or null.
export function decodeToken(token, secret) {
  const lastDot = token.lastIndexOf('.');
  if (lastDot === -1) return null;
  const signature = token.slice(lastDot + 1);
  const message = token.slice(0, lastDot);
  const payloadDot = message.lastIndexOf('.');
  if (payloadDot === -1) return null;
  const encodedHeader = message.slice(0, payloadDot);
  const encodedPayload = message.slice(payloadDot + 1);

  const headerBytes = decodeBase64Url(encodedHeader);
  const header = headerBytes && parseJson(headerBytes);
  if (!header || typeof header !== 'object' || header.alg !== 'HS256') return null;
  if (signature !== sign(message, secret)) return null;

  const payloadBytes = decodeBase64Url(encodedPayload);
  const claims = payloadBytes && parseJson(payloadBytes);
  if (!claims || typeof claims !== 'object' || Array.isArray(claims)) return null;
  const { sub, exp, aud } = claims;
  if (!Number.isSafeInteger(exp) || exp < 0) return null;
  if (typeof sub !== 'string') return null;
  if (typeof aud === 'string' || (Array.isArray(aud) && aud.every((a) => typeof a === 'string'))) {
    return null;
  }
  if (exp < nowSeconds() - LEEWAY_SECONDS) return null;
  return { sub, exp };
}
