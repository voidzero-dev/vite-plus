import fs from 'node:fs';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { findViteConfigUp, resolveViteConfig } from '../resolve-vite-config.js';

describe('findViteConfigUp', () => {
  let tempDir: string;

  beforeEach(() => {
    // Resolve symlinks (macOS /var -> /private/var) to match path.resolve behavior
    tempDir = fs.realpathSync(mkdtempSync(path.join(tmpdir(), 'vite-config-test-')));
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it('should find config in the start directory', () => {
    fs.writeFileSync(path.join(tempDir, 'vite.config.ts'), '');
    const result = findViteConfigUp(tempDir, tempDir);
    expect(result).toBe(path.join(tempDir, 'vite.config.ts'));
  });

  it('should find config in a parent directory', () => {
    const subDir = path.join(tempDir, 'packages', 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });
    fs.writeFileSync(path.join(tempDir, 'vite.config.ts'), '');

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBe(path.join(tempDir, 'vite.config.ts'));
  });

  it('should find config in an intermediate directory', () => {
    const subDir = path.join(tempDir, 'packages', 'my-lib', 'src');
    fs.mkdirSync(subDir, { recursive: true });
    fs.writeFileSync(path.join(tempDir, 'packages', 'vite.config.ts'), '');

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBe(path.join(tempDir, 'packages', 'vite.config.ts'));
  });

  it('should return undefined when no config exists', () => {
    const subDir = path.join(tempDir, 'packages', 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBeUndefined();
  });

  it('should not traverse beyond stopDir', () => {
    const parentConfig = path.join(tempDir, 'vite.config.ts');
    fs.writeFileSync(parentConfig, '');
    const stopDir = path.join(tempDir, 'packages');
    const subDir = path.join(stopDir, 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });

    const result = findViteConfigUp(subDir, stopDir);
    // Should not find the config in tempDir because stopDir is packages/
    expect(result).toBeUndefined();
  });

  it('should prefer the closest config file', () => {
    const subDir = path.join(tempDir, 'packages', 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });
    fs.writeFileSync(path.join(tempDir, 'vite.config.ts'), '');
    fs.writeFileSync(path.join(tempDir, 'packages', 'vite.config.ts'), '');

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBe(path.join(tempDir, 'packages', 'vite.config.ts'));
  });

  it('should find .js config files', () => {
    const subDir = path.join(tempDir, 'packages', 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });
    fs.writeFileSync(path.join(tempDir, 'vite.config.js'), '');

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBe(path.join(tempDir, 'vite.config.js'));
  });

  it('should find .mts config files', () => {
    const subDir = path.join(tempDir, 'packages', 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });
    fs.writeFileSync(path.join(tempDir, 'vite.config.mts'), '');

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBe(path.join(tempDir, 'vite.config.mts'));
  });

  it('should find .cjs config files', () => {
    const subDir = path.join(tempDir, 'packages', 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });
    fs.writeFileSync(path.join(tempDir, 'vite.config.cjs'), '');

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBe(path.join(tempDir, 'vite.config.cjs'));
  });

  it('should find .cts config files', () => {
    const subDir = path.join(tempDir, 'packages', 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });
    fs.writeFileSync(path.join(tempDir, 'vite.config.cts'), '');

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBe(path.join(tempDir, 'vite.config.cts'));
  });

  it('should find .mjs config files', () => {
    const subDir = path.join(tempDir, 'packages', 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });
    fs.writeFileSync(path.join(tempDir, 'vite.config.mjs'), '');

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBe(path.join(tempDir, 'vite.config.mjs'));
  });
});

describe('resolveViteConfig export conditions', () => {
  let tempDir: string;
  let originalNodeOptions: string | undefined;

  beforeEach(() => {
    tempDir = fs.realpathSync(mkdtempSync(path.join(tmpdir(), 'vite-config-conditions-test-')));
    originalNodeOptions = process.env.NODE_OPTIONS;
  });

  afterEach(() => {
    if (originalNodeOptions !== undefined) {
      process.env.NODE_OPTIONS = originalNodeOptions;
    } else {
      delete process.env.NODE_OPTIONS;
    }
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it('resolves self-referencing package via custom condition from NODE_OPTIONS (--conditions=dev)', async () => {
    const pkgJson = {
      name: '@test-scope/thing',
      type: 'module',
      exports: {
        './rules': {
          dev: './src/rules.ts',
          default: './dist/rules.js',
        },
      },
    };
    fs.writeFileSync(path.join(tempDir, 'package.json'), JSON.stringify(pkgJson, null, 2));

    const srcDir = path.join(tempDir, 'src');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.writeFileSync(
      path.join(srcDir, 'rules.ts'),
      'export const customRule = { name: "custom-rule-from-dev-condition" };\n',
    );

    const configTs = `
      import { customRule } from '@test-scope/thing/rules';
      export default {
        lint: {
          plugins: [customRule],
        },
      };
    `;
    fs.writeFileSync(path.join(tempDir, 'vite.config.ts'), configTs);

    process.env.NODE_OPTIONS = `${originalNodeOptions || ''} --conditions=dev`.trim();
    const config = await resolveViteConfig(tempDir);
    expect(config).toBeDefined();
    expect(config.lint?.plugins).toEqual([{ name: 'custom-rule-from-dev-condition' }]);
  });

  it('resolves self-referencing package via -C short flag in NODE_OPTIONS', async () => {
    const pkgJson = {
      name: '@test-scope/short-flag',
      type: 'module',
      exports: {
        './plugin': {
          custom: './src/plugin.ts',
          default: './dist/plugin.js',
        },
      },
    };
    fs.writeFileSync(path.join(tempDir, 'package.json'), JSON.stringify(pkgJson, null, 2));

    const srcDir = path.join(tempDir, 'src');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.writeFileSync(
      path.join(srcDir, 'plugin.ts'),
      'export const flagPlugin = { id: "short-flag-condition" };\n',
    );

    const configTs = `
      import { flagPlugin } from '@test-scope/short-flag/plugin';
      export default {
        lint: {
          plugins: [flagPlugin],
        },
      };
    `;
    fs.writeFileSync(path.join(tempDir, 'vite.config.ts'), configTs);

    process.env.NODE_OPTIONS = `${originalNodeOptions || ''} -C custom`.trim();
    const config = await resolveViteConfig(tempDir);
    expect(config).toBeDefined();
    expect(config.lint?.plugins).toEqual([{ id: 'short-flag-condition' }]);
  });

  it('fails to resolve without custom condition when dist does not exist', async () => {
    const pkgJson = {
      name: '@test-scope/fail-case',
      type: 'module',
      exports: {
        './rules': {
          dev: './src/rules.ts',
          default: './dist/rules.js',
        },
      },
    };
    fs.writeFileSync(path.join(tempDir, 'package.json'), JSON.stringify(pkgJson, null, 2));

    const srcDir = path.join(tempDir, 'src');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.writeFileSync(
      path.join(srcDir, 'rules.ts'),
      'export const customRule = { name: "custom-rule" };\n',
    );

    const configTs = `
      import { customRule } from '@test-scope/fail-case/rules';
      export default {
        lint: {
          plugins: [customRule],
        },
      };
    `;
    fs.writeFileSync(path.join(tempDir, 'vite.config.ts'), configTs);

    // No condition set in NODE_OPTIONS
    delete process.env.NODE_OPTIONS;
    await expect(resolveViteConfig(tempDir)).rejects.toThrow();
  });
});
