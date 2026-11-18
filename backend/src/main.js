import { Config } from './config.js';
import * as db from './db/index.js';
import { initLogger } from './logging.js';
import { createServer } from './server/app.js';

// A failed startup exits non-zero with the same message the Rust binary
// printed for its io::Error.
function fail(kind, message) {
  console.error(`Error: Custom { kind: ${kind}, error: ${JSON.stringify(message)} }`);
  process.exit(1);
}

async function step(promise, label) {
  try {
    return await promise;
  } catch (error) {
    return fail('Other', `${label}: ${error.message}`);
  }
}

async function main() {
  initLogger();

  let config;
  try {
    config = Config.fromEnv();
  } catch (error) {
    return fail('InvalidInput', `Missing required environment variable: ${error.message}`);
  }

  const client = await step(db.connect(config), 'MongoDB connection failed');
  const database = db.database(client, config);
  await step(db.ensureIndexes(database), 'Failed to create indexes');
  await step(db.backfillTicketContentUpdatedAt(database), 'Ticket backfill failed');
  await step(db.wipeLegacyChatMessages(database), 'Legacy chat wipe failed');

  const server = createServer({ db: database, config });
  server.on('error', (error) => fail('Other', error.message));
  server.listen(Number(config.port), '0.0.0.0');
}

main();
