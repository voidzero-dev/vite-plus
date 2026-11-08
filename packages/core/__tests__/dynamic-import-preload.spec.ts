import { mkdtempSync, realpathSync, writeFileSync } from 'node:fs';
import { rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { parseSync } from 'oxc-parser';
import { expect, test } from 'vite-plus/test';

// Exercise the bundled Vite and native Rolldown together, including minification
// and the final preload-marker replacements in generateBundle.
import { build } from '../dist/vite/node/index.js';

test.each([false, true])(
  'dynamic imports preserve syntax and preloads (legacy markers=%s)',
  async (legacyMarkers) => {
    const root = realpathSync.native(mkdtempSync(path.join(tmpdir(), 'vp-preload-markers-')));
    try {
      const files = {
        'index.html': '<script type="module" src="/main.js"></script>',
        'main.js': `
        globalThis.loadModules = () => import('./first.js').then(async ({ first }) => [
          first,
          (await import('./second.js')).second,
        ]);
        globalThis.loadStyles = () => import('./style-only.css');
      `,
        'first.js': "import './first.css'; export const first = 'first';",
        'first.css': '.first { color: red }',
        'second.js': "export const second = 'second';",
        'style-only.css': '.style-only { color: blue }',
      };
      for (const [name, source] of Object.entries(files)) {
        writeFileSync(path.join(root, name), source);
      }
      let legacyMarkerCount = 0;
      const result = await build({
        root,
        configFile: false,
        logLevel: 'silent',
        plugins: legacyMarkers
          ? [
              {
                name: 'legacy-preload-markers',
                renderChunk(code) {
                  return code.replaceAll(/__VITE_PRELOAD__[a-f0-9]{32}/g, () => {
                    legacyMarkerCount++;
                    return '__VITE_PRELOAD__';
                  });
                },
              },
            ]
          : [],
        build: { write: false, minify: true },
      });
      if (Array.isArray(result) || !('output' in result)) {
        throw new Error('Expected one build output');
      }
      if (legacyMarkers) {
        expect(legacyMarkerCount).toBeGreaterThan(0);
      }
      const chunks = result.output.filter((output) => output.type === 'chunk');
      expect(chunks.length).toBeGreaterThan(1);
      for (const chunk of chunks) {
        const errors = parseSync(chunk.fileName, chunk.code, { sourceType: 'module' }).errors;
        expect(
          errors.map((error) => error.message),
          chunk.fileName,
        ).toEqual([]);
        expect(chunk.code).not.toContain('__VITE_PRELOAD__');
      }
      const entry = chunks.find((chunk) => chunk.isEntry)!;
      expect(entry.code).toContain('__vite__mapDeps');
      const styles = result.output.filter(
        (output) => output.type === 'asset' && output.fileName.endsWith('.css'),
      );
      expect(styles).toHaveLength(2);
      for (const style of styles) {
        expect(entry.code).toContain(style.fileName);
      }
    } finally {
      await rm(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
    }
  },
);
