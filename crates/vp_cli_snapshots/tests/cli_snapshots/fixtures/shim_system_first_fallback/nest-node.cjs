// Re-run this script through the `node` shim on PATH with the same arguments
// and cwd until NEST_DEPTH runs out, as a legitimate nested invocation would.
const { spawnSync } = require('node:child_process');
const path = require('node:path');

const depth = Number(process.env.NEST_DEPTH);
if (!Number.isInteger(depth) || depth < 1) {
  console.error(`Invalid NEST_DEPTH: ${process.env.NEST_DEPTH}`);
  process.exit(1);
}
if (depth === 1) {
  console.log('NESTED_NODE_OK');
  process.exit(0);
}
// Node resolves the script to an absolute path; pass it as the step did.
const script = path.relative(process.cwd(), process.argv[1]);
const child = spawnSync('node', [script, ...process.argv.slice(2)], {
  env: { ...process.env, NEST_DEPTH: String(depth - 1) },
  stdio: 'inherit',
});
if (child.error) {
  console.error(child.error.message);
}
process.exit(child.status ?? 1);
