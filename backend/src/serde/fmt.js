// serde's error-message wording, as the Rust server prints it inside
// "Json deserialize error: ..." / "Query deserialize error: ..." responses.

// <str as Debug>: quotes plus escapes. Grapheme extenders (e.g. U+FE0F) and
// non-printable characters (categories Cc Cf Cs Co Cn Zl Zp Zs, except the
// plain space) print as \u{hex}.
const GRAPHEME_EXTEND = /\p{Grapheme_Extend}/u;
const NOT_PRINTABLE = /[\p{Cc}\p{Cf}\p{Cs}\p{Co}\p{Cn}\p{Zl}\p{Zp}\p{Zs}]/u;

export function debugStr(s) {
  let out = '"';
  for (const ch of s) {
    switch (ch) {
      case '\0': out += '\\0'; break;
      case '\t': out += '\\t'; break;
      case '\r': out += '\\r'; break;
      case '\n': out += '\\n'; break;
      case '\\': out += '\\\\'; break;
      case '"': out += '\\"'; break;
      default:
        if (GRAPHEME_EXTEND.test(ch) || (ch !== ' ' && NOT_PRINTABLE.test(ch))) {
          out += `\\u{${ch.codePointAt(0).toString(16)}}`;
        } else {
          out += ch;
        }
    }
  }
  return `${out}"`;
}

// f64 formatting used by serde_json (the zmij crate): shortest round-trip
// digits, plain notation for decimal exponents -5..=15 (always with a
// fractional part), scientific with an explicit exponent sign otherwise.
export function formatFloat(value) {
  if (value === 0) return Object.is(value, -0) ? '-0.0' : '0.0';
  const sign = value < 0 ? '-' : '';
  const [mantissa, exponentText] = Math.abs(value).toExponential().split('e');
  const digits = mantissa.replace('.', '');
  const exp = Number(exponentText);
  const length = digits.length;
  let body;
  if (exp >= -5 && exp <= 15) {
    if (length - 1 <= exp) {
      body = `${digits}${'0'.repeat(exp + 1 - length)}.0`;
    } else if (exp >= 0) {
      body = `${digits.slice(0, exp + 1)}.${digits.slice(exp + 1)}`;
    } else {
      body = `0.${'0'.repeat(-exp - 1)}${digits}`;
    }
  } else {
    const fraction = length > 1 ? `.${digits.slice(1)}` : '';
    body = `${digits[0]}${fraction}e${exp < 0 ? '-' : '+'}${Math.abs(exp)}`;
  }
  return sign + body;
}

// serde::de::Unexpected, as displayed by serde_json (null instead of "unit
// value", zmij-formatted floats).
export function unexpected(value) {
  switch (value.type) {
    case 'null': return 'null';
    case 'bool': return `boolean \`${value.value}\``;
    case 'u64':
    case 'i64': return `integer \`${value.value}\``;
    case 'f64': return `floating point \`${formatFloat(value.value)}\``;
    case 'str': return `string ${debugStr(value.value)}`;
    case 'seq': return 'sequence';
    case 'map': return 'map';
    default: throw new Error(`unknown unexpected type ${value.type}`);
  }
}

export function invalidType(unexp, expected) {
  return `invalid type: ${unexpected(unexp)}, expected ${expected}`;
}

function oneOf(names) {
  if (names.length === 1) return `\`${names[0]}\``;
  if (names.length === 2) return `\`${names[0]}\` or \`${names[1]}\``;
  return `one of ${names.map((name) => `\`${name}\``).join(', ')}`;
}

export function unknownVariant(variant, variants) {
  return `unknown variant \`${variant}\`, expected ${oneOf(variants)}`;
}

export function missingField(field) {
  return `missing field \`${field}\``;
}

export function duplicateField(field) {
  return `duplicate field \`${field}\``;
}
