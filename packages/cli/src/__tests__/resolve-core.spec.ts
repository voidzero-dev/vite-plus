import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import cliPkg from '../../package.json' with { type: 'json' };
import { resolveCore } from '../resolve-core.ts';

function writeCore(
  directory: string,
  name = '@voidzero-dev/vite-plus-core',
  version = cliPkg.version,
) {
  mkdirSync(directory, { recursive: true });
  writeFileSync(
    join(directory, 'package.json'),
    JSON.stringify({
      name,
      version,
      exports: { '.': './index.js', './pack': './pack.js', './package.json': './package.json' },
    }),
  );
  writeFileSync(join(directory, 'index.js'), '');
  writeFileSync(join(directory, 'pack.js'), '');
}

function writeCli(directory: string, version = cliPkg.version) {
  mkdirSync(directory, { recursive: true });
  writeFileSync(
    join(directory, 'package.json'),
    JSON.stringify({
      name: 'vite-plus',
      version,
      exports: { './package.json': './package.json' },
    }),
  );
}

describe('resolveCore', () => {
  let root: string;
  let project: string;
  let cliModule: string;
  let bundled: string;

  beforeEach(() => {
    root = realpathSync(mkdtempSync(join(tmpdir(), 'vp-core-resolver-')));
    project = join(root, 'project');
    mkdirSync(project);
    writeFileSync(
      join(project, 'package.json'),
      JSON.stringify({
        devDependencies: { vite: `npm:@voidzero-dev/vite-plus-core@${cliPkg.version}` },
      }),
    );
    cliModule = join(root, 'cli', 'dist', 'bin.js');
    bundled = join(root, 'cli', 'node_modules', 'vite');
    writeCli(join(root, 'cli'));
    writeCore(bundled);
  });

  afterEach(() => {
    rmSync(root, { recursive: true, force: true });
  });

  it('supports a CLI installation without a project-level vite alias', () => {
    writeFileSync(
      join(project, 'package.json'),
      JSON.stringify({ devDependencies: { 'vite-plus': cliPkg.version } }),
    );
    expect(resolveCore('', project, cliModule)).toBe(join(bundled, 'index.js'));
    expect(resolveCore('/pack', project, cliModule)).toBe(join(bundled, 'pack.js'));
  });

  it('ignores an upstream Vite peer hoisted by npm in a CLI-only project', () => {
    writeFileSync(
      join(project, 'package.json'),
      JSON.stringify({ devDependencies: { 'vite-plus': cliPkg.version } }),
    );
    writeCore(join(project, 'node_modules', 'vite'), 'vite', '8.2.2');
    expect(resolveCore('', project, cliModule)).toBe(join(bundled, 'index.js'));
    expect(resolveCore('/pack', project, cliModule)).toBe(join(bundled, 'pack.js'));
  });

  it.each(['dependencies', 'devDependencies', 'optionalDependencies', 'peerDependencies'])(
    'validates an explicit Vite declaration in %s from a project subdirectory',
    (field) => {
      writeFileSync(join(project, 'package.json'), JSON.stringify({ [field]: { vite: '^8.0.0' } }));
      writeCore(join(project, 'node_modules', 'vite'), 'vite', '8.2.2');
      const subdirectory = join(project, 'src');
      mkdirSync(subdirectory);
      expect(() => resolveCore('', subdirectory, cliModule)).toThrow('found vite@8.2.2');
    },
  );

  it('keeps the same anchor as static exports when the project has a matching copy', () => {
    writeCore(join(project, 'node_modules', 'vite'));
    expect(resolveCore('', project, cliModule)).toBe(join(bundled, 'index.js'));
  });

  it.each(['0.0.0', '0.0.0-commit.1234567'])(
    'uses the installed CLI manifest after repacking as %s',
    (version) => {
      writeCli(join(root, 'cli'), version);
      writeCore(bundled, '@voidzero-dev/vite-plus-core', version);
      writeCore(join(project, 'node_modules', 'vite'), '@voidzero-dev/vite-plus-core', version);
      expect(resolveCore('', project, cliModule)).toBe(join(bundled, 'index.js'));
      expect(resolveCore('/pack', project, cliModule)).toBe(join(bundled, 'pack.js'));
    },
  );

  it.each(['project', 'bundled'])(
    'still rejects a mismatched %s core after repacking the CLI',
    (location) => {
      writeCli(join(root, 'cli'), '0.0.0');
      writeCore(bundled, '@voidzero-dev/vite-plus-core', '0.0.0');
      writeCore(location === 'project' ? join(project, 'node_modules', 'vite') : bundled);
      expect(() => resolveCore('', project, cliModule)).toThrow(
        `Expected @voidzero-dev/vite-plus-core@0.0.0, but found @voidzero-dev/vite-plus-core@${cliPkg.version}`,
      );
    },
  );

  it('does not fall back to the project when the CLI dependency is missing', () => {
    writeCore(join(project, 'node_modules', 'vite'));
    rmSync(bundled, { recursive: true });
    expect(() => resolveCore('', project, cliModule)).toThrow('Could not resolve the bundled Vite');
  });

  it.each(['project', 'bundled'])('rejects upstream Vite in the %s dependency', (location) => {
    writeCore(
      location === 'project' ? join(project, 'node_modules', 'vite') : bundled,
      'vite',
      '8.2.2',
    );
    expect(() => resolveCore('', project, cliModule)).toThrow('found vite@8.2.2');
  });

  it.each(['project', 'bundled'])('rejects a stale core in the %s dependency', (location) => {
    writeCore(
      location === 'project' ? join(project, 'node_modules', 'vite') : bundled,
      '@voidzero-dev/vite-plus-core',
      '0.0.0-stale',
    );
    expect(() => resolveCore('', project, cliModule)).toThrow(
      'found @voidzero-dev/vite-plus-core@0.0.0-stale',
    );
  });

  it('reports a missing core subpath instead of selecting a project copy', () => {
    writeCore(join(project, 'node_modules', 'vite'));
    rmSync(join(bundled, 'pack.js'));
    expect(() => resolveCore('/pack', project, cliModule)).toThrow();
  });
});
