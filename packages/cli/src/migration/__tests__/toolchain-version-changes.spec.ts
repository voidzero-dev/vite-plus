import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import url from 'node:url';

import { afterAll, afterEach, beforeEach, describe, expect, it } from 'vitest';

import { VITE_PLUS_VERSION, VITEST_VERSION } from '../../utils/constants.js';

// `collectToolchainVersionChanges` reads the bundled raw Vite version via
// `await import('../versions.js')`. In the built CLI that resolves to
// dist/versions.js (rewritten by the tsdown fix-versions-path plugin). In
// source-mode tests the file does not exist, so create a stub at the resolved
// source path (src/migration/versions.ts) for the duration of this spec.
const STUB_BUNDLED_VITE = '8.1.0';
const here = path.dirname(url.fileURLToPath(import.meta.url));
const versionsSourcePath = path.resolve(here, '../versions.ts');
const createdVersionsStub = !fs.existsSync(versionsSourcePath);
if (createdVersionsStub) {
  fs.writeFileSync(
    versionsSourcePath,
    `export const versions = ${JSON.stringify({ vite: STUB_BUNDLED_VITE, vitest: VITEST_VERSION })};\n`,
  );
}

const { collectToolchainVersionChanges } = await import('../migrator.js');

afterAll(() => {
  if (createdVersionsStub && fs.existsSync(versionsSourcePath)) {
    fs.rmSync(versionsSourcePath);
  }
});

describe('collectToolchainVersionChanges', () => {
  let projectPath: string;

  beforeEach(() => {
    projectPath = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-toolchain-versions-'));
  });

  afterEach(() => {
    fs.rmSync(projectPath, { recursive: true, force: true });
  });

  function writeProject(): void {
    fs.writeFileSync(
      path.join(projectPath, 'package.json'),
      JSON.stringify(
        {
          name: 'upgrade-fixture',
          devDependencies: {
            'vite-plus': 'catalog:',
            vite: 'catalog:',
            vitest: 'catalog:',
            '@vitest/coverage-v8': 'catalog:',
            // Already on the target version: must be dropped (from === to).
            '@vitest/spy': 'catalog:',
          },
        },
        null,
        2,
      ),
    );
    fs.writeFileSync(
      path.join(projectPath, 'pnpm-workspace.yaml'),
      [
        'packages:',
        '  - .',
        '',
        'catalog:',
        '  vite-plus: 0.1.21',
        '  vite: "npm:@voidzero-dev/vite-plus-core@0.1.21"',
        '  vitest: 3.2.4',
        "  '@vitest/coverage-v8': 3.2.4",
        `  '@vitest/spy': ${VITEST_VERSION}`,
        '',
      ].join('\n'),
    );
    // node_modules/vite is the Vite+ core alias, so the RAW vite version lives
    // in bundledVersions.vite (not `version`, which is the alias version).
    const viteDir = path.join(projectPath, 'node_modules', 'vite');
    fs.mkdirSync(viteDir, { recursive: true });
    fs.writeFileSync(
      path.join(viteDir, 'package.json'),
      JSON.stringify({
        name: '@voidzero-dev/vite-plus-core',
        version: '0.1.21',
        bundledVersions: { vite: '8.0.0' },
      }),
    );
  }

  it('captures toolchain version changes, resolving catalog refs and the raw vite alias', async () => {
    writeProject();

    const changes = await collectToolchainVersionChanges(projectPath);

    expect(changes).toEqual([
      { name: 'vite-plus', from: '0.1.21', to: VITE_PLUS_VERSION },
      { name: 'vite', from: '8.0.0', to: STUB_BUNDLED_VITE },
      { name: 'vitest', from: '3.2.4', to: VITEST_VERSION },
      { name: '@vitest/coverage-v8', from: '3.2.4', to: VITEST_VERSION },
    ]);
    // A package whose old value already equals the target is omitted.
    expect(changes.some((change) => change.name === '@vitest/spy')).toBe(false);
  });

  it('reads the RAW vite version from a real upstream vite install', async () => {
    writeProject();
    // Replace the alias stub with a real upstream vite package.json.
    fs.writeFileSync(
      path.join(projectPath, 'node_modules', 'vite', 'package.json'),
      JSON.stringify({ name: 'vite', version: '7.1.0' }),
    );

    const changes = await collectToolchainVersionChanges(projectPath);

    expect(changes).toContainEqual({ name: 'vite', from: '7.1.0', to: STUB_BUNDLED_VITE });
  });

  it('excludes @vitest/eslint-plugin and concretizes a legacy wrapper / range from', async () => {
    fs.writeFileSync(
      path.join(projectPath, 'package.json'),
      JSON.stringify({
        name: 'wrapper-fixture',
        devDependencies: {
          'vite-plus': '^0.1.21',
          vitest: 'npm:@voidzero-dev/vite-plus-test@^0.1.21',
          // Versions independently — never aligned, so never a "change".
          '@vitest/eslint-plugin': '^1.6.0',
          '@vitest/ui': '3.2.4',
        },
      }),
    );

    const changes = await collectToolchainVersionChanges(projectPath);

    expect(changes).toContainEqual({ name: 'vite-plus', from: '0.1.21', to: VITE_PLUS_VERSION });
    expect(changes).toContainEqual({ name: 'vitest', from: '0.1.21', to: VITEST_VERSION });
    expect(changes).toContainEqual({ name: '@vitest/ui', from: '3.2.4', to: VITEST_VERSION });
    expect(changes.some((change) => change.name === '@vitest/eslint-plugin')).toBe(false);
  });

  it('skips a preserved protocol-pinned vite-plus and an unused bare vitest', async () => {
    fs.writeFileSync(
      path.join(projectPath, 'package.json'),
      JSON.stringify({
        name: 'pin-fixture',
        devDependencies: {
          // A file: pin is preserved by the migrator (no version change).
          'vite-plus': 'file:../custom-vite-plus',
          // Bare, unused: removed by the migrator (not bumped).
          vitest: '^3.0.0',
        },
      }),
    );

    const changes = await collectToolchainVersionChanges(projectPath);

    expect(changes.some((change) => change.name === 'vite-plus')).toBe(false);
    expect(changes.some((change) => change.name === 'vitest')).toBe(false);
  });

  it('leaves vite `from` undefined when node_modules/vite is absent', async () => {
    writeProject();
    fs.rmSync(path.join(projectPath, 'node_modules'), { recursive: true, force: true });

    const changes = await collectToolchainVersionChanges(projectPath);

    expect(changes).toContainEqual({ name: 'vite', to: STUB_BUNDLED_VITE });
  });
});
