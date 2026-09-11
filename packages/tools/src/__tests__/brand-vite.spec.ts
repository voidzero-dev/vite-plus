import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

import { expect, test } from 'vitest';

import { brandVite } from '../brand-vite.ts';
import { alignVendoredVitestDependencies } from '../vendored-vitest.mjs';

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
    '      !process.env.VITE_CONFIG_NATIVE_IGNORE_WARNING &&',
    '        createNativeConfigCompatPlugin(nativeIncompatibilities),',
    '',
  ].join('\n'),
};

function write(file: string, source: string) {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, source);
}

test('branding preserves aligned dependencies and unrelated changes across repeated builds', () => {
  const root = mkdtempSync(join(tmpdir(), 'vp-brand-vite-'));
  const viteDir = join(root, 'vite');
  const nodeDir = join(viteDir, 'packages/vite/src/node');
  const git = (...args: string[]) => execFileSync('git', args, { cwd: viteDir, stdio: 'pipe' });
  try {
    for (const [file, source] of Object.entries(upstreamSources)) {
      write(join(nodeDir, file), source);
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

    mkdirSync(join(root, 'rolldown/packages'), { recursive: true });
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
