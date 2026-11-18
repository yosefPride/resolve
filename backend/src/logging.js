// Minimal env_logger-style logging. LOG_LEVEL (error|warn|info|debug, default
// error) plays the role RUST_LOG did: quiet until something fails; set
// LOG_LEVEL=info to also get the per-request access log.

const LEVELS = { error: 1, warn: 2, info: 3, debug: 4, trace: 5 };

let threshold = LEVELS.error;

export function initLogger() {
  const level = (process.env.LOG_LEVEL ?? '').trim().toLowerCase();
  threshold = LEVELS[level] ?? LEVELS.error;
  // A log write that fails (e.g. the reader of a piped stderr went away) is
  // dropped, as env_logger does — it must not take the server down.
  process.stdout.on('error', () => {});
  process.stderr.on('error', () => {});
}

function write(level, target, message) {
  if (LEVELS[level] > threshold) return;
  const timestamp = new Date().toISOString().replace(/\.\d{3}Z$/, 'Z');
  process.stderr.write(`[${timestamp} ${level.toUpperCase().padEnd(5)} ${target}] ${message}\n`);
}

export const log = {
  error: (target, message) => write('error', target, message),
  info: (target, message) => write('info', target, message),
  enabled: (level) => LEVELS[level] <= threshold,
};
