import { mkdtempSync, writeFileSync } from 'node:fs';
import { rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { parseSync } from 'oxc-parser';
import { expect, it, onTestFinished } from 'vite-plus/test';

import { build } from '../dist/vite/node/index.js';

it.each([false, true])(
  'replaces complete preload markers (inline imports: %s)',
  async (inlineDynamicImports) => {
    const root = mkdtempSync(path.join(tmpdir(), 'vp-preload-markers-'));
    onTestFinished(() => rm(root, { recursive: true, force: true, maxRetries: 5 }));
    writeFileSync(
      path.join(root, 'entry.js'),
      "globalThis.load = flag => flag ? import('./lazy.js') : import('./other.js');\n",
    );
    writeFileSync(path.join(root, 'lazy.js'), "import './lazy.css'; export default 'lazy';\n");
    writeFileSync(path.join(root, 'other.js'), "export default 'other';\n");
    writeFileSync(path.join(root, 'lazy.css'), '.lazy { color: red }\n');

    const result = await build({
      root,
      configFile: false,
      logLevel: 'silent',
      build: {
        write: false,
        minify: true,
        rolldownOptions: {
          input: path.join(root, 'entry.js'),
          output: { codeSplitting: !inlineDynamicImports },
        },
      },
    });
    if (!('output' in result)) {
      throw new Error('Expected a single build output');
    }
    const chunks = result.output.filter((output) => output.type === 'chunk');
    expect(chunks.length).toBeGreaterThan(0);
    for (const chunk of chunks) {
      expect(parseSync(chunk.fileName, chunk.code).errors).toEqual([]);
      expect(chunk.code).not.toContain('__VITE_PRELOAD__');
    }
    if (!inlineDynamicImports) {
      const css = result.output.find(
        (output) => output.type === 'asset' && output.fileName.endsWith('.css'),
      );
      expect(css).toBeDefined();
      expect(chunks.find((chunk) => chunk.isEntry)?.code).toContain(css!.fileName);
    }
  },
);
