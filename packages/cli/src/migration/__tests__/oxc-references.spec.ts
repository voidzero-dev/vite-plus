import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { finalizeCoreMigrationForExistingVitePlus } from '../migrator.ts';

describe('Oxc reference consumers survive recovery', () => {
  let project: string;
  let packages: { name: string; path: string }[];

  function write(filename: string, content: string | object) {
    const target = path.join(project, filename);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, typeof content === 'string' ? content : JSON.stringify(content));
  }

  function workspace(...names: string[]) {
    packages = names.map((name) => ({ name, path: `packages/${name}` }));
    for (const pkg of packages) {
      write(`${pkg.path}/package.json`, { name: pkg.name, type: 'module' });
      write(`${pkg.path}/vite.config.mjs`, 'export default {};');
    }
  }

  function recover() {
    return finalizeCoreMigrationForExistingVitePlus(
      {
        rootDir: project,
        packages,
      },
      true,
    );
  }

  function runNode(args: string[], directory = project) {
    return execFileSync(process.execPath, args, { cwd: directory, encoding: 'utf8' }).trim();
  }

  function loadConfig(directory = project) {
    // A fresh process gives each consumer its real cwd and avoids module caching.
    return JSON.parse(
      runNode(
        [
          '--input-type=module',
          '-e',
          `import { loadConfigFromFile } from ${JSON.stringify(import.meta.resolve('vite'))};
const loaded = await loadConfigFromFile({ command: 'build', mode: 'test' }, './vite.config.mjs', process.cwd(), 'silent', undefined, 'native');
console.log(JSON.stringify(loaded.config.define));`,
        ],
        directory,
      ),
    );
  }

  beforeEach(() => {
    project = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'vp-oxc-references-')));
    packages = [];
    write('package.json', {
      name: 'root',
      type: 'module',
      devDependencies: { 'vite-plus': 'latest' },
    });
    write('vite.config.mjs', 'export default {};');
  });

  afterEach(() => {
    fs.rmSync(project, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  });

  it.each([
    [
      'readFileSync',
      "import { readFileSync } from 'node:fs'; const fmt = JSON.parse(readFileSync('./.oxfmtrc.json', 'utf8'));",
      '.',
    ],
    [
      'aliased readFileSync',
      "import { readFileSync as read } from 'node:fs'; const fmt = JSON.parse(read('./.oxfmtrc.json', 'utf8'));",
      '.',
    ],
    [
      'new URL',
      "import { readFileSync } from 'node:fs'; const fmt = JSON.parse(readFileSync(new URL('./.oxfmtrc.json', import.meta.url), 'utf8'));",
      'packages/config',
    ],
    ['import', "import fmt from './.oxfmtrc.json' with { type: 'json' };", 'packages/config'],
  ])('resolves %s from the correct base', (_kind, helper, target) => {
    workspace('config');
    write('.oxfmtrc.json', { printWidth: 80 });
    write('packages/config/.oxfmtrc.json', { printWidth: 100 });
    write(
      'packages/config/helper.mjs',
      `${helper}\nexport default { define: { FORMAT_STYLE: fmt.printWidth } };`,
    );
    write(
      'vite.config.mjs',
      "import config from './packages/config/helper.mjs'; export default { ...config };",
    );
    const expected = { FORMAT_STYLE: target === '.' ? 80 : 100 };
    expect(loadConfig()).toEqual(expected);

    for (let run = 0; run < 2; run++) {
      recover();
      expect(loadConfig()).toEqual(expected);
      expect(fs.existsSync(path.join(project, target, '.oxfmtrc.json'))).toBe(true);
    }
    const unused = target === '.' ? 'packages/config' : '.';
    expect(fs.existsSync(path.join(project, unused, '.oxfmtrc.json'))).toBe(false);
  });

  it('tracks all working directories of a shared helper', () => {
    workspace('app', 'config');
    write('.oxfmtrc.json', { printWidth: 80 });
    write('packages/app/.oxfmtrc.json', { printWidth: 100 });
    write(
      'packages/config/helper.mjs',
      "import { readFileSync } from 'node:fs'; export default { define: { WIDTH: JSON.parse(readFileSync('./.oxfmtrc.json', 'utf8')).printWidth } };",
    );
    write(
      'vite.config.mjs',
      "import config from './packages/config/helper.mjs'; export default { ...config };",
    );
    write(
      'packages/app/vite.config.mjs',
      "import config from '../config/helper.mjs'; export default { ...config };",
    );
    const app = path.join(project, 'packages/app');
    expect(loadConfig()).toEqual({ WIDTH: 80 });
    expect(loadConfig(app)).toEqual({ WIDTH: 100 });

    recover();

    expect(loadConfig()).toEqual({ WIDTH: 80 });
    expect(loadConfig(app)).toEqual({ WIDTH: 100 });
  });

  it('preserves possible cwd dependencies after process.chdir', () => {
    workspace('app');
    write('packages/app/.oxfmtrc.json', { printWidth: 80 });
    write(
      'vite.config.mjs',
      "import { readFileSync } from 'node:fs'; process.chdir('packages/app'); const fmt = JSON.parse(readFileSync('./.oxfmtrc.json', 'utf8')); export default { define: { WIDTH: fmt.printWidth } };",
    );
    expect(loadConfig()).toEqual({ WIDTH: 80 });
    recover();
    expect(loadConfig()).toEqual({ WIDTH: 80 });
  });

  it('preserves configs when a module path cannot be resolved', () => {
    write('.oxfmtrc.json', { printWidth: 80 });
    write('vite.config.mjs', "if (false) await import('./missing.mjs'); export default {};");
    recover();
    expect(fs.existsSync(path.join(project, '.oxfmtrc.json'))).toBe(true);
  });

  it.each([
    'await import(`./${variant}.mjs`)',
    "await import('./' + variant + '.mjs')",
    "await import('./helper.mjs'.replace('helper', variant))",
  ])('preserves configs behind %s', (expression) => {
    write('.oxfmtrc.json', { printWidth: 80 });
    write(
      'helper.mjs',
      "import fmt from './.oxfmtrc.json' with { type: 'json' }; export default { define: { WIDTH: fmt.printWidth } };",
    );
    write(
      'vite.config.mjs',
      `const variant = 'helper'; const { default: config } = ${expression}; export default { ...config };`,
    );
    expect(loadConfig()).toEqual({ WIDTH: 80 });

    for (let run = 0; run < 2; run++) {
      recover();
      expect(loadConfig()).toEqual({ WIDTH: 80 });
    }
  });

  it.each([
    'node scripts/print-lint-rules.mjs',
    'node --no-warnings scripts/print-lint-rules.mjs',
    'bun scripts/print-lint-rules.mjs',
  ])('follows a script entry module: %s', (script) => {
    write('package.json', {
      name: 'root',
      type: 'module',
      scripts: { 'print-lint-rules': script },
    });
    write('.oxlintrc.json', { rules: { 'no-console': 'error' } });
    write(
      'scripts/print-lint-rules.mjs',
      "import lint from '../.oxlintrc.json' with { type: 'json' }; console.log(lint.rules['no-console']);",
    );
    expect(runNode(['scripts/print-lint-rules.mjs'])).toBe('error');

    for (let run = 0; run < 2; run++) {
      recover();
      expect(runNode(['scripts/print-lint-rules.mjs'])).toBe('error');
    }
  });

  it.each([
    'pnpm --dir packages/app exec',
    'pnpm --dir=packages/app exec',
    'pnpm -C packages/app exec',
    'yarn --cwd packages/app exec',
    'npm --prefix packages/app exec --',
    'bun --cwd packages/app run',
  ])('uses the directory selected by %s', (wrapper) => {
    workspace('app', 'other');
    write('package.json', {
      name: 'root',
      scripts: { fmt: `${wrapper} vp fmt -c .oxfmtrc.json src` },
    });
    write('packages/app/.oxfmtrc.json', { printWidth: 80 });
    write('packages/other/.oxfmtrc.json', { printWidth: 100 });
    const directory = path.join(project, 'packages/app');
    const read = () => runNode(['-p', "require('./.oxfmtrc.json').printWidth"], directory);
    expect(read()).toBe('80');

    for (let run = 0; run < 2; run++) {
      recover();
      expect(read()).toBe('80');
      expect(fs.existsSync(path.join(project, 'packages/other/.oxfmtrc.json'))).toBe(false);
    }
  });
});
