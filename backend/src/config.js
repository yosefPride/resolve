import fs from 'node:fs';
import path from 'node:path';
import { parseEnv } from 'node:util';

// dotenvy::dotenv(): loads the first .env found in the working directory or
// any parent; variables already set in the environment win, and an unreadable
// file is ignored like a missing one.
function loadDotenv() {
  let dir = process.cwd();
  for (;;) {
    const candidate = path.join(dir, '.env');
    if (fs.existsSync(candidate)) {
      try {
        for (const [key, value] of Object.entries(parseEnv(fs.readFileSync(candidate, 'utf8')))) {
          if (process.env[key] === undefined) process.env[key] = value;
        }
      } catch {
        // ignored, as with dotenv().ok()
      }
      return;
    }
    const parent = path.dirname(dir);
    if (parent === dir) return;
    dir = parent;
  }
}

// std::env::VarError's Display text, used in the startup error message.
export class MissingEnvVar extends Error {
  constructor() {
    super('environment variable not found');
  }
}

function required(name) {
  const value = process.env[name];
  if (value === undefined) throw new MissingEnvVar();
  return value;
}

export class Config {
  static fromEnv() {
    loadDotenv();
    const config = new Config();
    config.mongoUri = required('MONGO_URI');
    config.jwtSecret = required('JWT_SECRET');
    // Whether the refresh-token cookie gets the `Secure` attribute. Defaults
    // to true (required in production); set COOKIE_SECURE=false for local
    // HTTP development, where a real browser would otherwise silently refuse
    // to store a Secure cookie at all.
    config.cookieSecure = (process.env.COOKIE_SECURE ?? 'true') !== 'false';
    // Origin the frontend is served from, for CORS. Needed explicitly (rather
    // than a wildcard) because the refresh-token cookie requires credentialed
    // cross-origin requests, which the CORS spec forbids combining with
    // `Access-Control-Allow-Origin: *`.
    config.frontendOrigin = process.env.FRONTEND_ORIGIN ?? 'http://localhost:5173';
    // Gemini API key. Optional, unlike MONGO_URI/JWT_SECRET: AI is advisory
    // (not required for correctness), so a missing key fails the AI endpoints
    // per request instead of keeping the whole backend from booting.
    config.geminiApiKey = process.env.GEMINI_API_KEY ?? null;
    // Database name (the Rust server hard-coded "resolve").
    config.mongoDb = process.env.MONGO_DB ?? 'resolve';
    config.port = process.env.PORT ?? '8080';
    return config;
  }
}
