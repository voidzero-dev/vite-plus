import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@voidzero-dev/vite-plus-prompts', () => ({
  log: { info: vi.fn(), success: vi.fn(), warn: vi.fn() },
}));

import {
  detectOxnodeProject,
  migrateOxnodeToVpx,
  rewriteOxnodeCommand,
} from '../migrator/oxnode.ts';
import { createMigrationReport } from '../report.ts';

describe('rewriteOxnodeCommand', () => {
  it.each([
    ['oxnode ./scripts/build.ts --flag', 'vpx ./scripts/build.ts --flag'],
    ['oxnode -C dev ./build.ts', 'vpx -C dev ./build.ts'],
    ['oxnode --conditions=dev --inspect ./build.mts', 'vpx --conditions=dev --inspect ./build.mts'],
    ['oxnode watch src/server.ts', 'vpx --watch src/server.ts'],
    [
      'oxnode watch --include "config/*.json" --exclude src/gen.ts --tsconfig cli.json --clear-screen=false src/server.ts --port 3000',
      'vpx --watch --include "config/*.json" --exclude src/gen.ts --tsconfig cli.json --clear-screen=false src/server.ts --port 3000',
    ],
    ['oxnode --test ./test.test.ts', 'vpx --test ./test.test.ts'],
    ['npx oxnode a.ts', 'vpx a.ts'],
    ['npx --yes oxnode a.ts', 'vpx a.ts'],
    ['pnpm exec oxnode a.ts', 'vpx a.ts'],
    ['pnpm dlx oxnode a.ts', 'vpx a.ts'],
    ['npm exec -- oxnode a.ts', 'vpx a.ts'],
    ['yarn oxnode a.ts', 'vpx a.ts'],
    ['bunx oxnode a.ts', 'vpx a.ts'],
    ['NODE_ENV=dev oxnode a.ts', 'NODE_ENV=dev vpx a.ts'],
    [
      'cross-env OXC_TSCONFIG_PATH=cli.json oxnode a.ts',
      'cross-env OXC_TSCONFIG_PATH=cli.json vpx a.ts',
    ],
    ['OXC_TRANSFORM_ALL=typescript oxnode a.ts', 'OXC_TRANSFORM_ALL=typescript vpx a.ts'],
    ['node --import @oxc-node/core/register a.ts', 'node --import vite-plus/register a.ts'],
    [
      'node --no-warnings --experimental-specifier-resolution=node --import @oxc-node/core/register --loader ./scripts/variant-loader.js --loader ./scripts/image-loader.js a.ts',
      'node --no-warnings --experimental-specifier-resolution=node --import vite-plus/register --loader ./scripts/variant-loader.js --loader ./scripts/image-loader.js a.ts',
    ],
    [
      'node --loader=./variant-loader.js --import=@oxc-node/core/register.mjs --experimental-loader ./image-loader.js a.ts',
      'node --loader=./variant-loader.js --import=vite-plus/register --experimental-loader ./image-loader.js a.ts',
    ],
    [
      'NODE_OPTIONS="--import @oxc-node/core/register --loader ./image-loader.js" node a.ts',
      'NODE_OPTIONS="--import vite-plus/register --loader ./image-loader.js" node a.ts',
    ],
    [
      'NODE_OPTIONS=--import=@oxc-node/core/register node --loader ./image-loader.js a.ts',
      'NODE_OPTIONS=--import=vite-plus/register node --loader ./image-loader.js a.ts',
    ],
    [
      'node --import @oxc-node/core/register --watch a.ts',
      'node --import vite-plus/register --watch a.ts',
    ],
    [
      'node --import @oxc-node/core/register --watch-path src a.ts',
      'node --import vite-plus/register --watch-path src a.ts',
    ],
    ['node --import @oxc-node/core/register --test', 'node --import vite-plus/register --test'],
    ['node --import @oxc-node/core/register -e 1', 'node --import vite-plus/register -e 1'],
    ['node --import @oxc-node/core/register', 'node --import vite-plus/register'],
    [
      'node --import @oxc-node/core/register src/index',
      'node --import vite-plus/register src/index',
    ],
    [
      'pnpm exec node --import=@oxc-node/core/register.mjs a.ts',
      'pnpm exec node --import=vite-plus/register a.ts',
    ],
    [
      'NODE_OPTIONS=--import=@oxc-node/core/register node a.ts',
      'NODE_OPTIONS=--import=vite-plus/register node a.ts',
    ],
    [
      "NODE_OPTIONS='--import @oxc-node/core/register.mjs' mocha",
      "NODE_OPTIONS='--import vite-plus/register' mocha",
    ],
    [
      'NODE_OPTIONS=--import=@oxc-node/core/register concurrently "node a.ts" "node --test"',
      'NODE_OPTIONS=--import=vite-plus/register concurrently "node a.ts" "node --test"',
    ],
    [
      'OXC_TSCONFIG_PATH=cli.json node --import @oxc-node/core/register a.ts',
      'OXC_TSCONFIG_PATH=cli.json node --import vite-plus/register a.ts',
    ],

    [
      'node --import=@oxc-node/core/register.mjs -C dev a.ts',
      'node --import=vite-plus/register -C dev a.ts',
    ],
    [
      'node --import vite-plus/register --import @oxc-node/core/register a.ts',
      'node --import vite-plus/register --import vite-plus/register a.ts',
    ],
    ['oxnode --import @oxc-node/core/register a.ts', 'vpx a.ts'],
    ['oxnode a.ts && oxnode watch b.ts', 'vpx a.ts && vpx --watch b.ts'],
    ["node -e 'console.log(1)' && oxnode a.ts", "node -e 'console.log(1)' && vpx a.ts"],
    ['concurrently "oxnode watch api.ts" "vp dev"', 'concurrently "vpx --watch api.ts" "vp dev"'],
    ["nodemon --exec 'oxnode a.ts'", "nodemon --exec 'vpx a.ts'"],
    ["sh -c 'oxnode a.ts'", "sh -c 'vpx a.ts'"],
    ['cross-env-shell "oxnode a.ts"', 'cross-env-shell "vpx a.ts"'],
  ])('rewrites %s', (command, expected) => {
    expect(rewriteOxnodeCommand(command)).toEqual({ command: expected, kept: [] });
    expect(rewriteOxnodeCommand(expected)).toEqual({ command: expected, kept: [] });
  });

  it.each([
    ['oxnode --watch a.ts', 'Node.js watcher'],
    ['node -r ./setup.cjs --import @oxc-node/core/register a.ts', 'additional startup preload'],
    [
      'node --require=dotenv/config --import @oxc-node/core/register a.ts',
      'additional startup preload',
    ],
    [
      'node --import ./setup.mjs --import @oxc-node/core/register a.ts',
      'additional startup preload',
    ],
    [
      'node --import @oxc-node/core/register --import ./setup.ts a.ts',
      'additional startup preload',
    ],
    [
      'NODE_OPTIONS="-r ./setup.cjs --import @oxc-node/core/register" node a.ts',
      'additional startup preload',
    ],
    [
      'NODE_OPTIONS="--require=dotenv/config --import @oxc-node/core/register" mocha',
      'additional startup preload',
    ],

    ['oxnode --watch_path=src a.ts', 'Node.js watcher'],
    ['oxnode --node-help a.ts', '--node-help'],
    ['oxnode --env-file=.env a.ts', 'env file can configure'],
    ['node --import @oxc-node/core/register --env-file=.env a.ts', 'env file can configure'],
    ['node --import @oxc-node/core/register --env_file=.env a.ts', 'env file can configure'],
    [
      'node --import @oxc-node/core/register --experimental_config_file=node.json a.ts',
      'config file can load additional hooks',
    ],
    [
      'node --import @oxc-node/core/register --config-file=node.json a.ts',
      'config file can load additional hooks',
    ],
    [
      'node --import @oxc-node/core/register --experimental-default-config-file a.ts',
      'config file can load additional hooks',
    ],
    ['OXC_TRANSFORM_ALL=1 node --import @oxc-node/core/register a.ts', 'dependency transforms'],
    [
      'OXC_TRANSFORM_ALL=0 NODE_OPTIONS=--import=@oxc-node/core/register node a.ts',
      'dependency transforms',
    ],
    [
      'TSX_TSCONFIG_PATH=cli.json node --import @oxc-node/core/register a.ts',
      'different configuration',
    ],
    [
      'VP_SCRIPT_TSCONFIG=cli.json NODE_OPTIONS=--import=@oxc-node/core/register mocha',
      'different configuration',
    ],
    ['TSX_TSCONFIG_PATH=cli.json oxnode a.ts', 'different configuration'],
    [
      'NODE_OPTIONS=--import=@oxc-node/core/register concurrently "node --import @oxc-node/core/register a.ts" "oxnode b.ts"',
      'hooks that would compete',
    ],

    ['oxnode --env-file-if-exists .env a.ts', 'env file can configure'],
    ['oxnode --help a.ts', 'different help'],
    ['oxnode --version a.ts', 'different versions'],
    ["oxnode -e 'console.log(1)'", '--eval or --print'],
    ['oxnode -c a.ts', 'shell mode'],
    ['oxnode', 'REPL'],
    ['oxnode --test', 'test files explicitly'],
    ['oxnode watch --test a.test.ts', 'does not run `--test`'],
    ['oxnode src/index', 'file extension'],
    ['oxnode a.ts | tee output', 'shell syntax'],
    ['oxnode $ENTRY', 'shell syntax'],
    ['bunx --bun oxnode a.ts', 'does not rewrite'],
    ['npx oxnode@1 a.ts', 'does not rewrite'],
    ['node --loader @oxc-node/core/esm a.ts', 'does not rewrite --loader'],
    ['node --import @oxc-node/core/esm.mjs a.ts', 'does not rewrite --import'],
    ['oxnode --require @oxc-node/core/register a.ts', 'does not rewrite --require'],
    ['OXC_TRANSFORM_ALL=1 oxnode a.ts', 'dependency transforms'],
    ['OXC_TRANSFORM_ALL=0 oxnode a.ts', 'dependency transforms'],
    ['OXC_TRANSFORM_ALL=false oxnode a.ts', 'dependency transforms'],
    ['OXC_TRANSFORM_ALL= oxnode a.ts', 'dependency transforms'],
    ['OXC_TRANSFORM_ALL=TypeScript oxnode a.ts', 'dependency transforms'],
    ["OXC_TRANSFORM_ALL=' typescript ' oxnode a.ts", 'dependency transforms'],
    ['OXC_TRANSFORM_ALL=1 concurrently "oxnode a.ts"', 'dependency transforms'],
    [
      'cross-env NODE_OPTIONS=--import=@oxc-node/core/register oxnode a.ts',
      'hooks that would compete',
    ],
  ])('retains %s', (command, reason) => {
    expect(rewriteOxnodeCommand(command)).toEqual({
      command,
      kept: expect.arrayContaining([expect.stringContaining(reason)]),
    });
  });

  it.each(['vpx a.ts', 'node a.js', 'tsx a.ts', 'oxnode-tools a.ts'])(
    'ignores unrelated command %s',
    (command) => {
      expect(rewriteOxnodeCommand(command)).toEqual({ command, kept: [] });
    },
  );
});

describe('oxnode inline code', () => {
  it.each(['--eval', '--print', '-e', '-p', '-pe'])(
    'retains %s code that configures the standalone loader',
    (flag) => {
      const code = "require('@oxc-node/core').setTransformDeps('all')";
      for (const command of [
        `node --import @oxc-node/core/register ${flag} "${code}"`,
        `NODE_OPTIONS=--import=@oxc-node/core/register node ${flag} "${code}"`,
        `node --import @oxc-node/core/register ${flag}="${code}"`,
        ...(flag.startsWith('--')
          ? []
          : [`node --import @oxc-node/core/register ${flag}"${code}"`]),
      ]) {
        expect(rewriteOxnodeCommand(command)).toEqual({
          command,
          kept: [expect.stringContaining('inline code uses a standalone oxc-node API')],
        });
      }
    },
  );
});

describe('oxnode migration', () => {
  let root: string;
  const write = (file: string, content: string | object) => {
    const target = path.join(root, file);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, typeof content === 'string' ? content : JSON.stringify(content));
  };
  const read = (file: string) => fs.readFileSync(path.join(root, file), 'utf8');
  const readJson = (file: string) => JSON.parse(read(file));

  beforeEach(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-oxnode-'));
    vi.spyOn(process, 'cwd').mockReturnValue(root);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    fs.rmSync(root, { recursive: true, force: true });
  });

  it('rewrites scripts, JSON staged tasks and shebangs, and removes unused packages', () => {
    write('package.json', {
      scripts: { build: 'oxnode -C dev ./build.ts', dev: 'oxnode watch src/server.ts' },
      'lint-staged': { '*.ts': ['oxnode scripts/check.ts'] },
      devDependencies: {
        '@oxc-node/cli': '^1.0.0',
        '@oxc-node/core': '^1.0.0',
        typescript: '^6.0.0',
      },
    });
    write('.lintstagedrc.json', { '*.md': 'oxnode scripts/docs.ts' });
    write('scripts/release.ts', '#!/usr/bin/env oxnode\nexport {};\n');
    write('scripts/dev.mts', '#!/usr/bin/env -S oxnode -C dev\nexport {};\n');
    const report = createMigrationReport();

    expect(detectOxnodeProject(root)).toBe(true);
    expect(migrateOxnodeToVpx(root, undefined, report)).toEqual({
      changed: true,
      dependencyRemoved: true,
    });
    expect(readJson('package.json')).toEqual({
      scripts: { build: 'vpx -C dev ./build.ts', dev: 'vpx --watch src/server.ts' },
      'lint-staged': { '*.ts': ['vpx scripts/check.ts'] },
      devDependencies: { typescript: '^6.0.0' },
    });
    expect(readJson('.lintstagedrc.json')).toEqual({ '*.md': 'vpx scripts/docs.ts' });
    expect(read('scripts/release.ts')).toBe('#!/usr/bin/env vpx\nexport {};\n');
    expect(read('scripts/dev.mts')).toBe('#!/usr/bin/env -S vpx -C dev\nexport {};\n');
    expect(report.oxnodeMigrated).toBe(true);
    expect(report.warnings).toEqual([]);
    expect(detectOxnodeProject(root)).toBe(false);
    expect(migrateOxnodeToVpx(root)).toEqual({ changed: false, dependencyRemoved: false });
  });

  it.each([
    ['src/api.ts', "import { OxcTransformer } from '@oxc-node/core';\n"],
    ['src/importer.ts', "import { createImporter } from '@oxc-node/core/importer';\n"],
    ['dist/helpers.js', "import helper from '@oxc-node/core/helpers/typeof';\n"],
    ['src/watch.ts', "import { runWatch } from '@oxc-node/cli/watch';\n"],
    ['src/spawn.ts', "spawn('oxnode', ['a.ts']);\n"],
    ['src/windows.ts', "spawn('OXNODE.CMD', ['a.ts']);\n"],
    ['src/path.ts', "spawn('./node_modules/.bin/oxnode', ['a.ts']);\n"],
    ['.github/workflows/ci.yml', 'jobs:\n  run: npx oxnode scripts/ci.ts\n'],
    ['.husky/pre-commit', 'oxnode scripts/check.ts\n'],
    ['scripts/release.sh', 'oxnode scripts/release.ts\n'],
    ['.mocharc.json', '{"import":"@oxc-node/core/register"}'],
    ['.env.local', 'NODE_OPTIONS=--import=@oxc-node/core/register\n'],
    ['.env.config.json', 'NODE_OPTIONS=--import=@oxc-node/core/register\n'],
    [
      'ava.config.js',
      "export default { nodeArguments: ['--import', '@oxc-node/core/register'] };\n",
    ],
    ['lint-staged.config.ts', "export default { '*.ts': 'oxnode check.ts' };\n"],
    ['bin/release', '#!/usr/bin/env oxnode\nexport {};\n'],
  ])('retains dependencies used by %s', (file, content) => {
    write('package.json', {
      scripts: { build: 'oxnode ./build.ts' },
      devDependencies: { '@oxc-node/cli': '^1.0.0', '@oxc-node/core': '^1.0.0' },
    });
    write(file, content);
    const report = createMigrationReport();

    expect(migrateOxnodeToVpx(root, undefined, report).dependencyRemoved).toBe(false);
    expect(readJson('package.json').scripts.build).toBe('vpx ./build.ts');
    expect(readJson('package.json').devDependencies).toEqual({
      '@oxc-node/cli': '^1.0.0',
      '@oxc-node/core': '^1.0.0',
    });
    expect(report.warnings).toContainEqual(expect.stringContaining(`${file} still uses oxnode`));
  });

  it.each([
    ['scripts/entry.ts', '#!/usr/bin/env node --import @oxc-node/core/register'],
    ['scripts/entry.mts', '#!/usr/bin/env -S node --import=@oxc-node/core/register.mjs'],
    [
      'scripts/entry.ts',
      '#!/usr/bin/env node --no-warnings --experimental-specifier-resolution=node --import @oxc-node/core/register --loader ./scripts/variant-loader.js --loader ./scripts/image-loader.js',
    ],
    [
      'bin/release',
      '#!/usr/bin/env -S node --loader=./scripts/variant-loader.js --import @oxc-node/core/register --experimental-loader ./scripts/image-loader.js',
    ],
  ])('rewrites the Node.js preload shebang in %s: %s', (file, shebang) => {
    write('package.json', { devDependencies: { '@oxc-node/core': '^1.0.0' } });
    const body = '\nconst value: string = "hello";\nconsole.log(value);\n';
    write(file, shebang + body);
    const report = createMigrationReport();

    expect(detectOxnodeProject(root)).toBe(true);
    expect(migrateOxnodeToVpx(root, undefined, report)).toEqual({
      changed: true,
      dependencyRemoved: true,
    });
    expect(read(file)).toBe(
      shebang.replace(/@oxc-node\/core\/register(?:\.mjs)?/, 'vite-plus/register') + body,
    );
    expect(readJson('package.json').devDependencies).toEqual({});
    expect(report.oxnodeMigrated).toBe(true);
    expect(report.warnings).toEqual([]);
    expect(detectOxnodeProject(root)).toBe(false);
    expect(migrateOxnodeToVpx(root)).toEqual({ changed: false, dependencyRemoved: false });
  });

  it('preserves shebang whitespace, CRLF, and the executable file mode', () => {
    write('package.json', { devDependencies: { '@oxc-node/core': '^1.0.0' } });
    const source =
      '#!/usr/bin/env -S node\t--import "@oxc-node/core/register"  \r\nconsole.log(1);\r\n';
    write('bin/release', source);
    const file = path.join(root, 'bin/release');
    fs.chmodSync(file, 0o755);
    const mode = fs.statSync(file).mode;

    expect(migrateOxnodeToVpx(root)).toEqual({ changed: true, dependencyRemoved: true });
    expect(read('bin/release')).toBe(
      source.replace('@oxc-node/core/register', 'vite-plus/register'),
    );
    expect(fs.statSync(file).mode).toBe(mode);
  });

  it('ignores a Node.js shebang without the oxnode register preload', () => {
    write('package.json', {});
    const source = '#!/usr/bin/env -S node --no-warnings\nconsole.log(1);\n';
    write('entry.ts', source);
    const report = createMigrationReport();

    expect(detectOxnodeProject(root)).toBe(false);
    expect(migrateOxnodeToVpx(root, undefined, report)).toEqual({
      changed: false,
      dependencyRemoved: false,
    });
    expect(read('entry.ts')).toBe(source);
    expect(report.warnings).toEqual([]);
  });

  it('checks the entry body without mistaking its register shebang for an API call', () => {
    write('package.json', {
      scripts: { release: 'node --import @oxc-node/core/register scripts/release.ts' },
      devDependencies: { '@oxc-node/core': '^1.0.0' },
    });
    write(
      'scripts/release.ts',
      '#!/usr/bin/env -S node --import @oxc-node/core/register\nexport {};\n',
    );

    expect(migrateOxnodeToVpx(root)).toEqual({ changed: true, dependencyRemoved: true });
    expect(readJson('package.json').scripts.release).toBe(
      'node --import vite-plus/register scripts/release.ts',
    );
    expect(read('scripts/release.ts')).toBe(
      '#!/usr/bin/env -S node --import vite-plus/register\nexport {};\n',
    );
  });

  it('retains a Node.js preload shebang when the body calls a standalone API', () => {
    write('package.json', { devDependencies: { '@oxc-node/core': '^1.0.0' } });
    const source =
      '#!/usr/bin/env -S node --import @oxc-node/core/register\n' +
      "import { setTsconfigPath } from '@oxc-node/core'; setTsconfigPath('cli.json');\n";
    write('bin/release', source);
    const report = createMigrationReport();

    expect(migrateOxnodeToVpx(root, undefined, report)).toEqual({
      changed: false,
      dependencyRemoved: false,
    });
    expect(read('bin/release')).toBe(source);
    expect(report.warnings).toContainEqual(expect.stringContaining('standalone oxc-node API'));
  });

  it.each(['--require ./setup.cjs', '--import ./setup.mjs', '--env-file .env'])(
    'retains a Node.js preload shebang with %s',
    (option) => {
      write('package.json', { devDependencies: { '@oxc-node/core': '^1.0.0' } });
      const source = `#!/usr/bin/env -S node --import @oxc-node/core/register ${option}\nexport {};\n`;
      write('entry.ts', source);

      expect(migrateOxnodeToVpx(root)).toEqual({ changed: false, dependencyRemoved: false });
      expect(read('entry.ts')).toBe(source);
    },
  );

  it('checks an absolute shebang loader even when the caller directory is unknown', () => {
    write('package.json', { devDependencies: { '@oxc-node/core': '^1.0.0' } });
    write(
      'scripts/setup.js',
      "import { setTsconfigPath } from '@oxc-node/core'; setTsconfigPath('cli.json');\n",
    );
    const loader = path.join(root, 'scripts/setup.js').replaceAll(path.sep, '/');
    const source = `#!/usr/bin/env -S node --import @oxc-node/core/register --loader "${loader}"\nexport {};\n`;
    write('entry.ts', source);
    const report = createMigrationReport();

    expect(migrateOxnodeToVpx(root, undefined, report)).toEqual({
      changed: false,
      dependencyRemoved: false,
    });
    expect(read('entry.ts')).toBe(source);
    expect(report.warnings).toContainEqual(
      expect.stringContaining('Kept oxnode in the shebang of entry.ts:'),
    );
    expect(report.warnings).toContainEqual(expect.stringContaining('standalone oxc-node API'));
  });

  it('retains a shebang when a package-relative loader calls a standalone API', () => {
    write('package.json', { devDependencies: { '@oxc-node/core': '^1.0.0' } });
    write('setup.js', "import { setTsconfigPath } from '@oxc-node/core';\n");
    const source =
      '#!/usr/bin/env -S node --import @oxc-node/core/register --loader ./setup.js\nexport {};\n';
    write('scripts/entry.ts', source);
    const report = createMigrationReport();

    expect(migrateOxnodeToVpx(root, undefined, report)).toEqual({
      changed: false,
      dependencyRemoved: false,
    });
    expect(read('scripts/entry.ts')).toBe(source);
    expect(report.warnings).toContainEqual(
      expect.stringContaining('Kept oxnode in the shebang of scripts/entry.ts: `./setup.js`'),
    );
  });

  it('migrates a shebang with a compatible package-relative loader', () => {
    write('package.json', { devDependencies: { '@oxc-node/core': '^1.0.0' } });
    write('setup.js', 'export const resolve = (s, c, next) => next(s, c);\n');
    const source =
      '#!/usr/bin/env -S node --import @oxc-node/core/register --loader ./setup.js\nexport {};\n';
    write('scripts/entry.ts', source);

    expect(migrateOxnodeToVpx(root)).toEqual({ changed: true, dependencyRemoved: true });
    expect(read('scripts/entry.ts')).toBe(
      source.replace('@oxc-node/core/register', 'vite-plus/register'),
    );
  });

  it('does not inspect a relative loader as if it were beside the shebang file', () => {
    write('package.json', { devDependencies: { '@oxc-node/core': '^1.0.0' } });
    write('setup.js', 'export const resolve = (s, c, next) => next(s, c);\n');
    write('scripts/setup.js', "import { setTsconfigPath } from '@oxc-node/core';\n");
    const source =
      '#!/usr/bin/env -S node --import @oxc-node/core/register --loader ./setup.js\nexport {};\n';
    write('scripts/entry.ts', source);
    const report = createMigrationReport();

    expect(migrateOxnodeToVpx(root, undefined, report)).toEqual({
      changed: true,
      dependencyRemoved: false,
    });
    expect(read('scripts/entry.ts')).toBe(
      source.replace('@oxc-node/core/register', 'vite-plus/register'),
    );
    expect(report.warnings).not.toContainEqual(expect.stringContaining('in the shebang'));
    expect(report.warnings).toContainEqual(
      expect.stringContaining('scripts/setup.js still uses oxnode'),
    );
  });

  it.each(['dependencies', 'optionalDependencies', 'peerDependencies'])(
    'preserves a runner needed in %s',
    (field) => {
      write('package.json', {
        scripts: { start: 'oxnode a.ts' },
        [field]: { '@oxc-node/cli': '^1.0.0' },
      });
      const report = createMigrationReport();
      expect(detectOxnodeProject(root)).toBe(true);
      expect(migrateOxnodeToVpx(root, undefined, report).changed).toBe(false);
      expect(readJson('package.json').scripts.start).toBe('oxnode a.ts');
      expect(report.warnings).toContainEqual(expect.stringContaining('listed in dependencies'));
    },
  );

  it('detects and reports a project with only an unsupported command', () => {
    write('package.json', {
      scripts: { dev: 'oxnode --watch a.ts' },
      devDependencies: { '@oxc-node/cli': '^1.0.0' },
    });
    const report = createMigrationReport();
    expect(detectOxnodeProject(root)).toBe(true);
    expect(migrateOxnodeToVpx(root, undefined, report).changed).toBe(false);
    expect(report.warnings).toContainEqual(expect.stringContaining('Node.js watcher'));
    expect(report.oxnodeMigrated).toBe(false);
  });

  it('detects a project that uses only the core register loader', () => {
    write('package.json', {
      scripts: { build: 'node --import @oxc-node/core/register.mjs build.ts' },
      devDependencies: { '@oxc-node/core': '^1.0.0' },
    });
    expect(detectOxnodeProject(root)).toBe(true);
    expect(migrateOxnodeToVpx(root)).toEqual({ changed: true, dependencyRemoved: true });
    expect(readJson('package.json').scripts.build).toBe(
      'node --import vite-plus/register build.ts',
    );
  });

  it.each([
    'oxnode --import ./setup.ts a.ts',
    'oxnode ./setup.ts',
    'node --import @oxc-node/core/register --loader ./setup.ts a.ts',
    'NODE_OPTIONS="--import @oxc-node/core/register --loader ./setup.ts" node a.ts',
  ])('retains %s when a named file configures the standalone loader', (command) => {
    write(
      'setup.ts',
      "import { setTsconfigPath } from '@oxc-node/core'; setTsconfigPath('cli.json');\n",
    );
    expect(rewriteOxnodeCommand(command, root)).toEqual({
      command,
      kept: [expect.stringContaining('standalone oxc-node API')],
    });
  });

  it('retains root dependencies while a workspace or nested package still uses them', () => {
    write('package.json', {
      devDependencies: { '@oxc-node/cli': '^1.0.0', '@oxc-node/core': '^1.0.0' },
    });
    write('packages/a/package.json', { scripts: { build: 'oxnode build.ts' } });
    write('packages/b/package.json', { scripts: { build: 'oxnode --watch build.ts' } });
    write('packages/a/examples/demo/package.json', {
      imports: { '#loader': '@oxc-node/core/importer' },
    });
    const packages = [
      { name: 'a', path: 'packages/a' },
      { name: 'b', path: 'packages/b' },
    ];
    expect(migrateOxnodeToVpx(root, packages).dependencyRemoved).toBe(false);
    expect(readJson('packages/a/package.json').scripts.build).toBe('vpx build.ts');
    expect(readJson('packages/b/package.json').scripts.build).toBe('oxnode --watch build.ts');
    expect(readJson('package.json').devDependencies['@oxc-node/core']).toBe('^1.0.0');
  });

  it('retains the loader referenced by an env file of a separate Node.js command', () => {
    write('package.json', {
      scripts: { build: 'oxnode build.ts', run: 'node --env-file=.env.local run.ts' },
      devDependencies: { '@oxc-node/cli': '^1.0.0', '@oxc-node/core': '^1.0.0' },
    });
    write('.env.local', 'NODE_OPTIONS=--import=@oxc-node/core/register\n');
    expect(migrateOxnodeToVpx(root).dependencyRemoved).toBe(false);
    expect(readJson('package.json').scripts).toEqual({
      build: 'vpx build.ts',
      run: 'node --env-file=.env.local run.ts',
    });
    expect(readJson('package.json').devDependencies['@oxc-node/core']).toBe('^1.0.0');
  });
});
