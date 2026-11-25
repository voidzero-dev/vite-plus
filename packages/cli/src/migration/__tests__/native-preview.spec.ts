import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import type { JsonObject } from '../../utils/json.ts';
import { readJsonFile } from '../../utils/json.ts';
import { migrateNativePreviewPack } from '../migrator/native-preview.ts';
import { createMigrationReport } from '../report.ts';

const preview = { '@typescript/native-preview': '7.0.0-dev.20260707.2' };
const nativeConfig = "export default { pack: { dts: { generator: 'tsgo' } } };";
const upgraded = '7.1.0-beta';

describe('native-preview declaration migration', () => {
  let root: string;

  beforeEach(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-preview-migration-'));
  });
  afterEach(() => {
    fs.rmSync(root, { recursive: true, force: true });
  });

  function write(file: string, content: string | JsonObject) {
    const target = path.join(root, file);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, typeof content === 'string' ? content : JSON.stringify(content));
    return target;
  }

  function migrate(...members: Array<string>) {
    return migrateNativePreviewPack(
      {
        rootDir: root,
        packages: members.map((name) => ({ name, path: `packages/${name}` })),
        packageManager: PackageManager.pnpm,
        packageManagerVersion: '11.24.0',
      },
      createMigrationReport(),
    );
  }

  it('upgrades the shared compiler for an inheriting library and preserves a sibling compiler', () => {
    const manifest = write('package.json', {
      devDependencies: { typescript: '5.9.3', ...preview },
    });
    const library = write('packages/library/package.json', { name: 'library' });
    write('packages/library/vite.config.ts', nativeConfig);
    const sibling = { name: 'classic', devDependencies: { typescript: '6.0.3' } };
    const classic = write('packages/classic/package.json', sibling);

    expect(migrate('library', 'classic')).toEqual({ changed: true, warnings: [] });
    expect(readJsonFile(manifest)).toEqual({ devDependencies: { typescript: upgraded } });
    expect(readJsonFile(library)).toEqual({ name: 'library' });
    expect(readJsonFile(classic)).toEqual(sibling);
  });

  it('replaces the library catalog reference without changing its other consumers', () => {
    const catalog = 'packages:\n  - packages/*\ncatalog:\n  typescript: 5.9.3\n';
    const yaml = write('pnpm-workspace.yaml', catalog);
    const manifest = write('package.json', {
      devDependencies: { ...preview, typescript: 'catalog:' },
    });
    const library = write('packages/library/package.json', {
      devDependencies: { typescript: 'catalog:' },
    });
    write('packages/library/vite.config.ts', nativeConfig);
    const classic = write('packages/classic/package.json', {
      devDependencies: { typescript: 'catalog:' },
    });

    expect(migrate('library', 'classic')).toEqual({ changed: true, warnings: [] });
    expect(readJsonFile(library)).toEqual({ devDependencies: { typescript: upgraded } });
    expect(readJsonFile(manifest)).toEqual({ devDependencies: { typescript: 'catalog:' } });
    expect(readJsonFile(classic)).toEqual({ devDependencies: { typescript: 'catalog:' } });
    expect(fs.readFileSync(yaml, 'utf8')).toBe(catalog);
  });

  it.each([
    [
      'a typecheck script',
      'packages/app/package.json',
      { name: 'app', scripts: { typecheck: 'tsgo --noEmit' } },
    ],
    [
      'a compiler API import',
      'packages/app/codegen.mjs',
      "import * as ts from '@typescript/native-preview/unstable/async';",
    ],
  ] as const)('keeps the shared preview package for %s', (_, file, content) => {
    const manifest = write('package.json', {
      devDependencies: { typescript: '5.9.3', ...preview },
    });
    write('vite.config.ts', nativeConfig);
    write('packages/app/package.json', { name: 'app' });
    write(file, content);

    const result = migrate('app');
    expect(result.changed).toBe(true);
    expect(result.warnings).toEqual([expect.stringContaining('@typescript/native-preview')]);
    expect(readJsonFile(manifest)).toEqual({
      devDependencies: { typescript: upgraded, ...preview },
    });
  });

  it.each([
    ['a dynamic generator', '{ generator: process.env.DTS_GENERATOR }'],
    ['a custom native executable', "{ generator: 'tsgo', tsgo: { path: './tools/tsgo' } }"],
  ])('requests review for %s and preserves the config and manifest', (_, dts) => {
    const pkg = { devDependencies: { typescript: '6.0.3', ...preview } };
    const manifest = write('package.json', pkg);
    const contents = `export default { pack: { dts: ${dts} } };`;
    const config = write('vite.config.ts', contents);

    expect(migrate()).toEqual({
      changed: false,
      warnings: [expect.stringContaining('vite.config.ts')],
    });
    expect(fs.readFileSync(manifest, 'utf8')).toBe(JSON.stringify(pkg));
    expect(fs.readFileSync(config, 'utf8')).toBe(contents);
  });

  it.each([
    [
      'a classic compiler peer contract',
      {
        devDependencies: { typescript: '6.0.3', ...preview },
        peerDependencies: { typescript: '^5 || ^6' },
      },
      'package.json',
    ],
    [
      'a public preview peer',
      { devDependencies: { typescript: '6.0.3' }, peerDependencies: preview },
      'vite.config.ts',
    ],
    [
      'a custom compiler package',
      { devDependencies: { typescript: 'npm:@example/typescript@6.0.3', ...preview } },
      'vite.config.ts',
    ],
  ] as const)('requests review before changing %s', (_, pkg, file) => {
    const manifest = write('package.json', pkg);
    write('vite.config.ts', nativeConfig);

    expect(migrate()).toEqual({
      changed: false,
      warnings: [expect.stringContaining(file)],
    });
    expect(fs.readFileSync(manifest, 'utf8')).toBe(JSON.stringify(pkg));
  });

  it('respects the peer contract of a library that inherits the workspace compiler', () => {
    const pkg = { devDependencies: { typescript: '6.0.3', ...preview } };
    const manifest = write('package.json', pkg);
    const plugin = { name: 'compiler-plugin', peerDependencies: { typescript: '^5 || ^6' } };
    const member = write('packages/compiler-plugin/package.json', plugin);
    write('packages/compiler-plugin/vite.config.ts', nativeConfig);

    expect(migrate('compiler-plugin')).toEqual({
      changed: false,
      warnings: [expect.stringContaining('packages/compiler-plugin/package.json')],
    });
    expect(fs.readFileSync(manifest, 'utf8')).toBe(JSON.stringify(pkg));
    expect(fs.readFileSync(member, 'utf8')).toBe(JSON.stringify(plugin));
  });

  it('preserves the shared compiler when a sibling uses its code generation API', () => {
    const pkg = { devDependencies: { typescript: '6.0.3', ...preview } };
    const manifest = write('package.json', pkg);
    write('packages/library/package.json', { name: 'library' });
    write('packages/library/vite.config.ts', nativeConfig);
    write('packages/codegen/package.json', {
      name: 'codegen',
      scripts: { build: 'node build.mjs' },
    });
    write(
      'packages/codegen/build.mjs',
      "import ts from 'typescript';\nconst program = ts.createProgram(['src/index.ts'], {});\nprogram.emit();",
    );

    expect(migrate('library', 'codegen')).toEqual({
      changed: false,
      warnings: [expect.stringContaining('packages/codegen/package.json')],
    });
    expect(fs.readFileSync(manifest, 'utf8')).toBe(JSON.stringify(pkg));
  });

  it.each(['~7.0.2', '7.1.0-beta'])(
    'leaves a project already using typescript@%s alone',
    (typescript) => {
      const pkg = { devDependencies: { typescript, ...preview } };
      const manifest = write('package.json', pkg);
      write('vite.config.ts', nativeConfig);

      expect(migrate()).toEqual({ changed: false, warnings: [] });
      expect(fs.readFileSync(manifest, 'utf8')).toBe(JSON.stringify(pkg));
    },
  );
});
