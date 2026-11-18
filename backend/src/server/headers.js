// Header access with the semantics the Rust server's code relies on:
// `headers().get(name)` is the first occurrence, and `to_str()` fails unless
// the value is visible ASCII (or tab).

export function headerValues(req, name) {
  const values = [];
  const raw = req.rawHeaders;
  for (let i = 0; i < raw.length; i += 2) {
    if (raw[i].toLowerCase() === name) values.push(raw[i + 1]);
  }
  return values;
}

export function firstHeader(req, name) {
  const values = headerValues(req, name);
  return values.length > 0 ? values[0] : null;
}

function isVisibleAscii(value) {
  for (let i = 0; i < value.length; i++) {
    const c = value.charCodeAt(i);
    if (!((c >= 0x20 && c < 0x7f) || c === 0x09)) return false;
  }
  return true;
}

// headers().get(name).and_then(|v| v.to_str().ok())
export function headerStr(req, name) {
  const value = firstHeader(req, name);
  return value !== null && isVisibleAscii(value) ? value : null;
}
