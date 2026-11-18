// Mirrors errors/api_error.rs: every failure a handler can return, with the
// exact status code, `code` and message the Rust server sends. Raw database or
// library errors are never surfaced (docs/api.md) — they collapse to Internal.

const KINDS = {
  InvalidCredentials: { status: 401, code: 'invalid_credentials', message: 'invalid email or password' },
  Unauthenticated: { status: 401, code: 'unauthenticated', message: 'missing or invalid authentication token' },
  Forbidden: { status: 403, code: 'forbidden', message: 'you do not have permission to perform this action' },
  NotFound: { status: 404, code: 'not_found', message: 'resource not found' },
  DuplicateEmail: { status: 409, code: 'duplicate_email', message: 'email already in use' },
  Conflict: { status: 409, code: 'conflict' },
  Validation: { status: 400, code: 'validation_error' },
  RateLimited: { status: 429, code: 'rate_limited' },
  AiUnavailable: {
    status: 503,
    code: 'ai_unavailable',
    message: 'the AI service is temporarily unavailable — please try again in a moment',
  },
  Internal: { status: 500, code: 'internal_error', message: 'internal server error' },
};

export class ApiError extends Error {
  constructor(kind, message) {
    const spec = KINDS[kind];
    super(message ?? spec.message);
    this.kind = kind;
    this.status = spec.status;
    this.code = spec.code;
  }

  static invalidCredentials() { return new ApiError('InvalidCredentials'); }
  static unauthenticated() { return new ApiError('Unauthenticated'); }
  static forbidden() { return new ApiError('Forbidden'); }
  static notFound() { return new ApiError('NotFound'); }
  static duplicateEmail() { return new ApiError('DuplicateEmail'); }
  static conflict(message) { return new ApiError('Conflict', message); }
  static validation(message) { return new ApiError('Validation', message); }
  static rateLimited(message) { return new ApiError('RateLimited', message); }
  static aiUnavailable() { return new ApiError('AiUnavailable'); }
  static internal() { return new ApiError('Internal'); }

  body() {
    return { error: { code: this.code, message: this.message } };
  }
}

// The `?`-operator conversion: anything that isn't already an ApiError (a
// MongoDB failure, a malformed stored document, ...) becomes Internal, same as
// the From impls in api_error.rs.
export function toApiError(error) {
  return error instanceof ApiError ? error : ApiError.internal();
}
