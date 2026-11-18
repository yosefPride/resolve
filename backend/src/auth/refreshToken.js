import { createHash, randomBytes } from 'node:crypto';

export const REFRESH_TOKEN_TTL_DAYS = 30;
export const REFRESH_TOKEN_COOKIE = 'refresh_token';

// 32 bytes of CSPRNG output (256 bits) — already high enough entropy that,
// unlike passwords, the token can be hashed with a fast general-purpose hash
// (SHA-256) rather than a deliberately slow one like bcrypt.
const TOKEN_BYTES = 32;

// Generates a new opaque refresh token. Returns [raw, hash]: `raw` is handed
// to the client and never stored; `hash` is what gets persisted, so a leaked
// database can't be used to mint sessions.
export function generate() {
  const raw = randomBytes(TOKEN_BYTES).toString('hex');
  return [raw, hashToken(raw)];
}

export function hashToken(raw) {
  return createHash('sha256').update(raw, 'utf8').digest('hex');
}
