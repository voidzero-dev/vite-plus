import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { resolveViteConfig } from '../../resolve-vite-config.ts';
import { resolveImportRuleConfig, rewriteAllImports } from '../migrator.ts';

vi.mock('../../resolve-vite-config.ts', () => ({ resolveViteConfig: vi.fn() }));

describe('migration import rule settings', () => {
  let root: string;
  const rule = 'vite-plus/prefer-vite-plus-imports';
  const write = (filename: string, content: string) => {
    const file = path.join(root, filename);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, content);
  };
  const read = (filename: string) => fs.readFileSync(path.join(root, filename), 'utf8');
  const rewrite = async () =>
    rewriteAllImports(root, true, undefined, true, [], await resolveImportRuleConfig(root));
  const config = (lint: unknown) =>
    write('vite.config.ts', `export default ${JSON.stringify({ lint })};`);

  beforeEach(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-import-rule-'));
    write('package.json', '{"private":true}');
    write('unit.test.ts', "import { test } from 'vitest';");
  });
  afterEach(() => {
    fs.rmSync(root, { recursive: true, force: true });
    vi.resetAllMocks();
  });

  it.each(['off', 0, ['off'], [0], 'allow'].map((severity) => [severity]))(
    'honors disabled severity %j',
    async (severity) => {
      config({ rules: { [rule]: severity } });
      expect(await rewrite()).toBe(false);
      expect(read('unit.test.ts')).toContain("from 'vitest'");
      expect(resolveViteConfig).not.toHaveBeenCalled();
    },
  );

  it.each([undefined, 'warn', 'error', 1, 2, ['warn'], [2], 'deny'].map((severity) => [severity]))(
    'keeps the default rewrite for severity %j',
    async (severity) => {
      config({ rules: { [rule]: severity } });
      expect(await rewrite()).toBe(true);
      expect(read('unit.test.ts')).toContain("from 'vite-plus/test'");
    },
  );

  it('merges inherited rules before applying ordered overrides', async () => {
    config({
      extends: [
        {
          rules: { [rule]: 'off' },
          overrides: [{ files: ['templates/**'], rules: { [rule]: 0 } }],
        },
        { rules: { [rule]: 'error' } },
      ],
      overrides: [
        { files: ['templates/managed/**'], rules: { [rule]: ['warn'] } },
        { files: ['templates/**'], rules: { 'no-debugger': 'error' } },
      ],
    });
    for (const file of ['templates/starter/test.ts', 'templates/managed/test.ts']) {
      write(file, "export { test } from 'vitest';");
    }
    expect(await rewrite()).toBe(true);
    expect(read('unit.test.ts')).toContain("from 'vite-plus/test'");
    expect(read('templates/starter/test.ts')).toContain("from 'vitest'");
    expect(read('templates/managed/test.ts')).toContain("from 'vite-plus/test'");
    expect(await rewrite()).toBe(false);
  });

  it('resolves dynamic lint config when static extraction cannot read it', async () => {
    write('vite.config.ts', 'export default () => getConfig();');
    vi.mocked(resolveViteConfig, { partial: true }).mockResolvedValue({
      lint: { rules: { [rule]: 'off' } },
    });
    expect(await rewrite()).toBe(false);
    expect(resolveViteConfig).toHaveBeenCalledWith(root);
    expect(read('unit.test.ts')).toContain("from 'vitest'");
  });

  it('reads JSON config inheritance before the config migration removes it', async () => {
    write('base.json', JSON.stringify({ rules: { [rule]: 'off' } }));
    write('.oxlintrc.json', '{"extends":["./base.json"]}');
    expect(await rewrite()).toBe(false);
    expect(read('unit.test.ts')).toContain("from 'vitest'");
  });

  it('fails on an invalid explicit rule setting', async () => {
    config({ rules: { [rule]: 'disabled' } });
    await expect(rewrite()).rejects.toThrow('Invalid severity');
    expect(read('unit.test.ts')).toContain("from 'vitest'");
  });
});
