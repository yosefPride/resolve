// Small ports of Rust standard-library behavior that the Rust server relies
// on and that JavaScript does differently. Each one keeps a validation rule or
// a stored/returned value identical to the Rust version (e.g. `str::trim` and
// `String::trim` disagree on U+0085 and U+FEFF; `str::len` counts UTF-8 bytes).

// char::is_whitespace — the Unicode White_Space property.
function isWhitespace(cp) {
  return (
    (cp >= 0x09 && cp <= 0x0d) ||
    cp === 0x20 ||
    cp === 0x85 ||
    cp === 0xa0 ||
    cp === 0x1680 ||
    (cp >= 0x2000 && cp <= 0x200a) ||
    cp === 0x2028 ||
    cp === 0x2029 ||
    cp === 0x202f ||
    cp === 0x205f ||
    cp === 0x3000
  );
}

// str::trim
export function trim(s) {
  const chars = Array.from(s);
  let start = 0;
  let end = chars.length;
  while (start < end && isWhitespace(chars[start].codePointAt(0))) start++;
  while (end > start && isWhitespace(chars[end - 1].codePointAt(0))) end--;
  return chars.slice(start, end).join('');
}

// str::split_whitespace
export function splitWhitespace(s) {
  const words = [];
  let current = '';
  for (const ch of s) {
    if (isWhitespace(ch.codePointAt(0))) {
      if (current) words.push(current);
      current = '';
    } else {
      current += ch;
    }
  }
  if (current) words.push(current);
  return words;
}

// str::len — UTF-8 byte length, not characters.
export function byteLength(s) {
  return Buffer.byteLength(s, 'utf8');
}

// str::chars().count()
export function charCount(s) {
  let count = 0;
  for (const _ of s) count++;
  return count;
}

// str::chars().take(n).collect::<String>()
export function takeChars(s, n) {
  return Array.from(s).slice(0, n).join('');
}

// Ord for &str — byte-wise UTF-8 order (which is code point order), unlike
// JavaScript's UTF-16 code unit comparison.
export function compareStr(a, b) {
  return Buffer.compare(Buffer.from(a, 'utf8'), Buffer.from(b, 'utf8'));
}

// chrono's representable range; DateTime::from_timestamp_millis(..)
// .unwrap_or_default() turns anything outside it into the Unix epoch.
const CHRONO_MIN_MS = Date.UTC(-262143, 0, 1);
const CHRONO_MAX_MS = Date.UTC(262142, 11, 31, 23, 59, 59, 999);

// chrono's Serialize for DateTime<Utc> (RFC 3339, SecondsFormat::AutoSi, `Z`):
// fractional seconds only when non-zero. Stored datetimes have millisecond
// precision, so the fraction is either absent or exactly three digits.
export function formatDateTime(input) {
  const inRange = input.getTime() >= CHRONO_MIN_MS && input.getTime() <= CHRONO_MAX_MS;
  const date = inRange ? input : new Date(0);
  const ms = date.getTime();
  const year = date.getUTCFullYear();
  const pad = (n, width) => String(n).padStart(width, '0');
  const yearText =
    year >= 0 && year <= 9999 ? pad(year, 4) : (year < 0 ? '-' : '+') + pad(Math.abs(year), 4);
  const millis = ((ms % 1000) + 1000) % 1000;
  return (
    `${yearText}-${pad(date.getUTCMonth() + 1, 2)}-${pad(date.getUTCDate(), 2)}` +
    `T${pad(date.getUTCHours(), 2)}:${pad(date.getUTCMinutes(), 2)}:${pad(date.getUTCSeconds(), 2)}` +
    (millis === 0 ? '' : `.${pad(millis, 3)}`) +
    'Z'
  );
}
