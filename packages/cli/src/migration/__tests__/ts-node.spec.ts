import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@voidzero-dev/vite-plus-prompts', () => ({
  log: { info: vi.fn(), success: vi.fn(), warn: vi.fn() },
}));

import {
  detectTsNodeProject,
  migrateTsNodeToVpx,
  rewriteTsNodeCommand,
} from '../migrator/ts-node.ts';
import { createMigrationReport } from '../report.ts';

describe('ts-node migration', () => {
  let root: string;
  const write = (file: string, content: string | object) => {
    const target = path.join(root, file);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, typeof content === 'string' ? content : JSON.stringify(content));
  };
  const read = (file: string) => fs.readFileSync(path.join(root, file), 'utf8');
  const readJson = (file: string) => JSON.parse(read(file));

  beforeEach(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-ts-node-'));
    vi.spyOn(process, 'cwd').mockReturnValue(root);
    write('package.json', { type: 'module' });
    for (const file of ['a.ts', 'b.ts', 'setup.ts', 'scripts/a.ts']) {
      write(file, 'export {};\n');
    }
    write('a.cts', 'exports.value = 1;\n');
    write('setup.cjs', "process.env.TS_NODE_TYPE_CHECK = 'true';\n");
  });

  afterEach(() => {
    vi.restoreAllMocks();
    fs.rmSync(root, { recursive: true, force: true });
  });

  it.each([
    ['ts-node -T a.ts --flag', 'vpx a.ts --flag'],
    ['ts-node --transpileOnly a.ts', 'vpx a.ts'],
    ['ts-node --transpile-only --esm a.ts', 'vpx a.ts'],
    ['ts-node-esm -T a.ts', 'vpx a.ts'],
    ['ts-node-transpile-only a.ts', 'vpx a.ts'],
    ['pnpm exec ts-node -T a.ts', 'vpx a.ts'],
    ['npx --yes ts-node -T a.ts', 'vpx a.ts'],
    ['npm exec -- ts-node -T a.ts', 'vpx a.ts'],
    ['yarn ts-node -T a.ts', 'vpx a.ts'],
    ['bunx ts-node -T a.ts', 'vpx a.ts'],
    ['node -r ts-node/register/transpile-only a.ts', 'node --import vite-plus/register a.ts'],
    [
      'node --require=ts-node/register/transpile-only a.cts',
      'node --import=vite-plus/register a.cts',
    ],
    ['node --loader ts-node/esm/transpile-only a.ts', 'node --import vite-plus/register a.ts'],
    [
      'node --loader=ts-node/esm/transpile-only.mjs --inspect a.ts',
      'node --import=vite-plus/register --inspect a.ts',
    ],
    [
      'node -C dev -r ts-node/register/transpile-only a.ts',
      'node -C dev --import vite-plus/register a.ts',
    ],
    [
      'node --experimental-loader ts-node/esm/transpile-only --watch a.ts',
      'node --import vite-plus/register --watch a.ts',
    ],
    [
      'node --import ts-node/register/transpile-only --test a.ts',
      'node --import vite-plus/register --test a.ts',
    ],
    [
      'TS_NODE_TRANSPILE_ONLY=1 node -r ts-node/register a.ts',
      'TS_NODE_TRANSPILE_ONLY=1 node --import vite-plus/register a.ts',
    ],
    [
      'NODE_OPTIONS="--require ts-node/register/transpile-only" node a.ts',
      'NODE_OPTIONS="--import vite-plus/register" node a.ts',
    ],
    [
      'NODE_OPTIONS=--loader=ts-node/esm/transpile-only node a.ts',
      'NODE_OPTIONS=--import=vite-plus/register node a.ts',
    ],
    [
      'NODE_OPTIONS="--trace-warnings" node -r ts-node/register/transpile-only a.ts',
      'NODE_OPTIONS="--trace-warnings" node --import vite-plus/register a.ts',
    ],
    [
      'NODE_OPTIONS="--require ts-node/register/transpile-only" concurrently "node a.ts" "node b.ts"',
      'NODE_OPTIONS="--import vite-plus/register" concurrently "node a.ts" "node b.ts"',
    ],
    [
      'cross-env-shell NODE_OPTIONS="--require ts-node/register/transpile-only" "node a.ts"',
      'cross-env-shell NODE_OPTIONS="--import vite-plus/register" "node a.ts"',
    ],
    [
      'cross-env TS_NODE_TRANSPILE_ONLY=true ts-node a.ts',
      'cross-env TS_NODE_TRANSPILE_ONLY=true vpx a.ts',
    ],
    ['ts-node -T a.ts && ts-node-transpile-only b.ts', 'vpx a.ts && vpx b.ts'],
    [
      'concurrently "ts-node -T a.ts" "ts-node-transpile-only b.ts"',
      'concurrently "vpx a.ts" "vpx b.ts"',
    ],
    ["nodemon --exec 'ts-node -T a.ts'", "nodemon --exec 'vpx a.ts'"],
    [
      'TS_NODE_TRANSPILE_ONLY=1 concurrently "ts-node a.ts"',
      'TS_NODE_TRANSPILE_ONLY=1 concurrently "vpx a.ts"',
    ],
  ])('rewrites %s and is idempotent', (command, expected) => {
    expect(rewriteTsNodeCommand(command, root)).toEqual({ command: expected, kept: [] });
    expect(rewriteTsNodeCommand(expected, root)).toEqual({ command: expected, kept: [] });
  });

  it.each([
    ['ts-node a.ts', 'type-checks'],
    ['ts-node-esm a.ts', 'type-checks'],
    ['node -r ts-node/register a.ts', 'type-checks'],
    ['ts-node -T --typeCheck a.ts', 'type-checks'],
    ['ts-node -T --typeCheck=false a.ts', 'enabled flag'],
    ['TS_NODE_TYPE_CHECK=true concurrently "ts-node -T a.ts"', 'type-checks'],
    ['TS_NODE_transpile_only=true ts-node a.ts', 'casing'],
    ['ts-node --swc a.ts', 'does not translate'],
    ['ts-node -T -C ttypescript a.ts', 'does not translate'],
    ['ts-node -T --compiler-options "{}" a.ts', 'does not translate'],
    ['ts-node -T watch a.ts', 'cannot be resolved'],
    ['ts-node -T --eval 1', 'does not translate'],
    ['ts-node -T', 'name a script'],
    ['node -r ts-node/register/transpile-only --env-file=.env a.ts', 'can configure ts-node'],
    ['node --env_file=.env -r ts-node/register/transpile-only a.ts', 'can configure ts-node'],
    ['node -r dotenv/config -r ts-node/register/transpile-only a.ts', 'additional preload'],
    [
      'NODE_OPTIONS="-r ./setup.cjs" node -r ts-node/register/transpile-only a.ts',
      'additional preload',
    ],
    ['NODE_OPTIONS="-r dotenv/config" ts-node -T a.ts', 'configuration preloads'],
    ['node -r ts-node/register/transpile-only --import ./setup.ts a.ts', 'additional preload'],
    [
      'NODE_OPTIONS=--require=ts-node/register/transpile-only ts-node -T a.ts',
      'hooks that would compete',
    ],
    ['bunx --bun ts-node -T a.ts', 'does not rewrite'],
    ['npx ts-node@10 -T a.ts', 'does not rewrite'],
    ['ts-node-cwd -T a.ts', 'does not rewrite'],
    ['ts-node -T a.ts | tee output', 'shell syntax'],
    ['ts-node -T $ENTRY', 'shell syntax'],
    ['NODE_OPTIONS="--require ts-node/register" node a.ts', 'type-checks'],
    [
      'TS_NODE_TYPE_CHECK=1 NODE_OPTIONS="--require ts-node/register/transpile-only" node a.ts',
      'type-checks',
    ],
    [
      'NODE_OPTIONS="--require ts-node/register/transpile-only" node -r ts-node/register/transpile-only a.ts',
      'more than once',
    ],
    [
      'NODE_OPTIONS="--require ts-node/register/transpile-only -r dotenv/config" node a.ts',
      'additional preload',
    ],
    [
      'NODE_OPTIONS="--require ts-node/register/transpile-only" node --env-file=.env a.ts',
      'can configure ts-node',
    ],
    [
      'TSX_TSCONFIG_PATH=other.json node -r ts-node/register/transpile-only a.ts',
      'different configuration',
    ],
    [
      'VP_SCRIPT_TSCONFIG=other.json node -r ts-node/register/transpile-only a.ts',
      'different configuration',
    ],
    [
      'OXC_TSCONFIG_PATH=other.json node -r ts-node/register/transpile-only a.ts',
      'different configuration',
    ],
    ['node -r ts-node/register/transpile-only --test', 'name a script'],
    [
      'node -r ts-node/register/transpile-only -e "require(\'ts-node\').register({ typeCheck: true })" a.ts',
      'inline code or REPL',
    ],
    ['node -r ts-node/register/transpile-only --print=1 a.ts', 'inline code or REPL'],
    ['node -r ts-node/register/transpile-only -pe1 a.ts', 'inline code or REPL'],
    ['node -r ts-node/register/transpile-only --interactive a.ts', 'inline code or REPL'],
    [
      'NODE_OPTIONS="--require ts-node/register/transpile-only" node --run check -- a.ts',
      'package script',
    ],
    ['NODE_OPTIONS="--require ts-node/register/transpile-only" mocha', 'literal node command'],
    ['NODE_OPTIONS="--require ts-node/register" concurrently npm:check', 'every launcher target'],
    [
      'NODE_OPTIONS="--require ts-node/register/transpile-only" concurrently "node a.ts" npm:check',
      'every launcher target',
    ],
    ['NODE_OPTIONS="--require ts-node/register" nodemon --exec node a.ts', 'every launcher target'],
    ['cross-env-shell NODE_OPTIONS="--require ts-node/register" node a.ts', 'type-checks'],
    ['cross-env-shell NODE_OPTIONS="--require ts-node/register" mocha', 'every launcher target'],
    ['NODE_OPTIONS="--require ts-node/register" nodemon a.ts', 'every launcher target'],
    [
      'NODE_OPTIONS="--require ts-node/register" concurrently "node a.ts" "npm run test"',
      'type-checks',
    ],
    ['cd scripts && ts-node -T a.ts', 'working directory is unknown'],
  ])('retains %s', (command, reason) => {
    expect(rewriteTsNodeCommand(command, root)).toEqual({
      command,
      kept: expect.arrayContaining([expect.stringContaining(reason)]),
    });
  });

  it('pins the CLI config and retains Node loaders that need an implicit config pin', () => {
    write('tsconfig.json', { 'ts-node': { transpileOnly: true } });
    write('scripts/tsconfig.json', {
      'ts-node': { transpileOnly: true },
      compilerOptions: { jsx: 'react' },
    });
    expect(rewriteTsNodeCommand('ts-node scripts/a.ts', root)).toEqual({
      command: 'vpx --tsconfig=scripts/tsconfig.json scripts/a.ts',
      kept: [],
    });
    for (const loader of [
      '--require ts-node/register',
      '--require=ts-node/register/transpile-only',
    ]) {
      const command = `node ${loader} scripts/a.ts`;
      expect(rewriteTsNodeCommand(command, root)).toEqual({
        command,
        kept: [expect.stringContaining('set TS_NODE_PROJECT')],
      });
    }
  });

  it('keeps explicit TS_NODE_PROJECT files for Node loaders and literal NODE_OPTIONS', () => {
    write('tsconfig.json', { 'ts-node': { typeCheck: true } });
    write('configs/runner.json', { 'ts-node': { transpileOnly: true } });
    for (const [before, after] of [
      ['node -r ts-node/register a.ts', 'node --import vite-plus/register a.ts'],
      [
        'NODE_OPTIONS="--require ts-node/register" node a.ts',
        'NODE_OPTIONS="--import vite-plus/register" node a.ts',
      ],
    ]) {
      const prefix = 'TS_NODE_PROJECT=configs/runner.json ';
      expect(rewriteTsNodeCommand(prefix + before, root)).toEqual({
        command: prefix + after,
        kept: [],
      });
    }
  });

  it.each([
    'node -r ts-node/register/transpile-only scripts/a.ts',
    'NODE_OPTIONS="--require ts-node/register/transpile-only" node scripts/a.ts',
  ])('retains %s when the public preload would discover a new entrypoint config', (command) => {
    write('scripts/tsconfig.json', { compilerOptions: { useDefineForClassFields: true } });
    expect(rewriteTsNodeCommand(command, root)).toEqual({
      command,
      kept: [expect.stringContaining('an entrypoint tsconfig that ts-node does not use')],
    });
  });

  it('removes ts-node when all Node.js loader and NODE_OPTIONS uses migrate', () => {
    write('package.json', {
      scripts: {
        seed: 'node -r ts-node/register/transpile-only a.ts',
        test: 'NODE_OPTIONS="--loader=ts-node/esm/transpile-only" node --test b.ts',
      },
      devDependencies: { 'ts-node': '^10.9.2' },
    });
    expect(migrateTsNodeToVpx(root)).toEqual({ changed: true, dependencyRemoved: true });
    expect(readJson('package.json').scripts).toEqual({
      seed: 'node --import vite-plus/register a.ts',
      test: 'NODE_OPTIONS="--import=vite-plus/register" node --test b.ts',
    });
    expect(readJson('package.json').devDependencies).toEqual({});
    expect(migrateTsNodeToVpx(root)).toEqual({ changed: false, dependencyRemoved: false });
  });

  it('translates explicit project paths with spaces using portable quotes', () => {
    write('config files/tsconfig.json', { 'ts-node': { transpileOnly: true } });
    expect(rewriteTsNodeCommand('ts-node -P "config files" a.ts', root)).toEqual({
      command: 'vpx "--tsconfig=config files/tsconfig.json" a.ts',
      kept: [],
    });
    expect(
      rewriteTsNodeCommand('ts-node --project="config files/tsconfig.json" a.ts', root),
    ).toEqual({
      command: 'vpx "--tsconfig=config files/tsconfig.json" a.ts',
      kept: [],
    });
  });

  it('retains generated config arguments that require platform-specific escaping', () => {
    write("config's.json", { 'ts-node': { transpileOnly: true } });
    const command = 'ts-node -P "config\'s.json" a.ts';
    expect(rewriteTsNodeCommand(command, root)).toEqual({
      command,
      kept: [expect.stringContaining('shell escaping')],
    });
  });

  it.each([
    ['a.cts', 'export const value = 1;\n', 'does not compile to CommonJS'],
    ['a.ts', "import fs from 'node:fs'; console.log(__dirname);\n", 'CommonJS globals'],
    [
      'setup.ts',
      "import { register } from 'ts-node'; register({ typeCheck: true });\n",
      'standalone ts-node API',
    ],
  ])('checks the source of %s before replacing its loader', (file, content, reason) => {
    write('package.json', {});
    write(file, content);
    const command = `ts-node -T ${file}`;
    expect(rewriteTsNodeCommand(command, root)).toEqual({
      command,
      kept: [expect.stringContaining(reason)],
    });
  });

  it('migrates scripts, staged tasks, and config-free shebangs and removes only ts-node', () => {
    write('package.json', {
      scripts: { seed: 'ts-node -T a.ts' },
      'lint-staged': { '*.ts': 'ts-node-transpile-only scripts/a.ts' },
      devDependencies: { 'ts-node': '^10.9.2', typescript: '^6.0.0' },
    });
    write('.lintstagedrc.json', { '*.md': 'ts-node -T b.ts' });
    write('scripts/release.ts', '#!/usr/bin/env -S ts-node -T\nexport {};\n');
    write('scripts/direct.ts', '#!/usr/bin/env ts-node-transpile-only\nexport {};\n');
    const report = createMigrationReport();
    expect(detectTsNodeProject(root)).toBe(true);
    expect(migrateTsNodeToVpx(root, undefined, report)).toEqual({
      changed: true,
      dependencyRemoved: true,
    });
    expect(readJson('package.json')).toEqual({
      scripts: { seed: 'vpx a.ts' },
      'lint-staged': { '*.ts': 'vpx scripts/a.ts' },
      devDependencies: { typescript: '^6.0.0' },
    });
    expect(readJson('.lintstagedrc.json')).toEqual({ '*.md': 'vpx b.ts' });
    expect(read('scripts/release.ts')).toBe('#!/usr/bin/env -S vpx\nexport {};\n');
    expect(read('scripts/direct.ts')).toBe('#!/usr/bin/env vpx\nexport {};\n');
    expect(report.tsNodeMigrated).toBe(true);
    expect(report.warnings).toEqual([]);
    expect(detectTsNodeProject(root)).toBe(false);
    expect(migrateTsNodeToVpx(root)).toEqual({ changed: false, dependencyRemoved: false });
  });

  it('keeps shebangs whose selected config cannot be pinned independently of caller cwd', () => {
    write('package.json', { devDependencies: { 'ts-node': '^10.9.2' } });
    write('scripts/tsconfig.json', { 'ts-node': { transpileOnly: true } });
    const source = '#!/usr/bin/env -S ts-node -T\nexport {};\n';
    write('scripts/release.ts', source);
    const report = createMigrationReport();
    expect(migrateTsNodeToVpx(root, undefined, report)).toEqual({
      changed: false,
      dependencyRemoved: false,
    });
    expect(read('scripts/release.ts')).toBe(source);
    expect(report.warnings).toContainEqual(expect.stringContaining('pinned tsconfig'));
  });

  it.each([
    ['src/api.ts', "import { register } from 'ts-node';\n"],
    ['dist/api.js', "require('ts-node/register/transpile-only');\n"],
    ['src/spawn.ts', "spawn('ts-node-esm', ['a.ts']);\n"],
    ['configs/legacy.json', '{"extends":"ts-node/node16/tsconfig.json"}'],
    ['.env.local', 'NODE_OPTIONS=--require=ts-node/register\n'],
    ['.github/workflows/ci.yml', 'jobs:\n  run: ts-node scripts/ci.ts\n'],
    ['.husky/pre-commit', 'ts-node -T scripts/check.ts\n'],
    ['scripts/release.sh', 'ts-node -T a.ts\n'],
    ['.mocharc.json', '{"require":"ts-node/register"}'],
    ['lint-staged.config.ts', "export default { '*.ts': 'ts-node -T a.ts' };\n"],
  ])('retains the dependency used by %s', (file, content) => {
    write('package.json', {
      scripts: { seed: 'ts-node -T a.ts' },
      devDependencies: { 'ts-node': '^10.9.2' },
    });
    write(file, content);
    const report = createMigrationReport();
    expect(migrateTsNodeToVpx(root, undefined, report).dependencyRemoved).toBe(false);
    expect(readJson('package.json').scripts.seed).toBe('vpx a.ts');
    expect(readJson('package.json').devDependencies['ts-node']).toBe('^10.9.2');
    expect(report.warnings).toContainEqual(expect.stringContaining(`${file} still uses ts-node`));
  });

  it.each(['dependencies', 'optionalDependencies', 'peerDependencies'])(
    'preserves scripts that need ts-node at runtime through %s',
    (field) => {
      write('package.json', {
        scripts: { seed: 'ts-node -T a.ts' },
        [field]: { 'ts-node': '^10.9.2' },
      });
      expect(detectTsNodeProject(root)).toBe(true);
      expect(migrateTsNodeToVpx(root).changed).toBe(false);
      expect(readJson('package.json').scripts.seed).toBe('ts-node -T a.ts');
    },
  );

  it('reports projects that contain only type-checking commands', () => {
    write('package.json', {
      scripts: { seed: 'ts-node a.ts' },
      devDependencies: { 'ts-node': '^10.9.2' },
    });
    const report = createMigrationReport();
    expect(detectTsNodeProject(root)).toBe(true);
    expect(migrateTsNodeToVpx(root, undefined, report).changed).toBe(false);
    expect(report.tsNodeMigrated).toBe(false);
    expect(report.warnings).toContainEqual(expect.stringContaining('type-checks'));
  });

  it('retains workspace and nested-package dependencies until all their users migrate', () => {
    write('package.json', { devDependencies: { 'ts-node': '^10.9.2' } });
    write('packages/a/package.json', {
      scripts: { seed: 'ts-node -T a.ts' },
      devDependencies: { 'ts-node': '^10.9.2' },
    });
    write('packages/a/a.ts', 'export {};\n');
    write('packages/a/examples/package.json', { imports: { '#loader': 'ts-node/register' } });
    write('packages/b/package.json', { scripts: { seed: 'ts-node a.ts' } });
    write('packages/b/a.ts', 'export {};\n');
    const packages = [
      { name: 'a', path: 'packages/a' },
      { name: 'b', path: 'packages/b' },
    ];
    expect(migrateTsNodeToVpx(root, packages).dependencyRemoved).toBe(false);
    expect(readJson('packages/a/package.json').scripts.seed).toBe('vpx a.ts');
    expect(readJson('packages/b/package.json').scripts.seed).toBe('ts-node a.ts');
    expect(readJson('packages/a/package.json').devDependencies['ts-node']).toBe('^10.9.2');
    expect(readJson('package.json').devDependencies['ts-node']).toBe('^10.9.2');
  });
});
