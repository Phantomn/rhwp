import { isTableV2Preview } from './core/table-v2-session.ts';

// The default and ?typeset=v2 use the full product editor. Only the explicit
// diagnostic preview skips editing/recovery handlers.
if (isTableV2Preview(location.search, import.meta.env.MODE)) {
  const { startTableV2Studio } = await import('./table-v2-studio.ts');
  await startTableV2Studio();
} else {
  await import('./main.ts');
}
