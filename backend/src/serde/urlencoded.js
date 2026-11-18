// A port of serde_urlencoded::from_str for the query-string structs this API
// accepts, so a bad query parameter yields the same "Query deserialize error:
// ..." text as the Rust server.

import { duplicateField, missingField, unknownVariant } from './fmt.js';

export class QueryError extends Error {}

// %XX decoding of valid sequences only; anything malformed stays literal.
function percentDecode(bytes) {
  const out = [];
  for (let i = 0; i < bytes.length; i++) {
    const b = bytes[i];
    if (b === 0x25 && i + 2 < bytes.length && isHex(bytes[i + 1]) && isHex(bytes[i + 2])) {
      out.push(parseInt(String.fromCharCode(bytes[i + 1], bytes[i + 2]), 16));
      i += 2;
    } else {
      out.push(b);
    }
  }
  return Buffer.from(out);
}

function isHex(b) {
  return (b >= 0x30 && b <= 0x39) || (b >= 0x41 && b <= 0x46) || (b >= 0x61 && b <= 0x66);
}

// form_urlencoded::parse: '&'-separated pairs (empty ones skipped), split at
// the first '=', '+' as space, percent-decoded, lossy UTF-8.
function decodeComponent(bytes) {
  const plusAsSpace = Buffer.from(bytes).map((b) => (b === 0x2b ? 0x20 : b));
  return percentDecode(plusAsSpace).toString('utf8');
}

export function parsePairs(query) {
  const bytes = Buffer.from(query, 'latin1');
  const pairs = [];
  let start = 0;
  while (start <= bytes.length) {
    let end = bytes.indexOf(0x26, start);
    if (end === -1) end = bytes.length;
    const sequence = bytes.subarray(start, end);
    start = end + 1;
    if (sequence.length === 0) continue;
    const eq = sequence.indexOf(0x3d);
    const name = eq === -1 ? sequence : sequence.subarray(0, eq);
    const value = eq === -1 ? Buffer.alloc(0) : sequence.subarray(eq + 1);
    pairs.push([decodeComponent(name), decodeComponent(value)]);
  }
  return pairs;
}

// <u64 as FromStr>
function parseU64(text) {
  if (text === '') throw new QueryError('cannot parse integer from empty string');
  let digits = text;
  if (text === '+' || text === '-') throw new QueryError('invalid digit found in string');
  if (text[0] === '+') digits = text.slice(1);
  if (!/^[0-9]+$/.test(digits)) throw new QueryError('invalid digit found in string');
  const value = BigInt(digits);
  if (value > (1n << 64n) - 1n) throw new QueryError('number too large to fit in target type');
  return value;
}

function deserializePart(value, type) {
  switch (type.kind) {
    case 'string': return value;
    case 'option': return deserializePart(value, type.inner); // always Some when present
    case 'enum':
      if (type.variants.includes(value)) return value;
      throw new QueryError(unknownVariant(value, type.variants));
    case 'u64': return parseU64(value);
    default: throw new Error(`unsupported query type ${type.kind}`);
  }
}

// serde_urlencoded::from_str::<T>(query) for a struct T.
export function fromQuery(query, type) {
  const values = new Map();
  for (const [key, value] of parsePairs(query)) {
    const field = type.fields.find(([name]) => name === key);
    if (!field) continue;
    if (values.has(key)) throw new QueryError(duplicateField(key));
    values.set(key, deserializePart(value, field[1]));
  }
  const out = {};
  for (const [name, fieldType] of type.fields) {
    if (values.has(name)) out[name] = values.get(name);
    else if (fieldType.kind === 'option') out[name] = null;
    else throw new QueryError(missingField(name));
  }
  return out;
}
