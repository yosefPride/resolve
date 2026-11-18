import { BSONRegExp, ObjectId } from 'mongodb';

// Mirrors utils/mod.rs.

// Inserts `doc` with a fresh _id as its first field (as the Rust driver
// stores it) and returns that id, so callers can hand back the stored
// document as { _id: id, ...doc }.
export async function insertWithId(collection, doc) {
  const _id = new ObjectId();
  await collection.insertOne({ _id, ...doc });
  return _id;
}

// ObjectId::parse_str — exactly 24 hex digits, either case.
const HEX_ID = /^[0-9a-fA-F]{24}$/;
export function parseObjectId(raw) {
  return HEX_ID.test(raw) ? new ObjectId(raw.toLowerCase()) : null;
}

// True when `err` is MongoDB's duplicate-key rejection (code 11000) from a
// unique index — on plain inserts and on findOneAndUpdate alike.
export function isDuplicateKey(err) {
  return err?.code === 11000;
}

// Escapes regex metacharacters so user-supplied text matches literally when
// embedded in a MongoDB `$regex` query.
const REGEX_META = new Set(['\\', '.', '+', '*', '?', '(', ')', '|', '[', ']', '{', '}', '^', '$']);
export function escapeRegex(input) {
  let escaped = '';
  for (const ch of input) {
    if (REGEX_META.has(ch)) escaped += '\\';
    escaped += ch;
  }
  return escaped;
}

// Case-insensitive substring match for a literal `term`, as a MongoDB field
// filter value.
export function substringRegex(term) {
  return new BSONRegExp(escapeRegex(term), 'i');
}

// Levenshtein (edit) distance over Unicode scalar values, for typo-tolerant
// ticket title search.
export function levenshteinDistance(a, b) {
  const left = Array.from(a);
  const right = Array.from(b);
  let prev = Array.from({ length: right.length + 1 }, (_, i) => i);
  let curr = new Array(right.length + 1).fill(0);
  for (let i = 0; i < left.length; i++) {
    curr[0] = i + 1;
    for (let j = 0; j < right.length; j++) {
      const cost = left[i] === right[j] ? 0 : 1;
      curr[j + 1] = Math.min(prev[j + 1] + 1, curr[j] + 1, prev[j] + cost);
    }
    [prev, curr] = [curr, prev];
  }
  return prev[right.length];
}
