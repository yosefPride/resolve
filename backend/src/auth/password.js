import bcrypt from 'bcryptjs';

// Explicitly define target cost. 12 is the industry standard: 2^12 rounds.
const WORK_FACTOR = 12;

// Hashes come out as $2b$12$..., the same format the Rust bcrypt crate writes,
// so hashes from either server verify on the other.
export function hashPassword(password) {
  return bcrypt.hash(password, WORK_FACTOR);
}

// bcrypt's own base64 alphabet; parts must be canonically encoded.
const BCRYPT_ALPHABET = './ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';

function isCanonicalBcryptBase64(text, bytes) {
  for (const ch of text) if (!BCRYPT_ALPHABET.includes(ch)) return false;
  // Unused trailing bits of the last character must be zero.
  const lastBits = (text.length * 6) % 8;
  if (lastBits === 0) return text.length * 6 === bytes * 8;
  const last = BCRYPT_ALPHABET.indexOf(text[text.length - 1]);
  return (last & ((1 << lastBits) - 1)) === 0 && Math.floor((text.length * 6) / 8) === bytes;
}

// Parses a stored hash the way the bcrypt crate's verify() does; any problem
// is an error (an Internal failure upstream), not a mismatch.
function splitHash(hash) {
  const parts = hash.split('$').filter((part) => part !== '');
  if (parts.length !== 3) throw new Error('invalid hash');
  if (!['2y', '2b', '2a', '2x'].includes(parts[0])) throw new Error('invalid prefix');
  if (!/^\+?[0-9]+$/.test(parts[1]) || Number(parts[1]) > 0xffffffff) throw new Error('invalid cost');
  const cost = Number(parts[1]);
  if (Buffer.byteLength(parts[2], 'utf8') !== 53 || parts[2].length !== 53) throw new Error('invalid hash');
  const salt = parts[2].slice(0, 22);
  const digest = parts[2].slice(22);
  if (!isCanonicalBcryptBase64(salt, 16) || !isCanonicalBcryptBase64(digest, 23)) {
    throw new Error('invalid base64');
  }
  if (cost < 4 || cost > 31) throw new Error('cost not allowed');
  return { cost, salt, digest };
}

export async function verifyPassword(password, hash) {
  const { cost, salt, digest } = splitHash(hash);
  const computed = await bcrypt.hash(password, `$2b$${String(cost).padStart(2, '0')}$${salt}`);
  return computed.slice(-31) === digest;
}
