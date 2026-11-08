// Compare two built CLI packages without a transform or Node compile cache.
// node bench/vpx-resolve.mjs --before <package> --after <package> --output <new-dir>
// Optional: --knip <checkout> --marked <checkout> --runs 40
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import {
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  realpathSync,
  writeFileSync,
} from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

const { values } = parseArgs({
  options: {
    before: { type: 'string' },
    after: { type: 'string' },
    output: { type: 'string' },
    knip: { type: 'string' },
    marked: { type: 'string' },
    runs: { type: 'string', default: '40' },
  },
});
assert(
  values.before && values.after && values.output,
  'Pass --before, --after and a new --output directory',
);
assert(Boolean(values.knip) === Boolean(values.marked), 'Pass both --knip and --marked');
const runs = Number(values.runs);
assert(Number.isSafeInteger(runs) && runs > 0);
mkdirSync(path.resolve(values.output), { recursive: false, mode: 0o700 });
const output = realpathSync(values.output);
for (let dir = output; ; dir = path.dirname(dir)) {
  assert(
    !lstatSync(path.join(dir, 'tsconfig.json'), { throwIfNoEntry: false }),
    'Choose an output directory without an ancestor tsconfig.json',
  );
  if (dir === path.dirname(dir)) {
    break;
  }
}
const packages = { before: realpathSync(values.before), after: realpathSync(values.after) };
const digest = (file) => createHash('sha256').update(readFileSync(file)).digest('hex');
const hashes = Object.fromEntries(
  Object.entries(packages).map(([label, dir]) => [
    label,
    Object.fromEntries(
      [
        'dist/script-preload.cjs',
        'dist/script-register.js',
        'dist/script-esm-hooks.js',
        'binding/index.cjs',
        ...readdirSync(path.join(dir, 'binding'))
          .filter((name) => name.endsWith('.node'))
          .map((name) => 'binding/' + name),
      ].map((name) => [name, digest(path.join(dir, name))]),
    ),
  ]),
);
for (const name of [
  'dist/script-preload.cjs',
  'dist/script-register.js',
  'dist/script-esm-hooks.js',
  'binding/index.cjs',
]) {
  assert.equal(
    hashes.before[name],
    hashes.after[name],
    'Only the native implementation may differ',
  );
}
const probe = path.join(output, 'node_modules/vpx-resolve-probe/probe.cjs');
mkdirSync(path.dirname(probe), { recursive: true });
writeFileSync(
  probe,
  `const fs = require('node:fs');
process.once('exit', () => {
  const binding = require(process.env.BENCH_BINDING).oxcNode;
  fs.writeSync(3, JSON.stringify({
    token: process.env.BENCH_TOKEN, node: process.versions.node, execPath: process.execPath,
    compileCache: require('node:module').getCompileCacheDir() ?? null,
    transformCacheAPI: typeof binding.cacheStats,
  }));
});\n`,
);

function moduleSource(i) {
  return `export interface Item${i} {
  id: number;
  name: string;
}

export enum Kind${i} {
  A = 'a${i}',
  B = 'b${i}',
}

export class Store${i} {
  items: Item${i}[] = [];

  constructor(private readonly prefix: string) {}

  add(id: number): Item${i} {
    const item = { id, name: \`\${this.prefix}-\${id}\` };
    this.items.push(item);
    return item;
  }
}

export function run${i}(): number {
  return new Store${i}(Kind${i}.A).add(${i}).id;
}
`;
}
const cases = [300, 1000, 10000].map((count) => {
  const cwd = path.join(output, 'fixtures', String(count));
  mkdirSync(cwd, { recursive: true });
  writeFileSync(path.join(cwd, 'package.json'), '{ "type": "module" }\n');
  for (let i = 0; i < count; i++) {
    writeFileSync(path.join(cwd, `mod${i}.ts`), moduleSource(i));
  }
  const entry = path.join(cwd, 'index.ts');
  writeFileSync(
    entry,
    [
      ...Array.from({ length: count }, (_, i) => `import { run${i} } from './mod${i}.ts';`),
      'let total = 0;',
      ...Array.from({ length: count }, (_, i) => `total += run${i}();`),
      "console.log('total', total);\n",
    ].join('\n'),
  );
  return {
    name: `${count} ESM modules`,
    cwd,
    entry,
    args: [],
    expected: `total ${(count * (count - 1)) / 2}\n`,
  };
});
if (values.knip) {
  const cwd = realpathSync(values.knip);
  cases.push({
    name: 'Knip scanning Marked',
    cwd,
    entry: path.join(cwd, 'packages/knip/src/cli.ts'),
    args: [
      '--directory',
      realpathSync(values.marked),
      '--production',
      '--include',
      'files,exports',
      '--reporter',
      'json',
      '--no-exit-code',
    ],
    json: true,
  });
}
const cleanEnv = Object.fromEntries(
  Object.entries(process.env).filter(
    ([key]) => !/^(?:VP|TSX_|TS_NODE_|OXC_|BENCH_|NODE_|NAPI_RS_|DYLD_)/i.test(key),
  ),
);
const report = {
  timestamp: new Date().toISOString(),
  node: process.execPath,
  version: process.versions.node,
  packages,
  hashes,
  cases,
  runs,
  metric:
    'Fresh process launch through exit; synthetic application or complete Knip scan runs every time',
  rows: [],
};
const file = path.join(output, 'results.json');
function normalize(test, text) {
  if (!test.json) {
    return text;
  }
  const value = JSON.parse(text);
  assert(Array.isArray(value.issues));
  assert.equal(new Set(value.issues.map((issue) => issue.file)).size, value.issues.length);
  value.issues.sort((a, b) => a.file.localeCompare(b.file));
  return value;
}
async function launch(test, label, phase, block) {
  const pkg = packages[label];
  const token = randomUUID();
  const args = [
    '--require',
    path.join(pkg, 'dist/script-preload.cjs'),
    '--require',
    probe,
    '--import',
    pathToFileURL(path.join(pkg, 'dist/script-register.js')).href,
    test.entry,
    ...test.args,
  ];
  const env = {
    ...cleanEnv,
    NODE_DISABLE_COMPILE_CACHE: '1',
    VPX_DISABLE_CACHE: '1',
    BENCH_BINDING: path.join(pkg, 'binding/index.cjs'),
    BENCH_TOKEN: token,
  };
  const start = performance.now();
  const child = spawn(process.execPath, args, {
    cwd: test.cwd,
    env,
    stdio: ['ignore', 'pipe', 'pipe', 'pipe'],
  });
  let stdout = '',
    stderr = '',
    identity = '';
  child.stdout.setEncoding('utf8');
  child.stderr.setEncoding('utf8');
  child.stdio[3].setEncoding('utf8');
  child.stdout.on('data', (part) => {
    stdout += part;
  });
  child.stderr.on('data', (part) => {
    stderr += part;
  });
  child.stdio[3].on('data', (part) => {
    identity += part;
  });
  const timer = setTimeout(() => child.kill('SIGKILL'), 45_000);
  let code, signal;
  try {
    [code, signal] = await new Promise((resolve, reject) => {
      child.once('error', reject);
      child.once('close', (code, signal) => resolve([code, signal]));
    });
  } finally {
    clearTimeout(timer);
  }
  const ms = performance.now() - start;
  assert.equal(signal, null, stderr);
  assert.equal(code, 0, stderr);
  assert.equal(stderr, '');
  const observed = JSON.parse(identity);
  assert.equal(observed.token, token);
  assert.equal(observed.node, process.versions.node);
  assert.equal(realpathSync(observed.execPath), realpathSync(process.execPath));
  assert.equal(observed.compileCache, null);
  assert.equal(observed.transformCacheAPI, 'undefined');
  const result = normalize(test, stdout);
  test.expected ??= result;
  assert.deepEqual(result, test.expected);
  report.rows.push({ case: test.name, label, phase, block, ms });
  writeFileSync(file, JSON.stringify(report, null, 2));
  process.stdout.write(`${test.name} ${label} ${phase} ${block} ${ms.toFixed(1)} ms\n`);
}
for (const test of cases) {
  for (let i = 0; i < 3; i++) {
    for (const label of ['before', 'after']) {
      await launch(test, label, 'prime', i);
    }
  }
}
// Alternate both orders equally to limit drift from shared OS caches or temperature.
for (let block = 0; block < runs; block++) {
  for (const test of cases) {
    for (const label of block % 2 ? ['after', 'before'] : ['before', 'after']) {
      await launch(test, label, 'warm', block);
    }
  }
}
const median = (values) => {
  const sorted = values.toSorted((a, b) => a - b);
  return (sorted[Math.floor(sorted.length / 2)] + sorted[Math.floor((sorted.length - 1) / 2)]) / 2;
};
report.medians = cases.map((test) => {
  const row = { case: test.name };
  for (const label of ['before', 'after']) {
    row[label] = median(
      report.rows
        .filter((r) => r.case === test.name && r.label === label && r.phase === 'warm')
        .map((r) => r.ms),
    );
  }
  row.reductionPercent = (1 - row.after / row.before) * 100;
  return row;
});
writeFileSync(file, JSON.stringify(report, null, 2));
process.stdout.write(JSON.stringify(report.medians, null, 2) + '\n');
