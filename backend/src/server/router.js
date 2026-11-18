// Route matching with actix-router's semantics: patterns like
// "/groups/{id}/tickets/{ticket_id}" (each {param} matches one non-empty path
// segment), exact and case-sensitive, tried in registration order, each
// guarded by a single HTTP method. Matching runs on the "requoted" path:
// percent-escapes are decoded except %25 ('%'), %2F ('/') and %2B ('+').

function isHex(b) {
  return (b >= 0x30 && b <= 0x39) || (b >= 0x41 && b <= 0x46) || (b >= 0x61 && b <= 0x66);
}

function decodePercent(bytes, protectedBytes) {
  const out = [];
  for (let i = 0; i < bytes.length; i++) {
    if (bytes[i] === 0x25 && i + 2 < bytes.length && isHex(bytes[i + 1]) && isHex(bytes[i + 2])) {
      const value = parseInt(String.fromCharCode(bytes[i + 1], bytes[i + 2]), 16);
      if (!(value < 128 && protectedBytes.has(value))) {
        out.push(value);
        i += 2;
        continue;
      }
    }
    out.push(bytes[i]);
  }
  return Buffer.from(out).toString('utf8'); // lossy, like String::from_utf8_lossy
}

const PROTECTED = new Set([0x25, 0x2f, 0x2b]);
const NOTHING_PROTECTED = new Set();

// Url::new — the path used for route matching and match_info().
export function requotePath(rawPath) {
  return decodePercent(Buffer.from(rawPath, 'latin1'), PROTECTED);
}

// web::Path<String> values are fully percent-decoded on top of that.
export function fullyDecode(segment) {
  return decodePercent(Buffer.from(segment, 'utf8'), NOTHING_PROTECTED);
}

// Splits a request target into (path, query) the way http::Uri does: a
// fragment is dropped, and an absolute-form target keeps only its path.
export function splitTarget(target) {
  let rest = target;
  const hash = rest.indexOf('#');
  if (hash !== -1) rest = rest.slice(0, hash);
  if (!rest.startsWith('/')) {
    const absolute = /^[A-Za-z][A-Za-z0-9+.-]*:\/\/[^/?]*/.exec(rest);
    if (absolute) rest = rest.slice(absolute[0].length) || '/';
  }
  const question = rest.indexOf('?');
  return question === -1
    ? { path: rest, query: '' }
    : { path: rest.slice(0, question), query: rest.slice(question + 1) };
}

export function compilePattern(pattern) {
  const names = [];
  const source = pattern.replace(/\{(\w+)\}|[^{]+/g, (part, name) => {
    if (name) {
      names.push(name);
      return '([^/]+)';
    }
    return part.replace(/[.*+?^$()|[\]\\]/g, '\\$&');
  });
  return { regex: new RegExp(`^${source}$`), names };
}

export class Router {
  constructor() {
    this.routes = [];
  }

  add(method, pattern, handler) {
    this.routes.push({ method, handler, ...compilePattern(pattern) });
  }

  // Returns { handler, params } (params in path order, as matched) or null.
  match(method, path) {
    for (const route of this.routes) {
      if (route.method !== method) continue;
      const m = route.regex.exec(path);
      if (!m) continue;
      const params = route.names.map((name, i) => [name, m[i + 1]]);
      return { handler: route.handler, params };
    }
    return null;
  }
}
