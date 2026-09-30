import { resolveTypesetMode } from './core/table-v2-session.ts';

// Pick the runtime before importing main.ts: it installs editing, recovery and
// Legacy document handlers at module scope. V2 never starts those handlers.
if (resolveTypesetMode(location.search, import.meta.env.MODE) === 'v2') {
  const { startTableV2Studio } = await import('./table-v2-studio.ts');
  await startTableV2Studio();
} else {
  await import('./main.ts');
}
