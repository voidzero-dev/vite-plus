import fs from 'node:fs';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { findViteConfigUp, mergeLintConfig } from '../resolve-vite-config';

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

describe('mergeLintConfig', () => {
  it('should return rootLint when cwdLint is undefined', () => {
    const root = { plugins: ['import'], rules: { 'no-console': 'warn' } };
    expect(mergeLintConfig(root, undefined)).toEqual(root);
  });

  it('should return cwdLint when rootLint is undefined', () => {
    const cwd = { ignorePatterns: ['tests/**'] };
    expect(mergeLintConfig(undefined, cwd)).toEqual(cwd);
  });

  it('should return undefined when both are undefined', () => {
    expect(mergeLintConfig(undefined, undefined)).toBeUndefined();
  });

  it('should merge plugins as union with dedup', () => {
    const root = { plugins: ['import', 'react'] };
    const cwd = { plugins: ['import', 'unicorn'] };
    const merged = mergeLintConfig(root, cwd)!;
    expect(merged.plugins).toEqual(['import', 'react', 'unicorn']);
  });

  it('should shallow merge rules with cwd taking priority', () => {
    const root = { rules: { 'no-console': 'warn', 'import/no-commonjs': 'error' } };
    const cwd = { rules: { 'no-console': 'off' } };
    const merged = mergeLintConfig(root, cwd)!;
    expect(merged.rules).toEqual({
      'no-console': 'off',
      'import/no-commonjs': 'error',
    });
  });

  it('should use cwd ignorePatterns when present', () => {
    const root = { ignorePatterns: ['dist/**'] };
    const cwd = { ignorePatterns: ['tests/fixtures/**/*'] };
    const merged = mergeLintConfig(root, cwd)!;
    expect(merged.ignorePatterns).toEqual(['tests/fixtures/**/*']);
  });

  it('should keep root ignorePatterns when cwd has none', () => {
    const root = { ignorePatterns: ['dist/**'], rules: { 'no-console': 'warn' } };
    const cwd = { rules: { 'no-console': 'off' } };
    const merged = mergeLintConfig(root, cwd)!;
    expect(merged.ignorePatterns).toEqual(['dist/**']);
  });

  it('should shallow merge options with cwd taking priority', () => {
    const root = { options: { typeAware: false, denyWarnings: true } };
    const cwd = { options: { typeAware: true } };
    const merged = mergeLintConfig(root, cwd)!;
    expect(merged.options).toEqual({ typeAware: true, denyWarnings: true });
  });

  it('should concatenate overrides from root and cwd', () => {
    const root = { overrides: [{ files: ['*.ts'], rules: { a: 'off' } }] };
    const cwd = { overrides: [{ files: ['*.tsx'], rules: { b: 'off' } }] };
    const merged = mergeLintConfig(root, cwd)!;
    expect(merged.overrides).toEqual([
      { files: ['*.ts'], rules: { a: 'off' } },
      { files: ['*.tsx'], rules: { b: 'off' } },
    ]);
  });

  it('should merge all fields together (issue #997 scenario)', () => {
    const root = {
      plugins: ['import'],
      rules: { 'import/no-commonjs': 'error' },
    };
    const cwd = {
      options: { typeAware: true, typeCheck: true },
      ignorePatterns: ['tests/fixtures/**/*'],
    };
    const merged = mergeLintConfig(root, cwd)!;
    expect(merged.plugins).toEqual(['import']);
    expect(merged.rules).toEqual({ 'import/no-commonjs': 'error' });
    expect(merged.ignorePatterns).toEqual(['tests/fixtures/**/*']);
    expect(merged.options).toEqual({ typeAware: true, typeCheck: true });
  });
});
