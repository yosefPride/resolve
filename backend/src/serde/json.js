// A port of serde_json 1.0's `from_slice` for the request-body types this API
// accepts (see types.js). actix-web returns serde_json's error text verbatim
// ("Json deserialize error: missing field `email` at line 1 column 2"), so
// parsing is reproduced step for step — the order bytes are consumed decides
// the reported line/column, and the order checks run decides which error wins.

import { duplicateField, invalidType, missingField, unknownVariant } from './fmt.js';

const MSG = {
  EofWhileParsingList: 'EOF while parsing a list',
  EofWhileParsingObject: 'EOF while parsing an object',
  EofWhileParsingString: 'EOF while parsing a string',
  EofWhileParsingValue: 'EOF while parsing a value',
  ExpectedColon: 'expected `:`',
  ExpectedListCommaOrEnd: 'expected `,` or `]`',
  ExpectedObjectCommaOrEnd: 'expected `,` or `}`',
  ExpectedSomeIdent: 'expected ident',
  ExpectedSomeValue: 'expected value',
  InvalidEscape: 'invalid escape',
  InvalidNumber: 'invalid number',
  NumberOutOfRange: 'number out of range',
  InvalidUnicodeCodePoint: 'invalid unicode code point',
  ControlCharacterWhileParsingString:
    'control character (\\u0000-\\u001F) found while parsing a string',
  KeyMustBeAString: 'key must be a string',
  LoneLeadingSurrogateInHexEscape: 'lone leading surrogate in hex escape',
  TrailingComma: 'trailing comma',
  TrailingCharacters: 'trailing characters',
  UnexpectedEndOfHexEscape: 'unexpected end of hex escape',
  RecursionLimitExceeded: 'recursion limit exceeded',
};

// line 0 = "no position yet"; the deserializer stamps the current position
// on such errors (fix_position) as they propagate.
export class JsonError extends Error {
  constructor(msg, line, column) {
    super(line === 0 ? msg : `${msg} at line ${line} column ${column}`);
    this.msg = msg;
    this.line = line;
    this.column = column;
  }
}

function custom(msg) {
  return new JsonError(msg, 0, 0);
}

const QUOTE = 0x22;
const BACKSLASH = 0x5c;
const COMMA = 0x2c;
const COLON = 0x3a;
const LBRACE = 0x7b;
const RBRACE = 0x7d;
const LBRACKET = 0x5b;
const RBRACKET = 0x5d;
const MINUS = 0x2d;
const PLUS = 0x2b;
const DOT = 0x2e;
const LOWER_E = 0x65;
const UPPER_E = 0x45;
const LOWER_U = 0x75;

const isDigit = (b) => b >= 0x30 && b <= 0x39;
const digitValue = (b) => b - 0x30;

// ignoreBOM: a leading U+FEFF is string content here, not a byte-order mark.
const utf8 = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true });

const U64_MAX = (1n << 64n) - 1n;
const I32_MAX = 2147483647;
const I32_MIN = -2147483648;
const POW10 = Array.from({ length: 309 }, (_, i) => Number(`1e${i}`));

// overflow!(a * 10 + b, max)
function overflows(a, b, max) {
  return a >= max / 10n && (a > max / 10n || b > max % 10n);
}

function hexValue(b) {
  if (b >= 0x30 && b <= 0x39) return b - 0x30;
  if (b >= 0x41 && b <= 0x46) return b - 0x41 + 10;
  if (b >= 0x61 && b <= 0x66) return b - 0x61 + 10;
  return -1;
}

function saturatingI32(n) {
  return Math.max(I32_MIN, Math.min(I32_MAX, n));
}

class Deserializer {
  constructor(slice) {
    this.slice = slice;
    this.index = 0;
    this.remainingDepth = 128;
  }

  // --- SliceRead ---

  next() {
    return this.index < this.slice.length ? this.slice[this.index++] : null;
  }

  peek() {
    return this.index < this.slice.length ? this.slice[this.index] : null;
  }

  peekOrNull() {
    const b = this.peek();
    return b === null ? 0 : b;
  }

  nextCharOrNull() {
    const b = this.next();
    return b === null ? 0 : b;
  }

  eatChar() {
    this.index += 1;
  }

  positionOf(i) {
    const startOfLine = this.slice.subarray(0, i).lastIndexOf(0x0a) + 1;
    let line = 1;
    for (let k = 0; k < startOfLine; k++) {
      if (this.slice[k] === 0x0a) line++;
    }
    return { line, column: i - startOfLine };
  }

  // Error caused by a byte from next().
  error(msg) {
    const { line, column } = this.positionOf(this.index);
    return new JsonError(msg, line, column);
  }

  // Error caused by a byte from peek().
  peekError(msg) {
    const { line, column } = this.positionOf(Math.min(this.slice.length, this.index + 1));
    return new JsonError(msg, line, column);
  }

  fixPosition(err) {
    return err instanceof JsonError && err.line === 0 ? this.error(err.msg) : err;
  }

  // --- strings ---

  parseStr() {
    const chunks = [];
    let copied = false;
    let start = this.index;
    const len = this.slice.length;
    for (;;) {
      while (this.index < len) {
        const b = this.slice[this.index];
        if (b === QUOTE || b === BACKSLASH || b < 0x20) break;
        this.index++;
      }
      if (this.index === len) throw this.error(MSG.EofWhileParsingString);
      const b = this.slice[this.index];
      if (b === QUOTE) {
        chunks.push(this.slice.subarray(start, this.index));
        this.index++;
        return this.asStr(copied ? Buffer.concat(chunks) : chunks[0]);
      }
      if (b === BACKSLASH) {
        copied = true;
        chunks.push(this.slice.subarray(start, this.index));
        this.index++;
        this.parseEscape(chunks);
        start = this.index;
      } else {
        this.index++;
        throw this.error(MSG.ControlCharacterWhileParsingString);
      }
    }
  }

  asStr(bytes) {
    try {
      return utf8.decode(bytes);
    } catch {
      throw this.error(MSG.InvalidUnicodeCodePoint);
    }
  }

  parseEscape(chunks) {
    const ch = this.next();
    if (ch === null) throw this.error(MSG.EofWhileParsingString);
    switch (ch) {
      case QUOTE: chunks.push(Buffer.of(0x22)); break;
      case BACKSLASH: chunks.push(Buffer.of(0x5c)); break;
      case 0x2f: chunks.push(Buffer.of(0x2f)); break; // '/'
      case 0x62: chunks.push(Buffer.of(0x08)); break; // 'b'
      case 0x66: chunks.push(Buffer.of(0x0c)); break; // 'f'
      case 0x6e: chunks.push(Buffer.of(0x0a)); break; // 'n'
      case 0x72: chunks.push(Buffer.of(0x0d)); break; // 'r'
      case 0x74: chunks.push(Buffer.of(0x09)); break; // 't'
      case LOWER_U: this.parseUnicodeEscape(chunks); break;
      default: throw this.error(MSG.InvalidEscape);
    }
  }

  parseUnicodeEscape(chunks) {
    const n = this.decodeHexEscape();
    if (n >= 0xdc00 && n <= 0xdfff) throw this.error(MSG.LoneLeadingSurrogateInHexEscape);
    if (n < 0xd800 || n > 0xdbff) {
      chunks.push(Buffer.from(String.fromCodePoint(n), 'utf8'));
      return;
    }
    // A leading surrogate must be followed by \u and a trailing surrogate.
    for (const expected of [BACKSLASH, LOWER_U]) {
      const b = this.peek();
      if (b === null) throw this.error(MSG.EofWhileParsingString);
      this.eatChar();
      if (b !== expected) throw this.error(MSG.UnexpectedEndOfHexEscape);
    }
    const n2 = this.decodeHexEscape();
    if (n2 < 0xdc00 || n2 > 0xdfff) throw this.error(MSG.LoneLeadingSurrogateInHexEscape);
    const codePoint = (((n - 0xd800) << 10) | (n2 - 0xdc00)) + 0x10000;
    chunks.push(Buffer.from(String.fromCodePoint(codePoint), 'utf8'));
  }

  decodeHexEscape() {
    if (this.slice.length - this.index < 4) {
      this.index = this.slice.length;
      throw this.error(MSG.EofWhileParsingString);
    }
    const digits = this.slice.subarray(this.index, this.index + 4);
    this.index += 4;
    let value = 0;
    for (const d of digits) {
      const h = hexValue(d);
      if (h < 0) throw this.error(MSG.InvalidEscape);
      value = value * 16 + h;
    }
    return value;
  }

  // --- structure ---

  parseWhitespace() {
    for (;;) {
      const b = this.peek();
      if (b === 0x20 || b === 0x0a || b === 0x09 || b === 0x0d) {
        this.eatChar();
      } else {
        return b;
      }
    }
  }

  parseIdent(ident) {
    for (const expected of Buffer.from(ident)) {
      const next = this.next();
      if (next === null) throw this.error(MSG.EofWhileParsingValue);
      if (next !== expected) throw this.error(MSG.ExpectedSomeIdent);
    }
  }

  parseObjectColon() {
    const b = this.parseWhitespace();
    if (b === COLON) {
      this.eatChar();
      return;
    }
    throw this.peekError(b === null ? MSG.EofWhileParsingObject : MSG.ExpectedColon);
  }

  endSeq() {
    const b = this.parseWhitespace();
    if (b === RBRACKET) {
      this.eatChar();
      return;
    }
    if (b === COMMA) {
      this.eatChar();
      const next = this.parseWhitespace();
      throw this.peekError(next === RBRACKET ? MSG.TrailingComma : MSG.TrailingCharacters);
    }
    throw this.peekError(b === null ? MSG.EofWhileParsingList : MSG.TrailingCharacters);
  }

  endMap() {
    const b = this.parseWhitespace();
    if (b === RBRACE) {
      this.eatChar();
      return;
    }
    if (b === COMMA) throw this.peekError(MSG.TrailingComma);
    throw this.peekError(b === null ? MSG.EofWhileParsingObject : MSG.TrailingCharacters);
  }

  end() {
    if (this.parseWhitespace() !== null) throw this.peekError(MSG.TrailingCharacters);
  }

  enterNested() {
    this.remainingDepth -= 1;
    if (this.remainingDepth === 0) throw this.peekError(MSG.RecursionLimitExceeded);
  }

  // `match (ret, self.end_seq()/end_map())` — both run, the visitor's error wins.
  visitThenEnd(visit, end) {
    this.enterNested();
    this.eatChar();
    let value;
    let visitError;
    try {
      value = visit();
    } catch (err) {
      visitError = err;
    }
    this.remainingDepth += 1;
    let endError;
    try {
      end();
    } catch (err) {
      endError = err;
    }
    if (visitError) throw visitError;
    if (endError) throw endError;
    return value;
  }

  // --- numbers (only parsed to describe them in an "invalid type" error) ---

  parseInteger(positive) {
    const next = this.next();
    if (next === null) throw this.error(MSG.EofWhileParsingValue);
    if (next === 0x30) {
      if (isDigit(this.peekOrNull())) throw this.peekError(MSG.InvalidNumber);
      return this.parseNumber(positive, 0n);
    }
    if (next >= 0x31 && next <= 0x39) {
      let significand = BigInt(digitValue(next));
      for (;;) {
        const c = this.peekOrNull();
        if (!isDigit(c)) return this.parseNumber(positive, significand);
        const digit = BigInt(digitValue(c));
        if (overflows(significand, digit, U64_MAX)) {
          return { type: 'f64', value: this.parseLongInteger(positive, significand) };
        }
        this.eatChar();
        significand = significand * 10n + digit;
      }
    }
    throw this.error(MSG.InvalidNumber);
  }

  parseNumber(positive, significand) {
    const c = this.peekOrNull();
    if (c === DOT) return { type: 'f64', value: this.parseDecimal(positive, significand, 0) };
    if (c === LOWER_E || c === UPPER_E) {
      return { type: 'f64', value: this.parseExponent(positive, significand, 0) };
    }
    if (positive) return { type: 'u64', value: significand };
    const neg = BigInt.asIntN(64, -BigInt.asIntN(64, significand));
    // Convert into a float if we underflow, or on `-0`.
    if (neg >= 0n) return { type: 'f64', value: -Number(significand) };
    return { type: 'i64', value: neg };
  }

  parseDecimal(positive, significand, exponentBeforeDecimalPoint) {
    this.eatChar();
    let exponentAfterDecimalPoint = 0;
    for (;;) {
      const c = this.peekOrNull();
      if (!isDigit(c)) break;
      const digit = BigInt(digitValue(c));
      if (overflows(significand, digit, U64_MAX)) {
        return this.parseDecimalOverflow(
          positive,
          significand,
          exponentBeforeDecimalPoint + exponentAfterDecimalPoint,
        );
      }
      this.eatChar();
      significand = significand * 10n + digit;
      exponentAfterDecimalPoint -= 1;
    }
    // At least one digit must follow the decimal point.
    if (exponentAfterDecimalPoint === 0) {
      throw this.peekError(this.peek() === null ? MSG.EofWhileParsingValue : MSG.InvalidNumber);
    }
    const exponent = exponentBeforeDecimalPoint + exponentAfterDecimalPoint;
    const c = this.peekOrNull();
    if (c === LOWER_E || c === UPPER_E) return this.parseExponent(positive, significand, exponent);
    return this.f64FromParts(positive, significand, exponent);
  }

  parseExponent(positive, significand, startingExp) {
    this.eatChar();
    let positiveExp = true;
    const sign = this.peekOrNull();
    if (sign === PLUS) {
      this.eatChar();
    } else if (sign === MINUS) {
      this.eatChar();
      positiveExp = false;
    }
    const next = this.next();
    if (next === null) throw this.error(MSG.EofWhileParsingValue);
    if (!isDigit(next)) throw this.error(MSG.InvalidNumber);
    let exp = digitValue(next);
    for (;;) {
      const c = this.peekOrNull();
      if (!isDigit(c)) break;
      this.eatChar();
      const digit = digitValue(c);
      if (overflows(BigInt(exp), BigInt(digit), BigInt(I32_MAX))) {
        return this.parseExponentOverflow(positive, significand === 0n, positiveExp);
      }
      exp = exp * 10 + digit;
    }
    const finalExp = positiveExp
      ? saturatingI32(startingExp + exp)
      : saturatingI32(startingExp - exp);
    return this.f64FromParts(positive, significand, finalExp);
  }

  parseExponentOverflow(positive, zeroSignificand, positiveExp) {
    if (!zeroSignificand && positiveExp) throw this.error(MSG.NumberOutOfRange);
    while (isDigit(this.peekOrNull())) this.eatChar();
    return positive ? 0 : -0;
  }

  parseLongInteger(positive, significand) {
    let exponent = 0;
    for (;;) {
      const c = this.peekOrNull();
      if (isDigit(c)) {
        this.eatChar();
        exponent += 1;
      } else if (c === DOT) {
        return this.parseDecimal(positive, significand, exponent);
      } else if (c === LOWER_E || c === UPPER_E) {
        return this.parseExponent(positive, significand, exponent);
      } else {
        return this.f64FromParts(positive, significand, exponent);
      }
    }
  }

  parseDecimalOverflow(positive, significand, exponent) {
    while (isDigit(this.peekOrNull())) this.eatChar();
    const c = this.peekOrNull();
    if (c === LOWER_E || c === UPPER_E) return this.parseExponent(positive, significand, exponent);
    return this.f64FromParts(positive, significand, exponent);
  }

  f64FromParts(positive, significand, exponent) {
    let f = Number(significand);
    for (;;) {
      const index = exponent === I32_MIN ? Infinity : Math.abs(exponent);
      if (index < POW10.length) {
        if (exponent >= 0) {
          f *= POW10[index];
          if (!Number.isFinite(f)) throw this.error(MSG.NumberOutOfRange);
        } else {
          f /= POW10[index];
        }
        break;
      }
      if (f === 0) break;
      if (exponent >= 0) throw this.error(MSG.NumberOutOfRange);
      f /= 1e308;
      exponent += 308;
    }
    return positive ? f : -f;
  }

  // --- error helpers ---

  // Consumes the offending value to describe it, e.g. "invalid type:
  // integer `5`, expected a string".
  peekInvalidType(expected) {
    const b = this.peekOrNull();
    let err;
    if (b === 0x6e) {
      this.eatChar();
      this.parseIdent('ull');
      err = custom(invalidType({ type: 'null' }, expected));
    } else if (b === 0x74) {
      this.eatChar();
      this.parseIdent('rue');
      err = custom(invalidType({ type: 'bool', value: true }, expected));
    } else if (b === 0x66) {
      this.eatChar();
      this.parseIdent('alse');
      err = custom(invalidType({ type: 'bool', value: false }, expected));
    } else if (b === MINUS) {
      this.eatChar();
      err = custom(invalidType(this.parseInteger(false), expected));
    } else if (isDigit(b)) {
      err = custom(invalidType(this.parseInteger(true), expected));
    } else if (b === QUOTE) {
      this.eatChar();
      err = custom(invalidType({ type: 'str', value: this.parseStr() }, expected));
    } else if (b === LBRACKET) {
      err = custom(invalidType({ type: 'seq' }, expected));
    } else if (b === LBRACE) {
      err = custom(invalidType({ type: 'map' }, expected));
    } else {
      err = this.peekError(MSG.ExpectedSomeValue);
    }
    return this.fixPosition(err);
  }

  // Skips any JSON value (IgnoredAny, used for unknown struct fields).
  ignoreValue() {
    const stack = [];
    let enclosing = null;
    for (;;) {
      const peek = this.parseWhitespace();
      if (peek === null) throw this.peekError(MSG.EofWhileParsingValue);
      let frame = null;
      if (peek === 0x6e) {
        this.eatChar();
        this.parseIdent('ull');
      } else if (peek === 0x74) {
        this.eatChar();
        this.parseIdent('rue');
      } else if (peek === 0x66) {
        this.eatChar();
        this.parseIdent('alse');
      } else if (peek === MINUS) {
        this.eatChar();
        this.ignoreInteger();
      } else if (isDigit(peek)) {
        this.ignoreInteger();
      } else if (peek === QUOTE) {
        this.eatChar();
        this.ignoreStr();
      } else if (peek === LBRACKET || peek === LBRACE) {
        if (enclosing !== null) stack.push(enclosing);
        enclosing = null;
        this.eatChar();
        frame = peek;
      } else {
        throw this.peekError(MSG.ExpectedSomeValue);
      }

      let acceptComma;
      if (frame !== null) {
        acceptComma = false;
      } else if (enclosing !== null) {
        frame = enclosing;
        enclosing = null;
        acceptComma = true;
      } else if (stack.length > 0) {
        frame = stack.pop();
        acceptComma = true;
      } else {
        return;
      }

      for (;;) {
        const b = this.parseWhitespace();
        if (b === COMMA && acceptComma) {
          this.eatChar();
          break;
        }
        const closes = (b === RBRACKET && frame === LBRACKET) || (b === RBRACE && frame === LBRACE);
        if (!closes) {
          if (b === null) {
            throw this.peekError(
              frame === LBRACKET ? MSG.EofWhileParsingList : MSG.EofWhileParsingObject,
            );
          }
          if (acceptComma) {
            throw this.peekError(
              frame === LBRACKET ? MSG.ExpectedListCommaOrEnd : MSG.ExpectedObjectCommaOrEnd,
            );
          }
          break;
        }
        this.eatChar();
        if (stack.length === 0) return;
        frame = stack.pop();
        acceptComma = true;
      }

      if (frame === LBRACE) {
        const b = this.parseWhitespace();
        if (b === QUOTE) {
          this.eatChar();
        } else {
          throw this.peekError(b === null ? MSG.EofWhileParsingObject : MSG.KeyMustBeAString);
        }
        this.ignoreStr();
        const colon = this.parseWhitespace();
        if (colon === COLON) {
          this.eatChar();
        } else {
          throw this.peekError(colon === null ? MSG.EofWhileParsingObject : MSG.ExpectedColon);
        }
      }
      enclosing = frame;
    }
  }

  ignoreStr() {
    const len = this.slice.length;
    for (;;) {
      while (this.index < len) {
        const b = this.slice[this.index];
        if (b === QUOTE || b === BACKSLASH || b < 0x20) break;
        this.index++;
      }
      if (this.index === len) throw this.error(MSG.EofWhileParsingString);
      const b = this.slice[this.index];
      if (b === QUOTE) {
        this.index++;
        return;
      }
      if (b !== BACKSLASH) throw this.error(MSG.ControlCharacterWhileParsingString);
      this.index++;
      const ch = this.next();
      if (ch === null) throw this.error(MSG.EofWhileParsingString);
      if (ch === LOWER_U) {
        this.decodeHexEscape();
      } else if (![QUOTE, BACKSLASH, 0x2f, 0x62, 0x66, 0x6e, 0x72, 0x74].includes(ch)) {
        throw this.error(MSG.InvalidEscape);
      }
    }
  }

  ignoreInteger() {
    const first = this.nextCharOrNull();
    if (first === 0x30) {
      if (isDigit(this.peekOrNull())) throw this.peekError(MSG.InvalidNumber);
    } else if (first >= 0x31 && first <= 0x39) {
      while (isDigit(this.peekOrNull())) this.eatChar();
    } else {
      throw this.error(MSG.InvalidNumber);
    }
    const c = this.peekOrNull();
    if (c === DOT) this.ignoreDecimal();
    else if (c === LOWER_E || c === UPPER_E) this.ignoreExponent();
  }

  ignoreDecimal() {
    this.eatChar();
    let atLeastOneDigit = false;
    while (isDigit(this.peekOrNull())) {
      this.eatChar();
      atLeastOneDigit = true;
    }
    if (!atLeastOneDigit) throw this.peekError(MSG.InvalidNumber);
    const c = this.peekOrNull();
    if (c === LOWER_E || c === UPPER_E) this.ignoreExponent();
  }

  ignoreExponent() {
    this.eatChar();
    const sign = this.peekOrNull();
    if (sign === PLUS || sign === MINUS) this.eatChar();
    if (!isDigit(this.nextCharOrNull())) throw this.error(MSG.InvalidNumber);
    while (isDigit(this.peekOrNull())) this.eatChar();
  }

  // --- typed deserialization ---

  deserialize(type) {
    switch (type.kind) {
      case 'string': return this.deserializeString();
      case 'option': return this.deserializeOption(type.inner);
      case 'enum': return this.deserializeEnum(type);
      case 'struct': return this.deserializeStruct(type);
      case 'map': return this.deserializeStringMap();
      default: throw new Error(`unsupported JSON body type ${type.kind}`);
    }
  }

  // Runs a deserialize_* body and stamps the current position on an error
  // that has none yet.
  positioned(body) {
    try {
      return body();
    } catch (err) {
      throw this.fixPosition(err);
    }
  }

  deserializeString() {
    const peek = this.parseWhitespace();
    if (peek === null) throw this.peekError(MSG.EofWhileParsingValue);
    return this.positioned(() => {
      if (peek !== QUOTE) throw this.peekInvalidType('a string');
      this.eatChar();
      return this.parseStr();
    });
  }

  deserializeOption(inner) {
    if (this.parseWhitespace() === 0x6e) {
      this.eatChar();
      this.parseIdent('ull');
      return null;
    }
    return this.deserialize(inner);
  }

  // A variant name as a string (e.g. "low"), or the externally tagged map
  // form serde also accepts for unit variants ({"low": null}).
  deserializeEnum(type) {
    const peek = this.parseWhitespace();
    if (peek === QUOTE) {
      return this.positioned(() => {
        this.eatChar();
        return this.variant(this.parseStr(), type);
      });
    }
    if (peek === LBRACE) {
      this.enterNested();
      this.eatChar();
      let value;
      try {
        const keyPeek = this.parseWhitespace();
        if (keyPeek === RBRACE) throw this.peekError(MSG.ExpectedSomeValue);
        if (keyPeek === null) throw this.peekError(MSG.EofWhileParsingObject);
        if (keyPeek !== QUOTE) throw this.peekError(MSG.KeyMustBeAString);
        this.eatChar();
        try {
          value = this.variant(this.parseStr(), type);
        } catch (err) {
          throw this.fixPosition(err);
        }
        this.parseObjectColon();
        this.deserializeUnit();
      } finally {
        this.remainingDepth += 1;
      }
      const close = this.parseWhitespace();
      if (close === RBRACE) {
        this.eatChar();
        return value;
      }
      throw this.error(close === null ? MSG.EofWhileParsingObject : MSG.ExpectedSomeValue);
    }
    throw this.peekError(peek === null ? MSG.EofWhileParsingValue : MSG.ExpectedSomeValue);
  }

  variant(name, type) {
    if (type.variants.includes(name)) return name;
    throw custom(unknownVariant(name, type.variants));
  }

  deserializeUnit() {
    const peek = this.parseWhitespace();
    if (peek === null) throw this.peekError(MSG.EofWhileParsingValue);
    return this.positioned(() => {
      if (peek !== 0x6e) throw this.peekInvalidType('unit');
      this.eatChar();
      this.parseIdent('ull');
      return null;
    });
  }

  deserializeStruct(type) {
    const peek = this.parseWhitespace();
    if (peek === null) throw this.peekError(MSG.EofWhileParsingValue);
    return this.positioned(() => {
      if (peek === LBRACKET) {
        return this.visitThenEnd(() => this.visitStructSeq(type), () => this.endSeq());
      }
      if (peek === LBRACE) {
        return this.visitThenEnd(() => this.visitStructMap(type), () => this.endMap());
      }
      throw this.peekInvalidType(`struct ${type.name}`);
    });
  }

  // serde's derived visit_seq: fields in declaration order, all required.
  visitStructSeq(type) {
    const out = {};
    let first = true;
    type.fields.forEach(([name, fieldType], i) => {
      if (!this.hasNextElement(first)) {
        const n = type.fields.length;
        throw custom(
          `invalid length ${i}, expected struct ${type.name} with ${n} element${n === 1 ? '' : 's'}`,
        );
      }
      first = false;
      out[name] = this.deserialize(fieldType);
    });
    return out;
  }

  hasNextElement(first) {
    const peek = this.parseWhitespace();
    if (peek === null) throw this.peekError(MSG.EofWhileParsingList);
    if (peek === RBRACKET) return false;
    if (first) return true;
    if (peek === COMMA) {
      this.eatChar();
      const next = this.parseWhitespace();
      if (next === RBRACKET) throw this.peekError(MSG.TrailingComma);
      if (next === null) throw this.peekError(MSG.EofWhileParsingValue);
      return true;
    }
    throw this.peekError(MSG.ExpectedListCommaOrEnd);
  }

  hasNextKey(first) {
    const peek = this.parseWhitespace();
    if (peek === null) throw this.peekError(MSG.EofWhileParsingObject);
    if (peek === RBRACE) return false;
    if (first) {
      if (peek === QUOTE) return true;
      throw this.peekError(MSG.KeyMustBeAString);
    }
    if (peek === COMMA) {
      this.eatChar();
      const next = this.parseWhitespace();
      if (next === QUOTE) return true;
      if (next === RBRACE) throw this.peekError(MSG.TrailingComma);
      if (next === null) throw this.peekError(MSG.EofWhileParsingValue);
      throw this.peekError(MSG.KeyMustBeAString);
    }
    throw this.peekError(MSG.ExpectedObjectCommaOrEnd);
  }

  // serde's derived visit_map: unknown keys are skipped, a repeated known key
  // is an error, then missing fields are reported in declaration order
  // (Option fields default to None).
  visitStructMap(type) {
    const values = new Map();
    let first = true;
    while (this.hasNextKey(first)) {
      first = false;
      this.eatChar();
      const key = this.parseStr();
      const field = type.fields.find(([name]) => name === key);
      if (field) {
        if (values.has(key)) throw custom(duplicateField(key));
        this.parseObjectColon();
        values.set(key, this.deserialize(field[1]));
      } else {
        this.parseObjectColon();
        this.ignoreValue();
      }
    }
    const out = {};
    for (const [name, fieldType] of type.fields) {
      if (values.has(name)) out[name] = values.get(name);
      else if (fieldType.kind === 'option') out[name] = null;
      else throw custom(missingField(name));
    }
    return out;
  }

  // HashMap<String, String>; a repeated key keeps the last value.
  deserializeStringMap() {
    const peek = this.parseWhitespace();
    if (peek === null) throw this.peekError(MSG.EofWhileParsingValue);
    return this.positioned(() => {
      if (peek !== LBRACE) throw this.peekInvalidType('a map');
      return this.visitThenEnd(() => {
        const map = new Map();
        let first = true;
        while (this.hasNextKey(first)) {
          first = false;
          this.eatChar();
          const key = this.parseStr();
          this.parseObjectColon();
          map.set(key, this.deserializeString());
        }
        return map;
      }, () => this.endMap());
    });
  }
}

// serde_json::from_slice::<T>
export function fromSlice(bytes, type) {
  const de = new Deserializer(bytes);
  const value = de.deserialize(type);
  de.end();
  return value;
}
