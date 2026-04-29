import fs from 'node:fs';
import path from 'node:path';
import url from 'node:url';

import { describe, expect, it } from 'vitest';

const coreDir = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), '..');
const distDir = path.join(coreDir, 'dist');

describe('build artifacts', () => {
  it('should include esm-shims.js in dist for tsdown shims support', () => {
    const shimsPath = path.join(distDir, 'esm-shims.js');
    expect(fs.existsSync(shimsPath), `${shimsPath} should exist`).toBe(true);

    const content = fs.readFileSync(shimsPath, 'utf8');
    expect(content).toContain('__dirname');
    expect(content).toContain('__filename');
  });

  it('should include tsdown client.d.ts in dist/tsdown for pack/client support', () => {
    const clientPath = path.join(distDir, 'tsdown/client.d.ts');
    expect(fs.existsSync(clientPath), `${clientPath} should exist`).toBe(true);

    const content = fs.readFileSync(clientPath, 'utf8');
    expect(content).toContain('ImportMeta');
    expect(content).toContain('glob');
  });
});
