// Run a Vite+ shim by path: with a managed runtime first on PATH, the runner
// would otherwise resolve the step's command to the managed tool itself.
const { spawnSync } = require('node:child_process');
const path = require('node:path');

const [tool, ...args] = process.argv.slice(2);
const child = spawnSync(path.join(process.env.VP_HOME, 'bin', tool), args, { stdio: 'inherit' });
process.exit(child.status ?? 1);
