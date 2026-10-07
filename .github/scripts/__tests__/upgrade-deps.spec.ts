/// <reference types="node" />

import * as childProcess from 'node:child_process';
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

import { expect, onTestFinished, test, vi } from 'vite-plus/test';

vi.mock('node:child_process', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:child_process')>();
  return { ...actual, execFileSync: vi.fn(actual.execFileSync) };
});

test.each([false, true])(
  'runs the upgrade pipeline and retains metadata (install fails: %s)',
  async (failInstall) => {
    const root = resolve(import.meta.dirname, '../../..');
    const { root: tempDir, metaDir, corePath, pluginPath } = createSyncFixture();
    onTestFinished(() => {
      vi.mocked(childProcess.execFileSync).mockReset();
      vi.restoreAllMocks();
      vi.unstubAllGlobals();
      vi.unstubAllEnvs();
      vi.resetModules();
    });

    for (const file of [
      'pnpm-workspace.yaml',
      'packages/tools/.upstream-versions.json',
      'packages/cli/src/utils/constants.ts',
    ]) {
      mkdirSync(dirname(join(tempDir, file)), { recursive: true });
      copyFileSync(join(root, file), join(tempDir, file));
    }
    const workspacePath = join(tempDir, 'pnpm-workspace.yaml');
    writeFileSync(
      workspacePath,
      readFileSync(workspacePath, 'utf8').replace(/  lint-staged: .+/, '  lint-staged: ^16.2.6'),
    );

    vi.spyOn(process, 'cwd').mockReturnValue(tempDir);
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.stubEnv('UPGRADE_DEPS_META_DIR', metaDir);
    vi.stubGlobal('fetch', async (url: string) => {
      if (url.startsWith('https://api.github.com/repos/')) {
        return Response.json([{ name: 'v1.2.3', commit: { sha: 'a'.repeat(40) } }]);
      }
      if (url === 'https://registry.npmjs.org/vitest') {
        return Response.json({ versions: { '4.1.11': {}, '5.0.0': {} } });
      }
      if (url === 'https://registry.npmjs.org/@tsdown/css/latest') {
        return Response.json({ dependencies: { lightningcss: '^1.33.0' } });
      }
      if (url === 'https://registry.npmjs.org/lint-staged/latest') {
        return Response.json({ version: '17.5.1' });
      }
      if (url === 'https://registry.npmjs.org/oxlint/latest') {
        return Response.json({ version: '1.88.0' });
      }
      if (url === 'https://registry.npmjs.org/@oxlint/plugins/latest') {
        throw new Error('@oxlint/plugins must use the selected oxlint version');
      }
      if (url === 'https://registry.npmjs.org/oxlint-tsgolint/latest') {
        return Response.json({ version: '7.0.2004' });
      }
      if (url.startsWith('https://registry.npmjs.org/') && url.endsWith('/latest')) {
        return Response.json({ version: '1.2.3' });
      }
      throw new Error(`Unexpected request: ${url}`);
    });
    const commands: string[][] = [];
    vi.mocked(childProcess.execFileSync).mockImplementation((command, args) => {
      expect(command).toBe('pnpm');
      commands.push(args as string[]);
      if (failInstall) {
        throw new Error('Installation failed');
      }
      if (args?.[0] === 'dedupe') {
        // The actual bundled plugin can change during dependency deduplication.
        writeJson(pluginPath, {
          dependencies: { 'yuku-codegen': '^0.11.0', 'yuku-parser': '^0.11.0' },
        });
      }
      if (commands.length === 4) {
        expect(readJson(corePath).dependencies['yuku-parser']).toBe('^0.11.0');
      }
      return Buffer.from('');
    });

    vi.resetModules();
    if (failInstall) {
      await expect(import('../upgrade-deps.ts')).rejects.toThrow('Installation failed');
      expect(commands).toEqual([['install', '--no-frozen-lockfile']]);
      expect(readJson(corePath).dependencies['yuku-parser']).toBe('^0.9.3');
    } else {
      await import('../upgrade-deps.ts');
      expect(commands).toEqual([
        ['install', '--no-frozen-lockfile'],
        ['tool', 'sync-remote'],
        ['dedupe'],
        ['install', '--no-frozen-lockfile'],
      ]);
    }

    expect(readFileSync(workspacePath, 'utf8')).toContain('\n  lint-staged: ^17.5.1\n');
    expect(readFileSync(workspacePath, 'utf8')).toContain("\n  '@oxlint/plugins': =1.88.0\n");
    expect(readFileSync(workspacePath, 'utf8')).toContain('\n  oxlint: =1.88.0\n');
    expect(readFileSync(workspacePath, 'utf8')).toContain('\n  oxlint-tsgolint: =7.0.2004\n');
    const versions = JSON.parse(readFileSync(join(metaDir, 'versions.json'), 'utf8'));
    expect(versions['lint-staged']).toEqual({ old: '16.2.6', new: '17.5.1' });
    expect(versions['@oxlint/plugins'].new).toBe('1.88.0');
    expect(versions['@oxlint/plugins'].new).toBe(versions.oxlint.new);
    expect(versions['oxlint-tsgolint'].new).toBe('7.0.2004');
    if (!failInstall) {
      expect(versions['yuku-parser']).toEqual({ old: '^0.9.3', new: '^0.11.0' });
    }
    expect(readFileSync(join(metaDir, 'commit-message.txt'), 'utf8')).toContain(
      '- lint-staged: 16.2.6 -> 17.5.1',
    );
    expect(readFileSync(join(metaDir, 'pr-body.md'), 'utf8')).toContain(
      '| `lint-staged` | `16.2.6` | `17.5.1` |',
    );
  },
);

function writeJson(file: string, value: unknown) {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, JSON.stringify(value, null, 2) + '\n');
}

function readJson(file: string) {
  return JSON.parse(readFileSync(file, 'utf8'));
}

function createSyncFixture() {
  const root = mkdtempSync(join(tmpdir(), 'vite-plus-sync-bundled-deps-'));
  onTestFinished(() => rmSync(root, { recursive: true, force: true }));
  const corePath = join(root, 'packages/core/package.json');
  const pluginPath = join(root, 'store/tsdown/node_modules/rolldown-plugin-dts/package.json');
  const vitePath = join(root, 'vite/packages/vite/package.json');
  const metaDir = join(root, 'meta');
  const versionsPath = join(metaDir, 'versions.json');
  writeJson(corePath, {
    dependencies: { 'yuku-codegen': '^0.9.3', 'yuku-parser': '^0.9.3', postcss: '^8.5.6' },
    optionalDependencies: { fsevents: '~2.3.3' },
    devDependencies: { 'rolldown-plugin-dts': '^0.28.0' },
  });
  writeJson(join(root, 'store/tsdown/package.json'), { name: 'tsdown', version: '0.23.0' });
  writeJson(pluginPath, {
    dependencies: { 'yuku-codegen': '^0.10.1', 'yuku-parser': '~0.10.2' },
  });
  // A pnpm-style symlink ensures resolution starts at the real tsdown package.
  mkdirSync(join(root, 'packages/core/node_modules'), { recursive: true });
  symlinkSync(
    join(root, 'store/tsdown'),
    join(root, 'packages/core/node_modules/tsdown'),
    'junction',
  );
  writeJson(join(root, 'packages/core/node_modules/rolldown-plugin-dts/package.json'), {
    dependencies: { 'yuku-codegen': '^0.8.1', 'yuku-parser': '^0.8.1' },
  });
  writeJson(vitePath, {
    dependencies: { postcss: '^8.5.28' },
    optionalDependencies: { fsevents: '~2.4.0' },
  });
  writeJson(versionsPath, { oxlint: { old: '1.86.0', new: '1.87.0' } });
  const run = () =>
    childProcess.execFileSync(
      process.execPath,
      [resolve(import.meta.dirname, '../upgrade-deps.ts'), '--sync-bundled-deps'],
      {
        cwd: root,
        env: { ...process.env, UPGRADE_DEPS_META_DIR: metaDir },
        encoding: 'utf8',
        stdio: 'pipe',
      },
    );
  return { root, corePath, pluginPath, vitePath, metaDir, versionsPath, run };
}

test('syncs external ranges from bundled consumers and preserves upgrade metadata', () => {
  const fixture = createSyncFixture();
  const { corePath, versionsPath, metaDir, run } = fixture;
  run();
  expect(readJson(corePath)).toEqual({
    dependencies: { 'yuku-codegen': '^0.10.1', 'yuku-parser': '~0.10.2', postcss: '^8.5.28' },
    optionalDependencies: { fsevents: '~2.4.0' },
    devDependencies: { 'rolldown-plugin-dts': '^0.28.0' },
  });
  expect(readJson(versionsPath)).toEqual({
    oxlint: { old: '1.86.0', new: '1.87.0' },
    'yuku-codegen': { old: '^0.9.3', new: '^0.10.1' },
    'yuku-parser': { old: '^0.9.3', new: '~0.10.2' },
    postcss: { old: '^8.5.6', new: '^8.5.28' },
    fsevents: { old: '~2.3.3', new: '~2.4.0' },
  });
  expect(readFileSync(join(metaDir, 'pr-body.md'), 'utf8')).toContain(
    '| `yuku-parser` | `^0.9.3` | `~0.10.2` |',
  );

  // A second pass after lockfile deduplication must retain the original ranges.
  writeJson(fixture.pluginPath, {
    dependencies: { 'yuku-codegen': '^0.11.0', 'yuku-parser': '^0.11.0' },
  });
  run();
  expect(readJson(corePath).dependencies['yuku-codegen']).toBe('^0.11.0');
  expect(readJson(versionsPath)['yuku-codegen']).toEqual({ old: '^0.9.3', new: '^0.11.0' });
  expect(readFileSync(join(metaDir, 'commit-message.txt'), 'utf8')).toContain(
    '- yuku-codegen: ^0.9.3 -> ^0.11.0',
  );
  const before = readFileSync(corePath, 'utf8');
  run();
  expect(readFileSync(corePath, 'utf8')).toBe(before);
});

test.each([
  { source: 'pluginPath', field: 'dependencies', name: 'yuku-parser' },
  { source: 'vitePath', field: 'dependencies', name: 'postcss' },
  { source: 'vitePath', field: 'optionalDependencies', name: 'fsevents' },
] as const)('fails without writing core when upstream removes $name', ({ source, field, name }) => {
  const fixture = createSyncFixture();
  const pkg = readJson(fixture[source]);
  delete pkg[field][name];
  writeJson(fixture[source], pkg);
  const before = readFileSync(fixture.corePath, 'utf8');
  expect(fixture.run).toThrow("review core's externals");
  expect(readFileSync(fixture.corePath, 'utf8')).toBe(before);
  expect(readJson(fixture.versionsPath)).toEqual({
    oxlint: { old: '1.86.0', new: '1.87.0' },
  });
});
