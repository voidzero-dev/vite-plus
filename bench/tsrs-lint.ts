// Run from the workspace root with OXLINT_TSGOLINT_PATH pointing to tsrs-lint.
// Additional arguments (e.g. --type-check or file paths) go to both vp lint runs.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const { OXLINT_TSGOLINT_PATH: tsrs, ...env } = process.env;
if (!tsrs) {
  throw new Error('Set OXLINT_TSGOLINT_PATH to the tsrs-lint executable.');
}
const vp = fileURLToPath(new URL('../packages/cli/bin/vp', import.meta.url));
const require = createRequire(new URL('../packages/cli/package.json', import.meta.url));
const tsgolint =
  process.platform === 'win32'
    ? ['tsgolint.exe', 'tsgolint.cmd']
        .map((bin) =>
          fileURLToPath(new URL(`../packages/cli/node_modules/.bin/${bin}`, import.meta.url)),
        )
        .find(existsSync)
    : require.resolve('oxlint-tsgolint/bin/tsgolint');
assert.ok(tsgolint, 'Install workspace development dependencies to get the Go baseline.');
const baselineEnv = {
  ...env,
  OXLINT_TSGOLINT_PATH: tsgolint,
};
const args = [
  vp,
  'lint',
  '--type-aware',
  '--threads',
  '4',
  '--format',
  'json',
  ...process.argv.slice(2),
];

function run(env: NodeJS.ProcessEnv) {
  const start = performance.now();
  const { status, stdout, stderr, error } = spawnSync(process.execPath, args, {
    env,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  const seconds = (performance.now() - start) / 1000;
  if (error) {
    throw error;
  }
  if ((status !== 0 && status !== 1) || !stdout.trimStart().startsWith('{')) {
    throw new Error(`vp lint exited with ${status}\n${stderr}\n${stdout}`);
  }
  const report = JSON.parse(stdout) as {
    diagnostics: unknown[];
    number_of_files: number;
    number_of_rules: number;
  };
  assert.ok(report.number_of_files > 0, 'Expected vp lint to analyze files');
  return {
    seconds,
    result: {
      status,
      files: report.number_of_files,
      rules: report.number_of_rules,
      diagnostics: report.diagnostics.map((d) => JSON.stringify(d)).toSorted(),
    },
  };
}

const backends = [
  { name: 'tsgolint', env: baselineEnv, samples: [] as number[] },
  {
    name: 'tsrs-lint',
    env: { ...baselineEnv, OXLINT_TSGOLINT_PATH: tsrs },
    samples: [] as number[],
  },
];
const [expected, candidate] = backends.map(({ env }) => run(env).result);
assert.deepEqual(candidate, expected);
const pairs = 10;
for (let pair = 0; pair < pairs; pair++) {
  for (const backend of pair % 2 === 0 ? backends : backends.toReversed()) {
    const { result, seconds } = run(backend.env);
    assert.deepEqual(result, expected);
    backend.samples.push(seconds);
  }
}
console.log(
  `${expected.files} files, ${expected.rules} rules, ${expected.diagnostics.length} diagnostics, exit ${expected.status}`,
);
for (const { name, samples } of backends) {
  const sorted = samples.toSorted((a, b) => a - b);
  const median = (sorted[pairs / 2 - 1] + sorted[pairs / 2]) / 2;
  console.log(
    `${name} median ${median.toFixed(3)} s  [${samples.map((s) => s.toFixed(3)).join(', ')}]`,
  );
}
