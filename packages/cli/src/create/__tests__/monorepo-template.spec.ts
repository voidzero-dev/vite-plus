import { readFileSync } from 'node:fs';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { templatesDir } from '../../utils/path.js';

describe('monorepo template', () => {
  it('should keep the Yarn template free of pnpm-only catalog settings', () => {
    const yarnrc = readFileSync(path.join(templatesDir, 'monorepo', '_yarnrc.yml'), 'utf8');

    expect(yarnrc).toContain('catalog:');
    expect(yarnrc).not.toContain('catalogMode:');
  });
});
