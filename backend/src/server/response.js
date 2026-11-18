// A response under construction, shaped like actix-web's HttpResponseBuilder
// so handlers read like their Rust counterparts.
export class HttpResponse {
  constructor(status) {
    this.status = status;
    this.headers = [];
    this.body = Buffer.alloc(0);
  }

  static ok() { return new HttpResponse(200); }
  static created() { return new HttpResponse(201); }
  static noContent() { return new HttpResponse(204); }
  static notFound() { return new HttpResponse(404); }

  header(name, value) {
    this.headers.push([name, value]);
    return this;
  }

  cookie(setCookie) {
    return this.header('set-cookie', setCookie);
  }

  // .json(): compact serde_json output; `raw` is an already-serialized body.
  json(value, raw) {
    this.header('content-type', 'application/json');
    this.body = Buffer.from(raw ?? JSON.stringify(value), 'utf8');
    return this;
  }

  // ResponseError::error_response's default: the Display text as plain text.
  text(message, contentType = 'text/plain; charset=utf-8') {
    if (contentType) this.header('content-type', contentType);
    this.body = Buffer.from(message, 'utf8');
    return this;
  }

  finish() {
    return this;
  }
}

export function errorResponse(apiError) {
  return new HttpResponse(apiError.status).json(apiError.body());
}
