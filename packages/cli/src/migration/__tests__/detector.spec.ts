import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import {
  collectOxcConfigConflicts,
  detectConfigs,
  detectOxcConfigConflicts,
  formatOxcConfigConflict,
} from '../detector.ts';

describe('detectConfigs — dynamic Oxc configs', () => {
  let tmpDir: string;

  afterEach(() => {
    if (tmpDir) {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  it.each([
    ['oxlint.config.ts', 'oxlintConfig'],
    ['oxlint.config.mts', 'oxlintConfig'],
    ['oxfmt.config.ts', 'oxfmtConfig'],
    ['oxfmt.config.mts', 'oxfmtConfig'],
  ] as const)('detects %s', (filename, configKey) => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-detector-'));
    fs.writeFileSync(path.join(tmpDir, filename), 'export default {};\n');

    expect(detectConfigs(tmpDir)[configKey]).toBe(filename);
  });

  // Documents the raw precedence only. `vp migrate` never reaches it for this
  // input: `assertNoOxcConfigConflicts` rejects the directory first.
  it('prefers JSON configs when both JSON and dynamic configs exist', () => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-detector-'));
    fs.writeFileSync(path.join(tmpDir, '.oxlintrc.json'), '{}\n');
    fs.writeFileSync(path.join(tmpDir, 'oxlint.config.ts'), 'export default {};\n');
    fs.writeFileSync(path.join(tmpDir, '.oxfmtrc.jsonc'), '{}\n');
    fs.writeFileSync(path.join(tmpDir, 'oxfmt.config.mts'), 'export default {};\n');

    expect(detectConfigs(tmpDir)).toMatchObject({
      oxlintConfig: '.oxlintrc.json',
      oxfmtConfig: '.oxfmtrc.jsonc',
    });
  });
});

describe('detectOxcConfigConflicts', () => {
  let tmpDir: string;

  const write = (filename: string) =>
    fs.writeFileSync(
      path.join(tmpDir, filename),
      filename.endsWith('.ts') || filename.endsWith('.mts') ? 'export default {};\n' : '{}\n',
    );

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-oxc-conflict-'));
  });

  afterEach(() => {
    if (tmpDir) {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  it('reports no conflict for a directory with no Oxc config at all', () => {
    expect(detectOxcConfigConflicts(tmpDir)).toEqual([]);
  });

  it.each([
    '.oxlintrc.json',
    '.oxlintrc.jsonc',
    'oxlint.config.ts',
    'oxlint.config.mts',
    '.oxfmtrc.json',
    'oxfmt.config.mts',
  ])('reports no conflict when only %s is present', (filename) => {
    write(filename);

    expect(detectOxcConfigConflicts(tmpDir)).toEqual([]);
  });

  // The rule both tools enforce is one config per directory, not one config
  // *form*: `.oxlintrc.json` + `.oxlintrc.jsonc` and `oxlint.config.ts` +
  // `oxlint.config.mts` fail the same way the mixed pair does, so every
  // two-config shape below is a conflict.
  it.each([
    ['oxlint', '.oxlintrc.json', 'oxlint.config.ts'],
    ['oxlint', '.oxlintrc.jsonc', 'oxlint.config.mts'],
    ['oxlint', '.oxlintrc.json', '.oxlintrc.jsonc'],
    ['oxlint', 'oxlint.config.ts', 'oxlint.config.mts'],
    ['oxfmt', '.oxfmtrc.json', 'oxfmt.config.ts'],
    ['oxfmt', '.oxfmtrc.jsonc', 'oxfmt.config.mts'],
    ['oxfmt', '.oxfmtrc.json', '.oxfmtrc.jsonc'],
    ['oxfmt', 'oxfmt.config.ts', 'oxfmt.config.mts'],
  ] as const)('flags %s when %s and %s coexist', (tool, firstConfig, secondConfig) => {
    write(firstConfig);
    write(secondConfig);

    expect(detectOxcConfigConflicts(tmpDir)).toEqual([
      { tool, dir: '.', configs: [firstConfig, secondConfig] },
    ]);
  });

  it('flags oxlint and oxfmt independently in the same directory', () => {
    write('.oxlintrc.json');
    write('oxlint.config.ts');
    write('.oxfmtrc.json');
    write('oxfmt.config.ts');

    expect(detectOxcConfigConflicts(tmpDir).map((conflict) => conflict.tool)).toEqual([
      'oxlint',
      'oxfmt',
    ]);
  });

  it('lists every config present, in the tool candidate order', () => {
    write('oxlint.config.mts');
    write('.oxlintrc.jsonc');
    write('oxlint.config.ts');
    write('.oxlintrc.json');

    expect(detectOxcConfigConflicts(tmpDir)).toEqual([
      {
        tool: 'oxlint',
        dir: '.',
        configs: ['.oxlintrc.json', '.oxlintrc.jsonc', 'oxlint.config.ts', 'oxlint.config.mts'],
      },
    ]);
  });

  it('carries the workspace-relative directory through for workspace packages', () => {
    write('.oxlintrc.json');
    write('oxlint.config.ts');

    expect(detectOxcConfigConflicts(tmpDir, 'packages/app')).toEqual([
      {
        tool: 'oxlint',
        dir: 'packages/app',
        configs: ['.oxlintrc.json', 'oxlint.config.ts'],
      },
    ]);
  });
});

describe('collectOxcConfigConflicts', () => {
  let tmpDir: string;

  const writeAt = (dir: string, filename: string) => {
    fs.mkdirSync(path.join(tmpDir, dir), { recursive: true });
    fs.writeFileSync(
      path.join(tmpDir, dir, filename),
      filename.endsWith('.ts') || filename.endsWith('.mts') ? 'export default {};\n' : '{}\n',
    );
  };

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-oxc-workspace-'));
  });

  afterEach(() => {
    if (tmpDir) {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  it('returns nothing for a clean workspace', () => {
    writeAt('.', '.oxlintrc.json');
    writeAt('packages/app', 'oxlint.config.ts');

    expect(collectOxcConfigConflicts(tmpDir, ['packages/app'])).toEqual([]);
  });

  it('finds a conflict in a workspace package, not only at the root', () => {
    writeAt('packages/app', '.oxlintrc.json');
    writeAt('packages/app', 'oxlint.config.ts');

    expect(collectOxcConfigConflicts(tmpDir, ['packages/app'])).toEqual([
      {
        tool: 'oxlint',
        dir: 'packages/app',
        configs: ['.oxlintrc.json', 'oxlint.config.ts'],
      },
    ]);
  });

  it('reports the root before the packages, each package in order', () => {
    for (const dir of ['.', 'packages/a', 'packages/b']) {
      writeAt(dir, '.oxlintrc.json');
      writeAt(dir, 'oxlint.config.ts');
    }

    expect(collectOxcConfigConflicts(tmpDir, ['packages/a', 'packages/b'])).toMatchObject([
      { dir: '.' },
      { dir: 'packages/a' },
      { dir: 'packages/b' },
    ]);
  });

  it('ignores a package directory that does not exist on disk', () => {
    expect(collectOxcConfigConflicts(tmpDir, ['packages/missing'])).toEqual([]);
  });

  it('checks only the root when no packages are passed', () => {
    writeAt('packages/app', '.oxlintrc.json');
    writeAt('packages/app', 'oxlint.config.ts');

    expect(collectOxcConfigConflicts(tmpDir)).toEqual([]);
  });
});

describe('formatOxcConfigConflict', () => {
  it('names the project root for a root-level conflict', () => {
    expect(
      formatOxcConfigConflict({
        tool: 'oxlint',
        dir: '.',
        configs: ['.oxlintrc.json', 'oxlint.config.ts'],
      }),
    ).toBe(
      'the project root has `.oxlintrc.json` and `oxlint.config.ts` — oxlint allows only one config per directory.',
    );
  });

  it('names the package directory for a workspace conflict', () => {
    expect(
      formatOxcConfigConflict({
        tool: 'oxfmt',
        dir: 'packages/app',
        configs: ['.oxfmtrc.json', 'oxfmt.config.mts'],
      }),
    ).toBe(
      'packages/app has `.oxfmtrc.json` and `oxfmt.config.mts` — oxfmt allows only one config per directory.',
    );
  });

  it('separates three or more configs with commas', () => {
    expect(
      formatOxcConfigConflict({
        tool: 'oxlint',
        dir: '.',
        configs: ['.oxlintrc.json', '.oxlintrc.jsonc', 'oxlint.config.ts'],
      }),
    ).toBe(
      'the project root has `.oxlintrc.json`, `.oxlintrc.jsonc` and `oxlint.config.ts` — oxlint allows only one config per directory.',
    );
  });
});
