// Type descriptions for request bodies and query strings, standing in for the
// Rust structs' `#[derive(Deserialize)]`. The deserializers in json.js and
// urlencoded.js walk these the same way serde's generated code does.

export const string = { kind: 'string' }; // String
export const u64 = { kind: 'u64' }; // u64 (query strings only)
export const stringMap = { kind: 'map' }; // HashMap<String, String>

// Option<T>
export function option(inner) {
  return { kind: 'option', inner };
}

// A unit-only enum with #[serde(rename_all = "snake_case")]; `variants` are
// the serialized names, in declaration order.
export function unitEnum(name, variants) {
  return { kind: 'enum', name, variants };
}

// A struct; `fields` is { name: type } in declaration order.
export function struct(name, fields) {
  return { kind: 'struct', name, fields: Object.entries(fields) };
}
