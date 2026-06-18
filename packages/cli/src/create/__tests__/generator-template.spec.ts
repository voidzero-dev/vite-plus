import { spawnSync } from 'node:child_process';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { templatesDir } from '../../utils/path.ts';

const generatorTemplateDir = path.join(templatesDir, 'generator');

describe('generator template', () => {
  // The scaffolded generator is executed directly with `node bin/index.ts`
  // (see discoverTemplate), so its imports must resolve under Node type
  // stripping, which does not remap `.js` specifiers to `.ts` files.
  it('bin/index.ts runs directly with node', () => {
    const result = spawnSync(process.execPath, ['bin/index.ts', '--help'], {
      cwd: generatorTemplateDir,
      encoding: 'utf8',
      timeout: 30_000,
    });

    expect(result.stderr).not.toContain('ERR_MODULE_NOT_FOUND');
    expect(result.status).toBe(0);
  });
});
