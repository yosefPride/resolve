// HttpRequest::cookie(name) from actix-web: every Cookie header is split on
// ';', each piece parsed (and percent-decoded) by the `cookie` crate, pieces
// that fail to parse are skipped, and the first cookie with that name wins.
// A Cookie header that isn't valid UTF-8 makes the whole lookup fail.

import { trim } from '../rust.js';
import { headerValues } from './headers.js';

const utf8 = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true });

function isHex(c) {
  return /[0-9A-Fa-f]/.test(c);
}

// percent_encoding::percent_decode(..).decode_utf8()
function percentDecodeUtf8(text) {
  const bytes = Buffer.from(text, 'utf8');
  const out = [];
  for (let i = 0; i < bytes.length; i++) {
    if (
      bytes[i] === 0x25 &&
      i + 2 < bytes.length &&
      isHex(String.fromCharCode(bytes[i + 1])) &&
      isHex(String.fromCharCode(bytes[i + 2]))
    ) {
      out.push(parseInt(String.fromCharCode(bytes[i + 1], bytes[i + 2]), 16));
      i += 2;
    } else {
      out.push(bytes[i]);
    }
  }
  return utf8.decode(Buffer.from(out)); // throws on invalid UTF-8
}

function trimQuotes(s) {
  if (Buffer.byteLength(s, 'utf8') < 2) return s;
  return s.startsWith('"') && s.endsWith('"') ? s.slice(1, -1) : s;
}

// Cookie::parse_encoded for one "name=value" piece; null when unparsable.
function parseCookie(piece) {
  const eq = piece.indexOf('=');
  if (eq === -1) return null;
  const name = trim(piece.slice(0, eq));
  const value = trim(trimQuotes(trim(piece.slice(eq + 1))));
  if (name === '') return null;
  try {
    return { name: percentDecodeUtf8(name), value: percentDecodeUtf8(value) };
  } catch {
    return null;
  }
}

export function getCookie(req, name) {
  const cookies = [];
  for (const header of headerValues(req, 'cookie')) {
    let text;
    try {
      text = utf8.decode(Buffer.from(header, 'latin1'));
    } catch {
      return null;
    }
    for (const piece of text.split(';').map(trim).filter((s) => s !== '')) {
      const cookie = parseCookie(piece);
      if (cookie) cookies.push(cookie);
    }
  }
  return cookies.find((cookie) => cookie.name === name)?.value ?? null;
}
