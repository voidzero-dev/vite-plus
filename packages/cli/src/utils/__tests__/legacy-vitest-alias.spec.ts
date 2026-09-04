import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { findLegacyVitestAliasConfig } from '../legacy-vitest-alias.ts';

describe('findLegacyVitestAliasConfig', () => {
  let projectDir: string;

  beforeEach(() => {
    projectDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-legacy-vitest-alias-'));
  });

  afterEach(() => {
    fs.rmSync(projectDir, { recursive: true, force: true });
  });

  it('finds a nested package.json override alias', () => {
    fs.writeFileSync(
      path.join(projectDir, 'package.json'),
      JSON.stringify({
        overrides: {
          vitest: 'npm:@voidzero-dev/vite-plus-test@0.1.24',
        },
      }),
    );

    expect(findLegacyVitestAliasConfig(projectDir)).toBe(path.join(projectDir, 'package.json'));
  });

  it('finds a pnpm catalog alias from a workspace package', () => {
    const packageDir = path.join(projectDir, 'packages', 'app');
    fs.mkdirSync(packageDir, { recursive: true });
    fs.writeFileSync(path.join(packageDir, 'package.json'), '{}');
    fs.writeFileSync(
      path.join(projectDir, 'pnpm-workspace.yaml'),
      [
        'packages:',
        '  - packages/*',
        'catalog:',
        '  vitest: npm:@voidzero-dev/vite-plus-test@0.1.24',
        'overrides:',
        "  vitest: 'catalog:'",
        '',
      ].join('\n'),
    );

    expect(findLegacyVitestAliasConfig(packageDir)).toBe(
      path.join(projectDir, 'pnpm-workspace.yaml'),
    );
  });

  it('ignores current Vitest pins and malformed config files', () => {
    fs.writeFileSync(path.join(projectDir, 'package.json'), '{');
    fs.writeFileSync(
      path.join(projectDir, 'pnpm-workspace.yaml'),
      ['catalog:', '  vitest: 4.1.11', 'overrides:', '  vitest@*: 4.1.11', ''].join('\n'),
    );

    expect(findLegacyVitestAliasConfig(projectDir)).toBeUndefined();
  });
});
