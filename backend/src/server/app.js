import http from 'node:http';

import { ApiError, toApiError } from '../errors.js';
import { log } from '../logging.js';
import { ExtractError } from './extract.js';
import { firstHeader, headerStr } from './headers.js';
import { HttpResponse, errorResponse } from './response.js';
import { requotePath, splitTarget } from './router.js';
import { buildRouter } from './routes.js';

// --- CORS (actix-cors, configured as in main.rs) ---
//
// A wildcard origin cannot be combined with credentialed requests per the CORS
// spec — and the refresh cookie requires `credentials: 'include'` on the
// frontend's calls to be sent/received at all. So the origin must be explicit.
const ALLOWED_METHODS = new Set(['GET', 'POST', 'PATCH', 'PUT', 'DELETE']);
const ALLOWED_HEADERS = new Set(['authorization', 'content-type']);
const MAX_AGE_SECONDS = 3600;
const VARY = 'Origin, Access-Control-Request-Method, Access-Control-Request-Headers';
const TOKEN = /^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/;

function corsError(message) {
  return new HttpResponse(400).text(message, null);
}

function originAllowed(origin, config) {
  return origin !== null && Buffer.from(origin, 'latin1').equals(Buffer.from(config.frontendOrigin, 'utf8'));
}

function isPreflight(req) {
  if (req.method !== 'OPTIONS') return false;
  const method = headerStr(req, 'access-control-request-method');
  return method !== null && TOKEN.test(method);
}

function handlePreflight(req, config) {
  const origin = firstHeader(req, 'origin');
  if (origin === null) return corsError('Request header `Origin` is required but was not provided');
  if (!originAllowed(origin, config)) return corsError('Origin is not allowed to make this request');

  if (!ALLOWED_METHODS.has(headerStr(req, 'access-control-request-method'))) {
    return corsError('Requested method is not allowed');
  }
  if (firstHeader(req, 'access-control-request-headers') !== null) {
    const requested = headerStr(req, 'access-control-request-headers');
    if (requested === null) {
      return corsError('Request header `Access-Control-Request-Headers` has an invalid value');
    }
    const names = requested.split(',').map((name) => name.trim());
    if (names.some((name) => !TOKEN.test(name))) {
      return corsError('Request header `Access-Control-Request-Headers` has an invalid value');
    }
    if (!names.every((name) => ALLOWED_HEADERS.has(name.toLowerCase()))) {
      return corsError('One or more request headers are not allowed');
    }
  }

  return new HttpResponse(200)
    .header('access-control-allow-origin', origin)
    .header('access-control-allow-methods', [...ALLOWED_METHODS].join(', '))
    .header('access-control-allow-headers', [...ALLOWED_HEADERS].join(', '))
    .header('access-control-allow-credentials', 'true')
    .header('access-control-max-age', String(MAX_AGE_SECONDS))
    .header('vary', VARY);
}

function addCorsHeaders(response, origin) {
  if (origin !== null) response.header('access-control-allow-origin', origin);
  response.header('access-control-allow-credentials', 'true');
  response.header('vary', VARY);
  return response;
}

// --- dispatch ---

// Runs a handler's extractors the way actix resolves them, then the handler:
// synchronous checks in argument order, then the JSON body, then the
// database-backed guards (which in practice always finish after the body).
async function run(ctx, { extractors, fn }) {
  const values = extractors.map((extractor) => extractor.prepare?.(ctx));
  for (const [i, extractor] of extractors.entries()) {
    if (extractor.readBody) values[i] = await extractor.readBody(ctx, values[i]);
  }
  for (const [i, extractor] of extractors.entries()) {
    if (extractor.resolve) values[i] = await extractor.resolve(ctx, values[i]);
  }
  return fn(ctx, ...values);
}

async function dispatch(req, state, router) {
  const { path, query } = splitTarget(req.url);
  const matched = router.match(req.method, requotePath(path));
  if (!matched) return HttpResponse.notFound();
  const ctx = { req, state, params: matched.params, query };
  try {
    return await run(ctx, matched.handler);
  } catch (err) {
    if (err instanceof ExtractError) return err.response();
    if (!(err instanceof ApiError)) log.error('resolve::server', `unhandled error: ${err?.stack ?? err}`);
    return errorResponse(toApiError(err));
  }
}

function write(res, response) {
  res.statusCode = response.status;
  for (const [name, value] of response.headers) res.appendHeader(name, value);
  if (response.status !== 204) res.setHeader('content-length', response.body.length);
  res.end(response.body);
}

// actix's Logger::default() line: %a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T
function accessLog(req, response, startedAt) {
  if (!log.enabled('info')) return;
  const seconds = Number(process.hrtime.bigint() - startedAt) / 1e9;
  const remote = (req.socket.remoteAddress ?? '-').replace(/^::ffff:/, '');
  log.info(
    'actix_web::middleware::logger',
    `${remote} "${req.method} ${req.url} HTTP/${req.httpVersion}" ${response.status} ` +
      `${response.body.length} "${firstHeader(req, 'referer') ?? '-'}" ` +
      `"${firstHeader(req, 'user-agent') ?? '-'}" ${seconds.toFixed(6)}`,
  );
}

async function handle(req, res, state, router) {
  const startedAt = process.hrtime.bigint();
  if (isPreflight(req)) {
    write(res, handlePreflight(req, state.config));
    return;
  }
  const origin = firstHeader(req, 'origin');
  const response = await dispatch(req, state, router);
  accessLog(req, response, startedAt);
  write(res, addCorsHeaders(response, originAllowed(origin, state.config) ? origin : null));
}

export function createServer(state) {
  const router = buildRouter();
  // actix-web serves HTTP/1.1 requests that lack a Host header; Node would
  // reject them with 400 by default.
  return http.createServer({ requireHostHeader: false }, (req, res) => {
    handle(req, res, state, router).catch((err) => {
      log.error('resolve::server', `request failed: ${err?.stack ?? err}`);
      if (!res.headersSent) write(res, errorResponse(ApiError.internal()));
      else res.destroy();
    });
  });
}
