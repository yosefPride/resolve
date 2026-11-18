// Request extractors with actix-web's behavior (web::Json, web::Query,
// web::Path). An extractor has up to three steps, which the dispatcher runs
// across all of a handler's extractors in the order actix resolves them:
//   prepare(ctx)          synchronous checks, in argument order
//   readBody(ctx, state)  reading and parsing the JSON body
//   resolve(ctx, state)   database-backed checks (membership, admin role)
// Each step may throw an ApiError or an ExtractError (a plain-text response).

import zlib from 'node:zlib';

import { fromSlice } from '../serde/json.js';
import { fromQuery } from '../serde/urlencoded.js';
import { headerStr } from './headers.js';
import { isJsonContentType } from './mime.js';
import { HttpResponse } from './response.js';
import { fullyDecode } from './router.js';

// An extractor rejection rendered with its Display text, e.g. "Content type
// error" with status 400.
export class ExtractError extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }

  response() {
    return new HttpResponse(this.status).text(this.message);
  }
}

const JSON_LIMIT = 2_097_152; // JsonConfig's default, 2 MiB

const DECOMPRESSORS = {
  gzip: () => zlib.createGunzip(),
  deflate: () => zlib.createInflate(),
  br: () => zlib.createBrotliDecompress(),
  zstd: () => zlib.createZstdDecompress(),
};

function contentEncoding(req) {
  const value = headerStr(req, 'content-encoding');
  return value === null ? 'identity' : value.trim().toLowerCase();
}

function payloadError(err) {
  return new ExtractError(400, `Error that occur during reading payload: ${err.message}`);
}

// Collects the (decompressed) body, failing once it exceeds `limit`.
function readPayload(req, limit) {
  return new Promise((resolve, reject) => {
    const make = DECOMPRESSORS[contentEncoding(req)];
    const stream = make ? req.pipe(make()) : req;
    const chunks = [];
    let size = 0;
    let done = false;
    const finish = (fn, value) => {
      if (done) return;
      done = true;
      fn(value);
    };
    stream.on('data', (chunk) => {
      if (done) return;
      size += chunk.length;
      if (size > limit) {
        finish(reject, new ExtractError(413, `JSON payload has exceeded limit (${limit} bytes).`));
        return;
      }
      chunks.push(chunk);
    });
    stream.on('end', () => finish(resolve, Buffer.concat(chunks)));
    stream.on('error', (err) => finish(reject, payloadError(err)));
    if (make) req.on('error', (err) => finish(reject, payloadError(err)));
  });
}

// web::Json<T>
export function json(type) {
  return {
    prepare(ctx) {
      if (!isJsonContentType(headerStr(ctx.req, 'content-type'))) {
        throw new ExtractError(400, 'Content type error');
      }
      const length = Number.parseInt(ctx.req.headers['content-length'] ?? '', 10);
      if (Number.isFinite(length) && length > JSON_LIMIT) {
        throw new ExtractError(
          413,
          `JSON payload (${length} bytes) is larger than allowed (limit: ${JSON_LIMIT} bytes).`,
        );
      }
      return null;
    },
    async readBody(ctx) {
      const bytes = await readPayload(ctx.req, JSON_LIMIT);
      try {
        return fromSlice(bytes, type);
      } catch (err) {
        if (err.constructor.name === 'JsonError') {
          throw new ExtractError(400, `Json deserialize error: ${err.message}`);
        }
        throw err;
      }
    },
  };
}

// web::Query<T>
export function query(type) {
  return {
    prepare(ctx) {
      try {
        return fromQuery(ctx.query, type);
      } catch (err) {
        if (err.constructor.name === 'QueryError') {
          throw new ExtractError(400, `Query deserialize error: ${err.message}`);
        }
        throw err;
      }
    },
  };
}

// web::Path<(String, ...)> — every matched segment, fully percent-decoded.
export const path = {
  prepare(ctx) {
    return ctx.params.map(([, value]) => fullyDecode(value));
  },
};

// HttpRequest
export const request = {
  prepare(ctx) {
    return ctx.req;
  },
};
