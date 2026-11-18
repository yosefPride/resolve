// actix-web's Json extractor accepts a body only when Content-Type parses (by
// the `mime` crate's rules) to a */json or */*+json type; anything else,
// including a missing or unparsable header, is "Content type error".

// RFC 7230 token characters, as in mime's TOKEN_MAP.
function isToken(c) {
  return (
    c === 0x21 ||
    (c >= 0x23 && c <= 0x27) ||
    c === 0x2a ||
    c === 0x2b ||
    c === 0x2d ||
    c === 0x2e ||
    (c >= 0x30 && c <= 0x39) ||
    (c >= 0x41 && c <= 0x5a) ||
    (c >= 0x5e && c <= 0x7a) ||
    c === 0x7c ||
    c === 0x7e
  );
}

function isRestrictedQuotedChar(c) {
  return c > 31 && c !== 127;
}

// mime's params_from_str, reduced to "is the parameter list well-formed".
// `next()` walks the same byte iterator the essence parsing left off on.
function paramsValid(s, semicolon) {
  let pos = semicolon + 1;
  const next = () => (pos < s.length ? [pos, s.charCodeAt(pos++)] : null);
  let start = semicolon + 1;
  params: while (start < s.length) {
    for (;;) {
      const item = next();
      if (item === null) return false;
      const [i, c] = item;
      if (c === 0x20 && i === start) {
        start = i + 1;
        continue params;
      }
      if (isToken(c)) continue;
      if (c === 0x3d && i > start) {
        start = i + 1;
        break;
      }
      return false;
    }
    let quoted = false;
    for (;;) {
      const item = next();
      if (quoted) {
        if (item === null) return false;
        const [i, c] = item;
        if (c === 0x22 && i > start) break;
        if (isRestrictedQuotedChar(c)) continue;
        return false;
      }
      if (item === null) {
        start = s.length;
        break;
      }
      const [i, c] = item;
      if (c === 0x22 && i === start) {
        quoted = true;
        start = i + 1;
        continue;
      }
      if (isToken(c)) continue;
      if (c === 0x3b && i > start) {
        start = i + 1;
        break;
      }
      return false;
    }
    if (quoted) {
      for (;;) {
        const item = next();
        if (item === null) {
          start = s.length;
          break;
        }
        const [i, c] = item;
        if (c === 0x3b) {
          start = i + 1;
          break;
        }
        if (c !== 0x20) return false;
      }
    }
  }
  return true;
}

// Returns { subtype, suffix } (lowercased), or null when `s` doesn't parse.
export function parseMime(s) {
  if (s === '*/*') return { subtype: '*', suffix: null };
  let i = 0;
  for (;; i++) {
    if (i >= s.length) return null;
    const c = s.charCodeAt(i);
    if (isToken(c)) continue;
    if (c === 0x2f && i > 0) break;
    return null;
  }
  const start = i + 1;
  let plus = null;
  let end = s.length;
  for (i = start; i < s.length; i++) {
    const c = s.charCodeAt(i);
    if (c === 0x2b && i > start) {
      plus = i;
      continue;
    }
    if (c === 0x3b && i > start) {
      end = i;
      if (!paramsValid(s, i)) return null;
      break;
    }
    if (!isToken(c)) return null;
  }
  const lower = s.slice(0, end).toLowerCase();
  return plus === null
    ? { subtype: lower.slice(start), suffix: null }
    : { subtype: lower.slice(start, plus), suffix: lower.slice(plus + 1) };
}

export function isJsonContentType(headerValue) {
  if (headerValue === null) return false;
  const mime = parseMime(headerValue);
  return mime !== null && (mime.subtype === 'json' || mime.suffix === 'json');
}
