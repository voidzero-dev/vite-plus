import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { readLintConfigInWorker } from '../compat/runner.ts';
import {
  collectRetainedImportDependencies,
  resolveImportRuleConfig,
  rewriteAllImports,
  tryResolveImportRuleConfig,
} from '../migrator.ts';
import { createMigrationReport } from '../report.ts';

vi.mock('../compat/runner.ts', () => ({ readLintConfigInWorker: vi.fn() }));

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
    rewriteAllImports(root, true, undefined, {
      importRuleConfig: await resolveImportRuleConfig(root),
    });
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
      expect(readLintConfigInWorker).not.toHaveBeenCalled();
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
    vi.mocked(readLintConfigInWorker).mockResolvedValue({
      rules: { [rule]: 'off' },
    });
    expect(await rewrite()).toBe(false);
    expect(readLintConfigInWorker).toHaveBeenCalledWith(root);
    expect(read('unit.test.ts')).toContain("from 'vitest'");
  });

  it('finds the nearest managed supplier for each import in an unmanaged template', async () => {
    write(
      'package.json',
      JSON.stringify({
        devDependencies: { vitest: '*', '@vitest/browser': '*', '@vitest/browser-preview': '*' },
      }),
    );
    write(
      'packages/app/package.json',
      JSON.stringify({
        optionalDependencies: { '@vitest/browser': '*' },
        peerDependencies: { '@vitest/browser-preview': '^5' },
      }),
    );
    write('packages/app/templates/starter/package.json', '{}');
    write(
      'packages/app/templates/starter/test.ts',
      "import 'vitest';\nimport '@vitest/browser/context';\nimport '@vitest/browser-preview';",
    );
    config({ overrides: [{ files: ['packages/app/templates/**'], rules: { [rule]: 'off' } }] });
    const workspace = { rootDir: root, packages: [{ name: 'app', path: 'packages/app' }] };
    expect(
      collectRetainedImportDependencies(workspace, await resolveImportRuleConfig(root)),
    ).toEqual({
      vitest: new Set([root]),
      browser: new Map([
        [path.join(root, 'packages/app'), new Set(['@vitest/browser', '@vitest/browser-preview'])],
        [root, new Set(['@vitest/browser-preview'])],
      ]),
    });
  });

  it('defers a failed dynamic load until the scheduled install completes', async () => {
    const report = createMigrationReport();
    config({});
    expect(await tryResolveImportRuleConfig(root)).not.toBeNull();
    expect(readLintConfigInWorker).not.toHaveBeenCalled();

    write('vite.config.ts', 'export default () => getConfig();');
    vi.mocked(readLintConfigInWorker).mockRejectedValueOnce(new Error('missing dependency'));
    expect(await tryResolveImportRuleConfig(root)).toBeNull();
    expect(report.warnings).toEqual([]);
    expect(read('unit.test.ts')).toContain("from 'vitest'");

    vi.mocked(readLintConfigInWorker).mockResolvedValue({
      rules: { [rule]: 'off' },
    });
    const importRuleConfig = await resolveImportRuleConfig(root, report);
    expect(rewriteAllImports(root, true, report, { importRuleConfig })).toBe(false);
    expect(read('unit.test.ts')).toContain("from 'vitest'");
    expect(report.warnings).toEqual([]);
    expect(readLintConfigInWorker).toHaveBeenCalledTimes(2);
  });

  it('warns and preserves imports when dynamic config cannot be loaded', async () => {
    write('vite.config.ts', 'export default () => getConfig();');
    // A failed Vite config must not fall back to an unrelated JSON rule setting.
    write('.oxlintrc.json', JSON.stringify({ rules: { [rule]: 'error' } }));
    vi.mocked(readLintConfigInWorker).mockRejectedValue(new Error('config worker crashed'));
    const report = createMigrationReport();
    const importRuleConfig = await resolveImportRuleConfig(root, report);
    expect(rewriteAllImports(root, true, report, { importRuleConfig })).toBe(false);
    expect(read('unit.test.ts')).toContain("from 'vitest'");
    expect(report.warnings).toEqual([
      expect.stringContaining('Skipped Vite/Vitest import rewrites'),
    ]);
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
