import { Long, ObjectId } from 'mongodb';

// Typed reading of stored documents. The Rust server deserializes every
// document into a struct, so a document with a missing or wrongly typed field
// fails the whole request with a 500; these decoders keep that behavior.

export class StoredDocumentError extends Error {}

export const objectId = { check: (v) => v instanceof ObjectId };
export const string = { check: (v) => typeof v === 'string' };
export const bool = { check: (v) => typeof v === 'boolean' };
export const datetime = { check: (v) => v instanceof Date };
export const i64 = {
  check: (v) => (typeof v === 'number' && Number.isInteger(v)) || v instanceof Long,
  normalize: (v) => (v instanceof Long ? v.toNumber() : v),
};

export function oneOf(variants) {
  return { check: (v) => typeof v === 'string' && variants.includes(v) };
}

// Option<T>: missing or null reads as null.
export function option(kind) {
  return { ...kind, optional: true };
}

// #[serde(default)] on a non-Option field: missing reads as `value`.
export function withDefault(kind, value) {
  return { ...kind, fallback: value };
}

export function model(name, fields) {
  const entries = Object.entries(fields);
  return (doc) => {
    const out = {};
    for (const [key, kind] of entries) {
      const value = doc[key];
      if (value === undefined && 'fallback' in kind) {
        out[key] = kind.fallback;
      } else if (value === undefined || value === null) {
        if (!kind.optional) throw new StoredDocumentError(`${name}: missing or null field ${key}`);
        out[key] = null;
      } else if (!kind.check(value)) {
        throw new StoredDocumentError(`${name}: invalid field ${key}`);
      } else {
        out[key] = kind.normalize ? kind.normalize(value) : value;
      }
    }
    return out;
  };
}
