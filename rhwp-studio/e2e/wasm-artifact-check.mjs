import { createHash, randomUUID } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';

// Observe the bytes received during app initialization, not a second diagnostic fetch.
export function observeWasmArtifact(page, manifestPath = process.env.RHWP_WASM_BUILD_MANIFEST) {
  if (!manifestPath) return { finish: async () => {}, stop: () => {} };
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  const expected = manifest.artifacts?.['rhwp_bg.wasm']?.sha256;
  if (manifest.success !== true || !/^[a-f0-9]{64}$/.test(expected || '')) {
    throw new Error('WASM build manifest is incomplete or unsuccessful');
  }
  const responses = [];
  const listener = (response) => {
    if (!new URL(response.url()).pathname.endsWith('/rhwp_bg.wasm')) return;
    responses.push((async () => {
      try {
        const bytes = await response.buffer();
        return { url: response.url(), status: response.status(),
          sha256: createHash('sha256').update(bytes).digest('hex') };
      } catch (error) {
        return { url: response.url(), error: String(error) };
      }
    })());
  };
  page.on('response', listener);
  const stop = () => page.off('response', listener);
  return {
    stop,
    async finish() {
      stop();
      const received = await Promise.all(responses);
      const matched = received.length > 0 && received.every(
        (item) => item.status === 200 && item.sha256 === expected,
      );
      const directory = path.join(path.dirname(manifestPath), 'consumption');
      mkdirSync(directory, { recursive: true });
      writeFileSync(path.join(directory, `${process.pid}-${randomUUID()}.json`), JSON.stringify({
        source_sha: manifest.source_sha, profile: manifest.profile, expected_sha256: expected,
        page: page.url(), matched, received,
      }, null, 2) + '\n');
      if (!matched) throw new Error(`WASM artifact mismatch or missing response: expected ${expected}`);
      console.log(`  [wasm-artifact] matched ${expected}`);
    },
  };
}
