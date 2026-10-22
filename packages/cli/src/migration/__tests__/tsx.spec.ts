import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@voidzero-dev/vite-plus-prompts', () => ({
  log: { info: vi.fn(), success: vi.fn(), warn: vi.fn() },
}));

import { detectTsxProject, migrateTsxToVpx, rewriteTsxCommand } from '../migrator/tsx.ts';
import { createMigrationReport } from '../report.ts';

describe('rewriteTsxCommand', () => {
  it.each([
    ['tsx ./src/seed.ts --flag', 'vpx ./src/seed.ts --flag'],
    ['tsx src/seed.ts', 'vpx src/seed.ts'],
    ['tsx watch src/server.ts', 'vpx --watch src/server.ts'],
    [
      'tsx watch --clear-screen=false --include "config/*.json" --exclude src/gen.ts --tsconfig tsconfig.app.json src/server.ts --port 3000',
      'vpx --watch --clear-screen=false --include "config/*.json" --exclude src/gen.ts --tsconfig tsconfig.app.json src/server.ts --port 3000',
    ],
    ['tsx watch --no-cache src/server.ts', 'vpx --watch src/server.ts'],
    ['tsx --no-cache ./a.ts', 'vpx ./a.ts'],
    ['tsx --no_cache ./a.ts', 'vpx ./a.ts'],
    ['tsx --watch ./a.ts', 'vpx --watch ./a.ts'],
    ['tsx --env-file=.env --inspect ./a.ts', 'vpx --env-file=.env --inspect ./a.ts'],
    ['tsx --env-file .env ./a.ts arg', 'vpx --env-file .env ./a.ts arg'],
    ['tsx --test ./a.test.ts', 'vpx --test ./a.test.ts'],
    // tsx loading itself again is redundant, and would compete with vpx.
    ['tsx --import tsx ./a.ts', 'vpx ./a.ts'],
    ['tsx -r tsx/cjs --import ./setup.ts ./a.cts', 'vpx --import ./setup.ts ./a.cts'],
    ["tsx 'my script.ts'", "vpx 'my script.ts'"],
    ['npx tsx ./a.ts', 'vpx ./a.ts'],
    ['npx --yes tsx ./a.ts', 'vpx ./a.ts'],
    ['npx -y tsx ./a.ts', 'vpx ./a.ts'],
    ['pnpm exec tsx ./a.ts', 'vpx ./a.ts'],
    ['pnpm dlx tsx ./a.ts', 'vpx ./a.ts'],
    ['pnpm tsx ./a.ts', 'vpx ./a.ts'],
    ['yarn tsx watch ./a.ts', 'vpx --watch ./a.ts'],
    ['bunx tsx ./a.ts', 'vpx ./a.ts'],
    ['NODE_ENV=test tsx ./a.ts', 'NODE_ENV=test vpx ./a.ts'],
    ['cross-env NODE_ENV=production tsx ./a.ts', 'cross-env NODE_ENV=production vpx ./a.ts'],
    ['tsx ./a.ts && tsx watch ./b.ts; vp build', 'vpx ./a.ts && vpx --watch ./b.ts; vp build'],
    ['node --import tsx ./a.ts', 'vpx ./a.ts'],
    ['node --import=tsx --inspect ./a.ts', 'vpx --inspect ./a.ts'],
    ['node --loader tsx/esm --watch ./a.ts', 'vpx --watch ./a.ts'],
    ['node -r tsx/cjs ./a.cts', 'vpx ./a.cts'],
    ['node --import tsx --import ./setup.ts ./a.ts', 'vpx --import ./setup.ts ./a.ts'],
    [
      'concurrently "tsx watch src/api.ts" "vp dev"',
      'concurrently "vpx --watch src/api.ts" "vp dev"',
    ],
    [
      'concurrently --names "api,web" "tsx watch src/api.ts" "vp dev"',
      'concurrently --names "api,web" "vpx --watch src/api.ts" "vp dev"',
    ],
    ["nodemon --exec 'tsx src/a.ts'", "nodemon --exec 'vpx src/a.ts'"],
    ["sh -c 'tsx src/a.ts && vp build'", "sh -c 'vpx src/a.ts && vp build'"],
    ["bash -lc 'tsx src/a.ts'", "bash -lc 'vpx src/a.ts'"],
    ["nodemon --watch src --exec 'tsx src/a.ts'", "nodemon --watch src --exec 'vpx src/a.ts'"],
    ['cross-env-shell "tsx src/a.ts"', 'cross-env-shell "vpx src/a.ts"'],
  ])('rewrites %s', (command, expected) => {
    const result = rewriteTsxCommand(command);
    expect(result.command).toBe(expected);
    expect(result.kept).toEqual([]);
    // Running the migration again changes nothing.
    expect(rewriteTsxCommand(expected)).toEqual({ command: expected, kept: [] });
  });

  it.each([
    ['vpx ./a.ts'],
    ['vp build'],
    ['oxlint src/app.tsx'],
    ['node ./a.js'],
    ['tsx-foo ./a.ts'],
  ])('leaves %s alone', (command) => {
    expect(rewriteTsxCommand(command)).toEqual({ command, kept: [] });
  });

  it.each([
    ["tsx -e 'console.log(1)'", '--eval or --print'],
    ['tsx --print 1', '--eval or --print'],
    ['tsx -c ./a.js', '-c or --check'],
    ['tsx', 'REPL'],
    ['tsx watch', 'REPL'],
    ['tsx --test', 'list the test files'],
    ['tsx src/index', '`src/index` has no file extension'],
    ['tsx --watch-path=src ./a.ts', '--watch-path'],
    ['tsx --watch_path=src ./a.ts', '--watch-path'],
    ['tsx ./a.ts | tee log', 'shell syntax'],
    ['tsx $ENTRY', 'shell syntax'],
    ['tsx %ENTRY%.ts', 'shell syntax'],
    ['.\\node_modules\\.bin\\tsx.cmd scripts/a.ts', 'shell syntax'],
    ['node --import tsx', 'REPL'],
    ['node --import tsx -e 1', '--eval or --print'],
    ['node --import tsx --watch-path src ./a.ts', '--watch-path'],
    ["NODE_OPTIONS='--import tsx' node ./a.ts", '`NODE_OPTIONS` loads tsx'],
    // A quoted argument of a program that does not run commands is data.
    ['echo "tsx scripts/build.ts"', 'does not rewrite this use of tsx'],
    // Only the value of `-c` is a command; the next argument is the shell's `$0`.
    [`sh -c 'echo "$0"' 'tsx a.ts'`, 'does not rewrite this use of tsx'],
    ['TSX_TSCONFIG_PATH=cli.json tsx ./a.ts', 'TSX_TSCONFIG_PATH'],
    ['TSX_TSCONFIG_PATH=cli.json node --import tsx ./a.ts', 'TSX_TSCONFIG_PATH'],
    ['cross-env-shell TSX_TSCONFIG_PATH=cli.json "tsx ./a.ts"', 'TSX_TSCONFIG_PATH'],
    ['TSX_TSCONFIG_PATH=cli.json concurrently "tsx ./a.ts"', 'TSX_TSCONFIG_PATH'],
    ['NODE_OPTIONS=--import=tsx tsx ./a.ts', 'NODE_OPTIONS'],
    // Windows reads environment variable names in any case.
    ['cross-env node_options=--import=tsx tsx ./a.ts', 'NODE_OPTIONS'],
    // Windows also finds the tsx bin as `TSX`.
    ['TSX ./a.ts', 'does not rewrite this use of tsx'],
    ['export TSX_TSCONFIG_PATH=cli.json && tsx ./a.ts', 'shell syntax'],
    ['tsx watch --test ./a.test.ts', '`vpx --watch` does not run `--test`'],
    ['tsx --watch --test ./a.test.ts', '`vpx --watch` does not run `--test`'],
    ['node --import tsx --watch --test ./a.test.ts', '`vpx --watch` does not run `--test`'],
  ])('keeps %s', (command, reason) => {
    const result = rewriteTsxCommand(command);
    expect(result.command).toBe(command);
    expect(result.kept).toEqual([expect.stringContaining(reason)]);
  });

  it('rewrites the parts of a command list that vpx can run', () => {
    const result = rewriteTsxCommand("tsx ./a.ts && tsx -e 'run()'");
    expect(result.command).toBe("vpx ./a.ts && tsx -e 'run()'");
    expect(result.kept).toEqual([expect.stringContaining('--eval')]);
  });

  describe('with the scripts it runs', () => {
    let dir: string;

    beforeEach(() => {
      dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-tsx-scripts-'));
      fs.writeFileSync(path.join(dir, 'package.json'), '{}\n');
      fs.writeFileSync(path.join(dir, 'esm.cts'), 'export const a = 1;\n');
      fs.writeFileSync(path.join(dir, 'cjs.cts'), 'exports.a = 1;\n');
      fs.writeFileSync(
        path.join(dir, 'dirname.ts'),
        "import fs from 'node:fs';\nfs.readdirSync(__dirname);\n",
      );
      fs.writeFileSync(
        path.join(dir, 'plain.ts'),
        "import fs from 'node:fs';\nfs.readdirSync('.');\n",
      );
      fs.writeFileSync(
        path.join(dir, 'exports.ts'),
        "import fs from 'node:fs';\nmodule.exports = fs.readdirSync('.');\n",
      );
      // ES module idioms that only look like CommonJS globals.
      fs.writeFileSync(
        path.join(dir, 'esm-idioms.ts'),
        [
          "import path from 'node:path';",
          "import { createRequire } from 'node:module';",
          "import { fileURLToPath } from 'node:url';",
          'const __dirname = path.dirname(fileURLToPath(import.meta.url));',
          'const require = createRequire(import.meta.url);',
          "const pkg = require(path.join(__dirname, 'package.json'));",
          'console.log(pkg.exports.x);',
          '',
        ].join('\n'),
      );
      // `cd tools` stops the checks; this script would keep tsx otherwise.
      fs.mkdirSync(path.join(dir, 'tools'));
      fs.writeFileSync(
        path.join(dir, 'tools/only.ts'),
        "import fs from 'node:fs';\nfs.readdirSync(__dirname);\n",
      );
      fs.writeFileSync(path.join(dir, 'tools/plain.ts'), 'export {};\n');
      // tsx resolves `require('./helper.js')` to `helper.ts`; `vpx` does not.
      fs.writeFileSync(path.join(dir, 'helper.ts'), 'exports.x = 1;\n');
      fs.writeFileSync(path.join(dir, 'require-js.cts'), "const { x } = require('./helper.js');\n");
      fs.writeFileSync(path.join(dir, 'require-bare.cts'), "const { x } = require('./helper');\n");
      // Compact ES module syntax, and CommonJS that only looks like it.
      fs.writeFileSync(path.join(dir, 'compact.ts'), 'export{};console.log(__dirname);\n');
      fs.writeFileSync(
        path.join(dir, 'import-equals.cts'),
        "import fs = require('node:fs');\nexport = fs.readdirSync(__dirname);\n",
      );
      fs.writeFileSync(path.join(dir, 'importer.ts'), 'importer.run(__dirname);\n');
      fs.writeFileSync(
        path.join(dir, 'commented.ts'),
        "import fs from 'node:fs';\n// const require = createRequire(import.meta.url);\nrequire('./x');\n",
      );
      // A CommonJS global named only in a comment is not a use.
      fs.writeFileSync(
        path.join(dir, 'comment-only.ts'),
        "import fs from 'node:fs';\n// no require() or __dirname here\nfs.readdirSync('.');\n",
      );
      // A `//` in a string does not start a comment that hides the rest of the line.
      fs.writeFileSync(
        path.join(dir, 'string-url.ts'),
        "import fs from 'node:fs'; const url = 'http://x'; fs.readdirSync(__dirname);\n",
      );
      fs.writeFileSync(
        path.join(dir, 'export-assign.ts'),
        "import fs from 'node:fs';\nexport = fs;\n",
      );
      fs.writeFileSync(
        path.join(dir, 'bracket.ts'),
        "import fs from 'node:fs';\nexports['result'] = fs.statSync('.');\n",
      );
      // The declaration covers only the function, not the module-level `require`.
      fs.writeFileSync(
        path.join(dir, 'inner.ts'),
        [
          "import { createRequire } from 'node:module';",
          'function load() {',
          '  const require = createRequire(import.meta.url);',
          "  return require('./x');",
          '}',
          "require('./setup');",
          '',
        ].join('\n'),
      );
      fs.mkdirSync(path.join(dir, 'tests'));
      fs.writeFileSync(path.join(dir, 'tests/ok.test.ts'), 'export {};\n');
      fs.writeFileSync(
        path.join(dir, 'banner.ts'),
        "/* banner */ import fs from 'node:fs'; fs.readdirSync(__dirname);\n",
      );
      fs.writeFileSync(
        path.join(dir, 'cli-guard.ts'),
        "import fs from 'node:fs';\nif (require.main === module) fs.readdirSync('.');\n",
      );
      fs.writeFileSync(path.join(dir, 'builtin.cts'), "const fs = require('node:fs');\n");
      fs.writeFileSync(
        path.join(dir, 'module-filename.ts'),
        "import fs from 'node:fs';\nconsole.log(module.filename);\n",
      );
      fs.writeFileSync(
        path.join(dir, 'node-module.ts'),
        "import module from 'node:module';\nconst require = module.createRequire(import.meta.url);\nrequire('node:fs');\n",
      );
      fs.writeFileSync(
        path.join(dir, 'template-require.cts'),
        'const { x } = require(`./helper.js`);\n',
      );
    });

    afterEach(() => {
      fs.rmSync(dir, { recursive: true, force: true });
    });

    it.each([
      ['tsx esm.cts', '`esm.cts` uses import or export'],
      ['node --import tsx ./esm.cts', '`./esm.cts` uses import or export'],
      ['tsx watch dirname.ts', '`dirname.ts` mixes import or export with CommonJS globals'],
      [
        'tsx --test plain.ts dirname.ts',
        '`dirname.ts` mixes import or export with CommonJS globals',
      ],
      [
        'concurrently "tsx dirname.ts"',
        '`dirname.ts` mixes import or export with CommonJS globals',
      ],
      ['tsx exports.ts', '`exports.ts` mixes import or export with CommonJS globals'],
      ['tsx compact.ts', '`compact.ts` mixes import or export with CommonJS globals'],
      ['tsx commented.ts', '`commented.ts` mixes import or export with CommonJS globals'],
      ['tsx bracket.ts', '`bracket.ts` mixes import or export with CommonJS globals'],
      ['tsx string-url.ts', '`string-url.ts` mixes import or export with CommonJS globals'],
      ['tsx export-assign.ts', '`export-assign.ts` mixes import or export with CommonJS globals'],
      ['tsx banner.ts', '`banner.ts` mixes import or export with CommonJS globals'],
      ['tsx cli-guard.ts', '`cli-guard.ts` mixes import or export with CommonJS globals'],
      [
        'tsx module-filename.ts',
        '`module-filename.ts` mixes import or export with CommonJS globals',
      ],
      ['tsx template-require.cts', 'requires `./helper.js`, which only tsx resolves'],
      [
        'node --require tsx/cjs --require ./require-js.cts ./builtin.cts',
        '`./require-js.cts` requires `./helper.js`',
      ],
      ['generate && tsx generated.ts', '`generated.ts` does not exist yet'],
      ['tsx inner.ts', '`inner.ts` mixes import or export with CommonJS globals'],
      ['tsx require-js.cts', 'requires `./helper.js`, which only tsx resolves to `./helper.ts`'],
    ])('keeps %s', (command, reason) => {
      expect(rewriteTsxCommand(command, dir)).toEqual({
        command,
        kept: [expect.stringContaining(reason)],
      });
    });

    it('checks a preload given as an absolute path', () => {
      // A root-level path is absolute on Windows too, unlike a temporary directory with
      // an 8.3 name such as `RUNNER~1`, which reads as shell syntax.
      const command = 'tsx --require /vp-missing-preload/setup.cts ./builtin.cts';
      expect(rewriteTsxCommand(command, dir)).toEqual({
        command,
        kept: [expect.stringContaining('`/vp-missing-preload/setup.cts` does not exist yet')],
      });
    });

    it.each([
      ['tsx cjs.cts', 'vpx cjs.cts'],
      ['tsx plain.ts', 'vpx plain.ts'],
      ['tsx esm-idioms.ts', 'vpx esm-idioms.ts'],
      ['tsx comment-only.ts', 'vpx comment-only.ts'],
      ['tsx require-bare.cts', 'vpx require-bare.cts'],
      ['tsx builtin.cts', 'vpx builtin.cts'],
      ['tsx node-module.ts', 'vpx node-module.ts'],
      ['tsx import-equals.cts', 'vpx import-equals.cts'],
      ['tsx importer.ts', 'vpx importer.ts'],
      ['tsx --test plain.ts', 'vpx --test plain.ts'],
      // A glob, and a script after a shell state change, are not checked.
      ['tsx --test tests/*.test.ts', 'vpx --test tests/*.test.ts'],
      ['cd tools && tsx only.ts', 'cd tools && vpx only.ts'],
    ])('rewrites %s', (command, expected) => {
      expect(rewriteTsxCommand(command, dir)).toEqual({ command: expected, kept: [] });
    });

    it('rewrites a script with __dirname in an ES module package', () => {
      fs.writeFileSync(path.join(dir, 'package.json'), '{ "type": "module" }\n');
      // `"type": "module"` already ran it as an ES module under tsx.
      expect(rewriteTsxCommand('tsx dirname.ts', dir).command).toBe('vpx dirname.ts');
    });

    it('checks a script under the home directory', () => {
      vi.spyOn(os, 'homedir').mockReturnValue(dir);
      try {
        expect(rewriteTsxCommand('tsx ~/dirname.ts', path.join(dir, 'tools')).kept).toEqual([
          expect.stringContaining('mixes import or export with CommonJS globals'),
        ]);
      } finally {
        vi.restoreAllMocks();
      }
    });
  });
});

describe('migrateTsxToVpx', () => {
  let root: string;

  const write = (file: string, content: string | object) => {
    const target = path.join(root, file);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(
      target,
      typeof content === 'string' ? content : `${JSON.stringify(content, null, 2)}\n`,
    );
  };
  const read = (file: string) => fs.readFileSync(path.join(root, file), 'utf8');
  const readJson = (file: string) => JSON.parse(read(file));

  beforeEach(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-tsx-'));
    vi.spyOn(process, 'cwd').mockReturnValue(root);
    // The scripts the commands below run; migration reads them before rewriting.
    for (const file of [
      'src/server.ts',
      'scripts/seed.ts',
      'scripts/check.ts',
      'scripts/docs.ts',
      'packages/a/gen.ts',
    ]) {
      write(file, 'export {};\n');
    }
  });

  afterEach(() => {
    vi.restoreAllMocks();
    fs.rmSync(root, { recursive: true, force: true });
  });

  it('rewrites scripts, lint-staged tasks, and shebangs, then removes tsx', () => {
    write('package.json', {
      type: 'module',
      scripts: { dev: 'tsx watch src/server.ts', seed: 'npx tsx scripts/seed.ts' },
      'lint-staged': { '*.ts': ['tsx scripts/check.ts'] },
      // A file extension, not a use of tsx.
      jest: { moduleFileExtensions: ['ts', 'tsx'] },
      devDependencies: { tsx: '^4.20.0', vite: '^7.0.0' },
    });
    write('.lintstagedrc.json', { '*.md': 'tsx scripts/docs.ts' });
    write('scripts/release.ts', '#!/usr/bin/env tsx\nconsole.log("release");\n');
    write('src/server.ts', 'export {};\n');
    // File extensions are not uses of tsx.
    write('vite.config.ts', "export default { resolve: { extensions: ['ts', 'tsx'] } };\n");
    write('src/App.vue', '<script lang="tsx">\nexport default {};\n</script>\n');
    write('.eslintrc.json', { settings: { 'import/resolver': { node: { extensions: ['tsx'] } } } });
    // Caches and reports are not scanned.
    write('.next/cache/config.json', { command: 'tsx scripts/old.ts' });
    write('coverage/report.json', { command: 'tsx scripts/old.ts' });
    const report = createMigrationReport();

    expect(detectTsxProject(root)).toBe(true);
    expect(migrateTsxToVpx(root, undefined, report)).toEqual({
      changed: true,
      dependencyRemoved: true,
    });

    expect(readJson('package.json')).toEqual({
      type: 'module',
      scripts: { dev: 'vpx --watch src/server.ts', seed: 'vpx scripts/seed.ts' },
      'lint-staged': { '*.ts': ['vpx scripts/check.ts'] },
      jest: { moduleFileExtensions: ['ts', 'tsx'] },
      devDependencies: { vite: '^7.0.0' },
    });
    expect(readJson('.lintstagedrc.json')).toEqual({ '*.md': 'vpx scripts/docs.ts' });
    expect(read('scripts/release.ts')).toBe('#!/usr/bin/env vpx\nconsole.log("release");\n');
    expect(report.tsxMigrated).toBe(true);
    expect(report.warnings).toEqual([]);

    // A second run has nothing left to do.
    expect(detectTsxProject(root)).toBe(false);
    expect(migrateTsxToVpx(root, undefined, createMigrationReport())).toEqual({
      changed: false,
      dependencyRemoved: false,
    });
  });

  it.each([
    ['a kept script', 'package.json', undefined, '`scripts` in package.json still uses tsx'],
    [
      'an import of the tsx API',
      'src/load.ts',
      "import { tsImport } from 'tsx/esm/api';\n",
      'src/load.ts still uses tsx',
    ],
    [
      'a template-literal import of the tsx API',
      'src/lazy.ts',
      'const { tsImport } = await import(`tsx/esm/api`);\n',
      'src/lazy.ts still uses tsx',
    ],
    [
      'an annotated dynamic import of the tsx API',
      'src/ignored.ts',
      "const api = await import(/* @vite-ignore */ 'tsx/esm/api');\n",
      'src/ignored.ts still uses tsx',
    ],
    [
      'a resolved tsx subpath',
      'src/register.ts',
      "const loader = import.meta.resolve('tsx/esm');\n",
      'src/register.ts still uses tsx',
    ],
    [
      'a Mocha YAML config',
      '.mocharc.yml',
      'node-option:\n  - import=tsx\n',
      '.mocharc.yml still uses tsx',
    ],
    [
      'a Mocha JavaScript config',
      '.mocharc.cjs',
      "module.exports = { 'node-option': ['import=tsx'] };\n",
      '.mocharc.cjs still uses tsx',
    ],
    [
      'a CI workflow',
      '.github/workflows/ci.yml',
      'jobs:\n  run: npx tsx scripts/ci.ts\n',
      '.github/workflows/ci.yml still uses tsx',
    ],
    [
      'a git hook',
      '.husky/pre-commit',
      'npx tsx scripts/check.ts\n',
      '.husky/pre-commit still uses tsx',
    ],
    [
      'a bash script',
      'tools/release.bash',
      'tsx scripts/release.ts\n',
      'tools/release.bash still uses tsx',
    ],
    [
      'an extensionless shebang file',
      'bin/release',
      '#!/usr/bin/env tsx\nexport {};\n',
      'bin/release still uses tsx',
    ],
    [
      'an extensionless JavaScript file',
      'bin/cli',
      "import { tsImport } from 'tsx/esm/api';\n",
      'bin/cli still uses tsx',
    ],
    [
      'a nested package',
      'examples/demo/package.json',
      '{ "scripts": { "start": "tsx index.ts" } }\n',
      '`scripts` in examples/demo/package.json still uses tsx',
    ],
    [
      'build output',
      'dist/launcher.js',
      "import { register } from 'tsx/esm/api';\n",
      'dist/launcher.js still uses tsx',
    ],
    [
      'a process call',
      'src/run.ts',
      "import { spawn } from 'node:child_process';\nspawn('tsx', ['job.ts']);\n",
      'src/run.ts still uses tsx',
    ],
    [
      'a process call without arguments',
      'src/repl.ts',
      "import { spawn } from 'node:child_process';\nspawn('tsx');\n",
      'src/repl.ts still uses tsx',
    ],
    [
      'a process call with an argv array',
      'src/job.ts',
      "Bun.spawn(['tsx', 'job.ts']);\n",
      'src/job.ts still uses tsx',
    ],
    [
      'a TOML config',
      'netlify.toml',
      '[build]\ncommand = "tsx scripts/build.ts"\n',
      'netlify.toml still uses tsx',
    ],
    [
      'a shell pipe into tsx',
      'scripts/run.sh',
      'cat job.ts | tsx\n',
      'scripts/run.sh still uses tsx',
    ],
    [
      'a quoted tsx in a shell script',
      'scripts/job.sh',
      '"tsx" scripts/job.ts\n',
      'scripts/job.sh still uses tsx',
    ],
    [
      'a Windows batch file',
      'scripts/run.cmd',
      '@node_modules\\.bin\\tsx.cmd scripts\\a.ts\r\n',
      'scripts/run.cmd still uses tsx',
    ],
    [
      'a JSON tool config',
      '.mocharc.json',
      '{ "require": "tsx/cjs", "spec": "test/**/*.test.ts" }\n',
      '.mocharc.json still uses tsx',
    ],
    [
      'a bare loader in a JSON tool config',
      '.mocharc.json',
      '{ "import": "tsx" }\n',
      '.mocharc.json still uses tsx',
    ],
    [
      'a bare loader list in a JSON tool config',
      '.mocharc.json',
      '{ "require": ["tsx"] }\n',
      '.mocharc.json still uses tsx',
    ],
    [
      'a bare lint-staged task',
      '.lintstagedrc.json',
      '{ "*.ts": "tsx" }\n',
      '.lintstagedrc.json still uses tsx',
    ],
    [
      'a bare lint-staged task in an extensionless config',
      '.lintstagedrc',
      '{"*.ts":"tsx"}\n',
      '.lintstagedrc still uses tsx',
    ],
    [
      'a JSONC tool config',
      'wrangler.jsonc',
      '{\n  // Build step\n  "build": { "command": "tsx scripts/build.ts" },\n}\n',
      'wrangler.jsonc still uses tsx',
    ],
    [
      'a JavaScript tool config',
      'ava.config.js',
      "export default { nodeArguments: ['--import=tsx'] };\n",
      'ava.config.js still uses tsx',
    ],
    [
      'a JavaScript tool config with split loader arguments',
      'ava.config.mjs',
      "export default { nodeArguments: ['--import', 'tsx'] };\n",
      'ava.config.mjs still uses tsx',
    ],
    [
      'a framework component',
      'src/pages/index.astro',
      "---\nimport { tsImport } from 'tsx/esm/api';\n---\n",
      'src/pages/index.astro still uses tsx',
    ],
  ])('keeps the tsx dependency used by %s', (_, file, content, user) => {
    write('package.json', {
      scripts: {
        dev: 'tsx watch src/server.ts',
        ...(content === undefined ? { repl: 'tsx' } : {}),
      },
      devDependencies: { tsx: '^4.20.0' },
    });
    if (content !== undefined) {
      write(file, content);
    }
    const report = createMigrationReport();

    expect(migrateTsxToVpx(root, undefined, report).dependencyRemoved).toBe(false);
    expect(readJson('package.json').scripts.dev).toBe('vpx --watch src/server.ts');
    expect(readJson('package.json').devDependencies).toEqual({ tsx: '^4.20.0' });
    expect(report.warnings).toContain(`Kept the tsx devDependency in package.json: ${user}.`);
  });

  it.each(['dependencies', 'optionalDependencies', 'peerDependencies'])(
    'keeps tsx in a package that lists it in %s',
    (field) => {
      write('package.json', {
        scripts: { start: 'tsx src/server.ts' },
        [field]: { tsx: '^4.20.0' },
      });
      const report = createMigrationReport();

      expect(detectTsxProject(root)).toBe(false);
      expect(migrateTsxToVpx(root, undefined, report).changed).toBe(false);
      expect(readJson('package.json').scripts.start).toBe('tsx src/server.ts');
      expect(report.warnings).toEqual([expect.stringContaining('Kept tsx in package.json')]);
    },
  );

  it('keeps the tsx dependency of a workspace package used by its nested example', () => {
    write('package.json', { private: true });
    write('packages/a/package.json', {
      scripts: { gen: 'tsx gen.ts' },
      devDependencies: { tsx: '^4.20.0' },
    });
    write('packages/a/examples/demo/main.ts', "import { tsImport } from 'tsx/esm/api';\n");
    write('packages/a/examples/demo/package.json', { private: true });
    const report = createMigrationReport();

    migrateTsxToVpx(root, [{ name: 'a', path: 'packages/a' }], report);

    expect(readJson('packages/a/package.json')).toEqual({
      scripts: { gen: 'vpx gen.ts' },
      devDependencies: { tsx: '^4.20.0' },
    });
    expect(report.warnings).toEqual([
      'Kept the tsx devDependency in packages/a/package.json: packages/a/examples/demo/main.ts still uses tsx.',
    ]);
  });

  it.each([
    ['imports', { '#loader': { node: 'tsx/esm/api' } }],
    ['imports', { '#loader': 'tsx' }],
    ['ava', { extensions: { ts: 'module' }, nodeArguments: ['--import=tsx'] }],
  ])('keeps the tsx dependency used by the %s field of package.json', (field, value) => {
    write('package.json', {
      scripts: { dev: 'tsx watch src/server.ts' },
      [field]: value,
      devDependencies: { tsx: '^4.20.0' },
    });
    const report = createMigrationReport();

    expect(migrateTsxToVpx(root, undefined, report).dependencyRemoved).toBe(false);
    expect(readJson('package.json').scripts.dev).toBe('vpx --watch src/server.ts');
    expect(report.warnings).toEqual([
      `Kept the tsx devDependency in package.json: \`${field}\` in package.json still uses tsx.`,
    ]);
  });

  it('detects a project that runs tsx only through shebangs', () => {
    write('package.json', {
      type: 'module',
      scripts: { release: './scripts/release.ts' },
      devDependencies: { tsx: '^4.20.0' },
    });
    write('scripts/release.ts', '#!/usr/bin/env tsx\nexport {};\n');

    expect(detectTsxProject(root)).toBe(true);
    expect(migrateTsxToVpx(root)).toEqual({ changed: true, dependencyRemoved: true });
    expect(read('scripts/release.ts')).toBe('#!/usr/bin/env vpx\nexport {};\n');
    expect(detectTsxProject(root)).toBe(false);
  });

  it('detects tsx tasks in a lint-staged JSON file', () => {
    write('package.json', { scripts: { prepare: 'husky' } });
    write('.lintstagedrc.json', { '*.ts': 'tsx scripts/check.ts' });

    expect(detectTsxProject(root)).toBe(true);
    migrateTsxToVpx(root);
    expect(readJson('.lintstagedrc.json')).toEqual({ '*.ts': 'vpx scripts/check.ts' });
    expect(detectTsxProject(root)).toBe(false);
  });

  it('rewrites shebang options as it rewrites commands', () => {
    write('package.json', { type: 'module' });
    write('scripts/a.ts', '#!/usr/bin/env -S tsx --no-cache --tsconfig cli.json\nexport {};\n');
    write('scripts/b.ts', '#!/usr/bin/env -S tsx -e x\nexport {};\n');
    const report = createMigrationReport();

    migrateTsxToVpx(root, undefined, report);

    expect(read('scripts/a.ts')).toBe('#!/usr/bin/env -S vpx --tsconfig cli.json\nexport {};\n');
    expect(read('scripts/b.ts')).toBe('#!/usr/bin/env -S tsx -e x\nexport {};\n');
    expect(report.warnings).toEqual([
      expect.stringContaining('Kept tsx in the shebang of scripts/b.ts: `vpx` does not run'),
    ]);
  });

  it('keeps tsx for a script that would lose CommonJS globals under vpx', () => {
    write('package.json', { scripts: { build: 'tsx scripts/build.ts' } });
    write('scripts/build.ts', "import fs from 'node:fs';\nfs.readdirSync(__dirname);\n");
    write(
      'scripts/release.ts',
      '#!/usr/bin/env tsx\nimport fs from "node:fs";\nrequire("node:path");\n',
    );
    const report = createMigrationReport();

    expect(detectTsxProject(root)).toBe(false);
    migrateTsxToVpx(root, undefined, report);

    expect(readJson('package.json').scripts.build).toBe('tsx scripts/build.ts');
    expect(read('scripts/release.ts')).toMatch(/^#!\/usr\/bin\/env tsx\n/);
    expect(report.warnings).toEqual([
      expect.stringContaining(
        'Kept tsx in package.json scripts "build": `scripts/build.ts` mixes import or export with CommonJS globals',
      ),
      expect.stringContaining(
        'Kept tsx in the shebang of scripts/release.ts: `./release.ts` mixes import or export with CommonJS globals',
      ),
    ]);
  });

  it('keeps the root tsx while a workspace package still uses it', () => {
    write('package.json', { private: true, devDependencies: { tsx: '^4.20.0' } });
    write('packages/a/package.json', { scripts: { gen: 'tsx gen.ts' } });
    write('packages/b/package.json', { scripts: { calc: "tsx -e 'run()'" } });
    const packages = [
      { name: 'a', path: 'packages/a' },
      { name: 'b', path: 'packages/b' },
    ];
    const report = createMigrationReport();

    migrateTsxToVpx(root, packages, report);

    expect(readJson('packages/a/package.json').scripts.gen).toBe('vpx gen.ts');
    expect(readJson('packages/b/package.json').scripts.calc).toBe("tsx -e 'run()'");
    expect(readJson('package.json').devDependencies).toEqual({ tsx: '^4.20.0' });
    expect(report.warnings).toEqual([
      expect.stringContaining('Kept tsx in packages/b/package.json scripts "calc"'),
      'Kept the tsx devDependency in package.json: `scripts` in packages/b/package.json still uses tsx.',
    ]);
  });
});
