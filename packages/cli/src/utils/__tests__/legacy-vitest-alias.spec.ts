import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { findLegacyVitestAliasConfig } from '../legacy-vitest-alias.ts';

describe('findLegacyVitestAliasConfig', () => {
  const legacyAlias = 'npm:@voidzero-dev/vite-plus-test@0.1.24';
  let projectDir: string;

  function writePackageJson(config: Record<string, unknown>, directory?: string): string {
    const packageDir = directory ?? projectDir;
    const filePath = path.join(packageDir, 'package.json');
    fs.mkdirSync(packageDir, { recursive: true });
    fs.writeFileSync(filePath, JSON.stringify(config));
    return filePath;
  }

  beforeEach(() => {
    projectDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-legacy-vitest-alias-'));
  });

  afterEach(() => {
    fs.rmSync(projectDir, { recursive: true, force: true });
  });

  it('finds a nested package.json override alias', () => {
    const packagePath = writePackageJson({ overrides: { parent: { vitest: legacyAlias } } });

    expect(findLegacyVitestAliasConfig(projectDir)).toBe(packagePath);
  });

  it.each([
    'dependencies',
    'devDependencies',
    'optionalDependencies',
    'peerDependencies',
    'resolutions',
    'catalog',
  ])('finds aliases in package.json %s', (field) => {
    const packagePath = writePackageJson({ [field]: { vitest: legacyAlias } });

    expect(findLegacyVitestAliasConfig(projectDir)).toBe(packagePath);
  });

  it.each([
    { pnpm: { overrides: { vitest: legacyAlias } } },
    { catalogs: { testing: { vitest: legacyAlias } } },
    { workspaces: { packages: ['packages/*'], catalog: { vitest: legacyAlias } } },
    { workspaces: { packages: ['packages/*'], catalogs: { testing: { vitest: legacyAlias } } } },
  ])('finds aliases in nested package-manager settings: %j', (config) => {
    const packagePath = writePackageJson(config);

    expect(findLegacyVitestAliasConfig(projectDir)).toBe(packagePath);
  });

  it.each(['standalone', 'pnpm', 'npm'])('stops at the %s project boundary', (kind) => {
    const childDir = path.join(projectDir, 'child');
    const sourceDir = path.join(childDir, 'src');
    fs.mkdirSync(sourceDir, { recursive: true });
    writePackageJson({ devDependencies: { vitest: legacyAlias } });
    writePackageJson(
      {
        devDependencies: { vitest: '4.1.11' },
        ...(kind === 'npm' ? { workspaces: ['packages/*'] } : {}),
      },
      childDir,
    );
    if (kind === 'pnpm') {
      fs.writeFileSync(path.join(childDir, 'pnpm-workspace.yaml'), 'packages: [.]\n');
    }

    expect(findLegacyVitestAliasConfig(sourceDir)).toBeUndefined();
  });

  it('finds root npm overrides from a workspace member', () => {
    const packageDir = path.join(projectDir, 'packages', 'app');
    writePackageJson({}, packageDir);
    const packagePath = writePackageJson({
      workspaces: ['packages/*'],
      overrides: { vitest: legacyAlias },
    });

    expect(findLegacyVitestAliasConfig(packageDir)).toBe(packagePath);
  });

  it('ignores alias strings in metadata and unrelated settings', () => {
    writePackageJson({
      description: legacyAlias,
      config: { example: legacyAlias },
      pnpm: { metadata: legacyAlias },
      workspaces: { packages: [], metadata: legacyAlias },
      devDependencies: { vitest: '4.1.11' },
    });
    fs.writeFileSync(
      path.join(projectDir, 'pnpm-workspace.yaml'),
      `packages: [.]\nmetadata: ${legacyAlias}\ncatalog:\n  vitest: 4.1.11\n`,
    );

    expect(findLegacyVitestAliasConfig(projectDir)).toBeUndefined();
  });

  it.each([false, true])('handles cyclic YAML with a stale alias present: %s', (hasAlias) => {
    writePackageJson({});
    fs.writeFileSync(
      path.join(projectDir, 'pnpm-workspace.yaml'),
      [
        'packages: [.]',
        'metadata: &metadata',
        '  self: *metadata',
        'overrides: &overrides',
        '  self: *overrides',
        '  list: &list [*list]',
        `  vitest: ${hasAlias ? legacyAlias : '4.1.11'}`,
        '',
      ].join('\n'),
    );

    expect(findLegacyVitestAliasConfig(projectDir)).toBe(
      hasAlias ? path.join(projectDir, 'pnpm-workspace.yaml') : undefined,
    );
  });

  it('finds a pnpm catalog alias from a workspace package', () => {
    const packageDir = path.join(projectDir, 'packages', 'app');
    writePackageJson({}, packageDir);
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
