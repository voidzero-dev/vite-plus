import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import corePkg from '../../../core/package.json' with { type: 'json' };
import cliPkg from '../../package.json' with { type: 'json' };
import { resolveCore } from '../resolve-core.ts';

// The core version encodes the bundled Vite version, so it differs from the
// CLI version (rfcs/core-version-alignment.md).
const CORE_VERSION = corePkg.version;

function writePackageJson(directory: string, manifest: Record<string, unknown>): void {
  mkdirSync(directory, { recursive: true });
  writeFileSync(join(directory, 'package.json'), JSON.stringify(manifest));
}

function writeCore(
  directory: string,
  name = '@voidzero-dev/vite-plus-core',
  version = CORE_VERSION,
): void {
  writePackageJson(directory, {
    name,
    version,
    exports: { '.': './index.js', './pack': './pack.js', './package.json': './package.json' },
  });
  writeFileSync(join(directory, 'index.js'), '');
  writeFileSync(join(directory, 'pack.js'), '');
}

function writeCli(
  directory: string,
  version = cliPkg.version,
  viteSpec = `npm:@voidzero-dev/vite-plus-core@${CORE_VERSION}`,
): void {
  writePackageJson(directory, {
    name: 'vite-plus',
    version,
    dependencies: { vite: viteSpec },
    exports: { './package.json': './package.json' },
  });
}

describe('resolveCore', () => {
  let root: string;
  let project: string;
  let cliModule: string;
  let bundled: string;
  let projectCore: string;

  beforeEach(() => {
    root = realpathSync(mkdtempSync(join(tmpdir(), 'vp-core-resolver-')));
    project = join(root, 'project');
    writePackageJson(project, {
      devDependencies: { vite: `npm:@voidzero-dev/vite-plus-core@${CORE_VERSION}` },
    });
    cliModule = join(root, 'cli', 'dist', 'bin.js');
    bundled = join(root, 'cli', 'node_modules', 'vite');
    projectCore = join(project, 'node_modules', 'vite');
    writeCli(join(root, 'cli'));
    writeCore(bundled);
  });

  afterEach(() => {
    rmSync(root, { recursive: true, force: true });
  });

  it('supports a CLI installation without a project-level vite alias', () => {
    writePackageJson(project, { devDependencies: { 'vite-plus': cliPkg.version } });
    expect(resolveCore('', project, cliModule)).toBe(join(bundled, 'index.js'));
    expect(resolveCore('/pack', project, cliModule)).toBe(join(bundled, 'pack.js'));
  });

  it('ignores an upstream Vite peer hoisted by npm in a CLI-only project', () => {
    writePackageJson(project, { devDependencies: { 'vite-plus': cliPkg.version } });
    writeCore(projectCore, 'vite', '8.2.2');
    expect(resolveCore('', project, cliModule)).toBe(join(bundled, 'index.js'));
    expect(resolveCore('/pack', project, cliModule)).toBe(join(bundled, 'pack.js'));
  });

  it.each(['dependencies', 'devDependencies', 'optionalDependencies', 'peerDependencies'])(
    'validates an explicit Vite declaration in %s from a project subdirectory',
    (field) => {
      writePackageJson(project, { [field]: { vite: '^8.0.0' } });
      writeCore(projectCore, 'vite', '8.2.2');
      const subdirectory = join(project, 'src');
      mkdirSync(subdirectory);
      expect(() => resolveCore('', subdirectory, cliModule)).toThrow('found vite@8.2.2');
    },
  );

  it('keeps the same anchor as static exports when the project has a matching copy', () => {
    writeCore(projectCore);
    expect(resolveCore('', project, cliModule)).toBe(join(bundled, 'index.js'));
  });

  it.each([
    ['0.0.0', '8.3.599'],
    ['0.0.0-commit.1234567', '0.0.0-commit.1234567'],
  ])('uses the installed CLI manifest after repacking as %s', (version, coreVersion) => {
    writeCli(join(root, 'cli'), version, `npm:@voidzero-dev/vite-plus-core@${coreVersion}`);
    writeCore(bundled, '@voidzero-dev/vite-plus-core', coreVersion);
    writeCore(projectCore, '@voidzero-dev/vite-plus-core', coreVersion);
    expect(resolveCore('', project, cliModule)).toBe(join(bundled, 'index.js'));
    expect(resolveCore('/pack', project, cliModule)).toBe(join(bundled, 'pack.js'));
  });

  it.each(['project', 'bundled'])(
    'still rejects a mismatched %s core after repacking the CLI',
    (location) => {
      writeCli(join(root, 'cli'), '0.0.0', 'npm:@voidzero-dev/vite-plus-core@8.3.599');
      writeCore(bundled, '@voidzero-dev/vite-plus-core', '8.3.599');
      writeCore(location === 'project' ? projectCore : bundled);
      expect(() => resolveCore('', project, cliModule)).toThrow(
        `Expected @voidzero-dev/vite-plus-core@8.3.599, but found @voidzero-dev/vite-plus-core@${CORE_VERSION}`,
      );
    },
  );

  it('rejects a core that matches the CLI version instead of its alias', () => {
    writeCore(bundled, '@voidzero-dev/vite-plus-core', cliPkg.version);
    expect(() => resolveCore('', project, cliModule)).toThrow(
      `Expected @voidzero-dev/vite-plus-core@${CORE_VERSION}, but found @voidzero-dev/vite-plus-core@${cliPkg.version}`,
    );
  });

  describe('with the repository workspace spec', () => {
    beforeEach(() => {
      writeCli(join(root, 'cli'), cliPkg.version, 'workspace:@voidzero-dev/vite-plus-core@*');
      writeCore(bundled, '@voidzero-dev/vite-plus-core', '0.0.0-any');
    });

    it('compares the project core with the core that the CLI resolves', () => {
      writeCore(projectCore, '@voidzero-dev/vite-plus-core', '0.0.0-any');
      expect(resolveCore('', project, cliModule)).toBe(join(bundled, 'index.js'));
    });

    it('rejects a stale project core', () => {
      writeCore(projectCore, '@voidzero-dev/vite-plus-core', '0.0.0-stale');
      expect(() => resolveCore('', project, cliModule)).toThrow(
        'Expected @voidzero-dev/vite-plus-core@0.0.0-any, but found @voidzero-dev/vite-plus-core@0.0.0-stale',
      );
    });

    it('rejects upstream Vite in the CLI dependency', () => {
      writeCore(bundled, 'vite', '8.2.2');
      expect(() => resolveCore('', project, cliModule)).toThrow(
        'Expected @voidzero-dev/vite-plus-core, but found vite@8.2.2',
      );
    });
  });

  it('does not fall back to the project when the CLI dependency is missing', () => {
    writeCore(projectCore);
    rmSync(bundled, { recursive: true });
    // pnpm injects NODE_PATH, which can supply an unrelated Vite package.
    // Check the missing dependency in a process without global search paths.
    execFileSync(process.execPath, [
      '--no-global-search-paths',
      '--input-type=module',
      '--eval',
      `import { strict as assert } from 'node:assert';
       import { resolveCore } from ${JSON.stringify(new URL('../resolve-core.ts', import.meta.url).href)};
       assert.throws(
         () => resolveCore('', ${JSON.stringify(project)}, ${JSON.stringify(cliModule)}),
         /Could not resolve the bundled Vite/,
       );`,
    ]);
  });

  it.each(['project', 'bundled'])('rejects upstream Vite in the %s dependency', (location) => {
    writeCore(location === 'project' ? projectCore : bundled, 'vite', '8.2.2');
    expect(() => resolveCore('', project, cliModule)).toThrow('found vite@8.2.2');
  });

  it.each(['project', 'bundled'])('rejects a stale core in the %s dependency', (location) => {
    writeCore(
      location === 'project' ? projectCore : bundled,
      '@voidzero-dev/vite-plus-core',
      '0.0.0-stale',
    );
    expect(() => resolveCore('', project, cliModule)).toThrow(
      'found @voidzero-dev/vite-plus-core@0.0.0-stale',
    );
  });

  it('reports a missing core subpath instead of selecting a project copy', () => {
    writeCore(projectCore);
    rmSync(join(bundled, 'pack.js'));
    expect(() => resolveCore('/pack', project, cliModule)).toThrow();
  });
});
