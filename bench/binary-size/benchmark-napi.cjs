// Run each candidate in a separate Node process. Output hashes must match the
// baseline. These synthetic kernels are a regression screen, not an application
// build benchmark; do not use them alone to approve a release profile change.
const { createHash } = require('node:crypto');
const path = require('node:path');
const { performance } = require('node:perf_hooks');

const binding = require(path.resolve(process.argv[2]));
const source = Array.from(
  { length: 1500 },
  (_, i) =>
    `export function compute${i}(value) { const offset = ${i}; return value?.amount ?? offset + 1; }`,
).join('\n');
const typescript = Array.from(
  { length: 1500 },
  (_, i) =>
    `export const compute${i} = (value: { amount?: number } | undefined): number => value?.amount ?? ${i};`,
).join('\n');

const tasks = {
  parse: () => {
    const result = binding.parseSync('fixture.js', source);
    return { errors: result.errors, module: result.module };
  },
  transform: () =>
    binding.transformSync('fixture.ts', typescript, { target: 'es2015', sourcemap: true }),
  minify: () => binding.minifySync('fixture.js', source, { sourcemap: true }),
  invalidSyntax: () => binding.transformSync('invalid.ts', 'export const value: = ;'),
};

const results = {};
for (const [name, run] of Object.entries(tasks)) {
  for (let i = 0; i < 5; i++) {
    run();
  }
  const samples = [];
  for (let i = 0; i < 25; i++) {
    const start = performance.now();
    run();
    samples.push(performance.now() - start);
  }
  const output = JSON.stringify(run());
  results[name] = {
    median_ms: samples.toSorted((a, b) => a - b)[Math.floor(samples.length / 2)],
    samples_ms: samples,
    output_sha256: createHash('sha256').update(output).digest('hex'),
    output_bytes: Buffer.byteLength(output),
  };
}
process.stdout.write(
  JSON.stringify(
    { node: process.version, source_bytes: Buffer.byteLength(source), results },
    null,
    2,
  ) + '\n',
);
