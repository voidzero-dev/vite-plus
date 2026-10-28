import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

import { expect, test } from 'vitest';

import { brandVite } from '../brand-vite.ts';
import { assertViteConfigPatched } from '../patch-vite-config.ts';
import { alignVendoredVitestDependencies } from '../vendored-vitest.ts';

const upstreamSources = {
  'constants.ts': 'export const VERSION = version as string\n',
  'cli.ts': [
    "import { VERSION } from './constants'",
    "cac('vite')",
    'cli.version(VERSION)',
    'colors.green(',
    "            `${colors.bold('VITE')} v${VERSION}`,",
    '          )',
    '',
  ].join('\n'),
  'build.ts': [
    '  logger.info(',
    '    colors.cyan(',
    '      `vite v${VERSION} ${colors.green(',
    '        `building ${environment.name} environment for ${environment.config.mode}...`,',
    '      )}`,',
    '    ),',
    '  )',
    '`[vite]: Rolldown failed`',
    '',
  ].join('\n'),
  'logger.ts': "prefix = '[vite]'\n",
  'plugins/reporter.ts': [
    "import path from 'node:path'",
    '      logInfo: shouldLogInfo ? (msg) => env.logger.info(msg) : undefined,',
    '',
  ].join('\n'),
  'config.ts': [
    'export async function resolveConfig(',
    '  inlineConfig: InlineConfig,',
    "  command: 'build' | 'serve',",
    "  defaultMode = 'development',",
    "  defaultNodeEnv = 'development',",
    '  isPreview = false,',
    '  /** @internal */',
    '  patchConfig?: (config: ResolvedConfig) => void,',
    '  /** @internal */',
    '  patchPlugins?: (resolvedPlugins: Plugin[]) => void,',
    '): Promise<ResolvedConfig> {',
    '    if (loadResult) {',
    '      config = mergeConfig(loadResult.config, config)',
    '      configFile = loadResult.path',
    '      configFileDependencies = loadResult.dependencies',
    '    }',
    '}',
    '      !process.env.VITE_CONFIG_NATIVE_IGNORE_WARNING &&',
    '        createNativeConfigCompatPlugin(nativeIncompatibilities),',
    '',
  ].join('\n'),
};

function write(file: string, source: string) {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, source);
}

function createUpstreamFixture(configSource = upstreamSources['config.ts']): {
  root: string;
  nodeDir: string;
  manifest: string;
  lockfile: string;
  unrelatedSource: string;
} {
  const root = mkdtempSync(join(tmpdir(), 'vp-brand-vite-'));
  const viteDir = join(root, 'vite');
  const nodeDir = join(viteDir, 'packages/vite/src/node');
  const git = (...args: string[]) => execFileSync('git', args, { cwd: viteDir, stdio: 'pipe' });
  for (const [file, source] of Object.entries(upstreamSources)) {
    write(join(nodeDir, file), file === 'config.ts' ? configSource : source);
  }
  const manifest = join(viteDir, 'packages/vite/package.json');
  const lockfile = join(viteDir, 'pnpm-lock.yaml');
  const unrelatedSource = join(nodeDir, 'server.ts');
  write(manifest, '{"devDependencies":{"@vitest/utils":"4.1.10"}}\n');
  write(lockfile, '# upstream lockfile\n');
  write(unrelatedSource, '// upstream source\n');
  git('init');
  git('config', 'core.autocrlf', 'false');
  git('config', 'core.hooksPath', join(root, 'no-hooks'));
  git('add', '.');
  git(
    '-c',
    'user.name=Test',
    '-c',
    'user.email=test@example.com',
    '-c',
    'commit.gpgsign=false',
    'commit',
    '-m',
    'upstream fixture',
  );

  return { root, nodeDir, manifest, lockfile, unrelatedSource };
}

test('branding preserves aligned dependencies and unrelated changes across repeated builds', () => {
  const { root, nodeDir, manifest, lockfile, unrelatedSource } = createUpstreamFixture();
  try {
    alignVendoredVitestDependencies(root, '5.0.0');
    const alignedManifest = readFileSync(manifest, 'utf8');
    expect(JSON.parse(alignedManifest).devDependencies['@vitest/utils']).toBe('5.0.0');
    write(lockfile, '# local lockfile change\n');
    write(unrelatedSource, '// unrelated local source change\n');

    let brandedSources: string[] | undefined;
    for (let run = 0; run < 2; run++) {
      brandVite(root);

      expect(readFileSync(manifest, 'utf8')).toBe(alignedManifest);
      expect(readFileSync(lockfile, 'utf8')).toBe('# local lockfile change\n');
      expect(readFileSync(unrelatedSource, 'utf8')).toBe('// unrelated local source change\n');
      expect(readFileSync(join(nodeDir, 'cli.ts'), 'utf8')).toContain("cac('vp')");
      expect(readFileSync(join(nodeDir, 'build.ts'), 'utf8')).not.toContain('logger.info(');
      const configSource = readFileSync(join(nodeDir, 'config.ts'), 'utf8');
      expect(() => assertViteConfigPatched(configSource)).not.toThrow();
      expect(configSource).toContain('false &&\n        createNativeConfigCompatPlugin');
      expect(configSource).not.toContain('!process.env.VITE_CONFIG_NATIVE_IGNORE_WARNING');
      const sources = Object.entries(upstreamSources).map(([file, upstream]) => {
        const source = readFileSync(join(nodeDir, file), 'utf8');
        expect(source).not.toBe(upstream);
        return source;
      });
      if (brandedSources) {
        expect(sources).toEqual(brandedSources);
      }
      brandedSources = sources;
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('branding validates both config patches before writing config.ts', () => {
  const source = upstreamSources['config.ts'].replace('defaultNodeEnv', 'changedUpstreamParameter');
  const { root, nodeDir } = createUpstreamFixture(source);
  try {
    expect(() => brandVite(root)).toThrow(/changed, duplicate, or partial patch anchors/);
    expect(readFileSync(join(nodeDir, 'config.ts'), 'utf8')).toBe(source);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
