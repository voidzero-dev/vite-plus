// A system npm that reports which Node.js runs it. `npm nested` runs
// `npm --version` again from PATH, as a lifecycle script would: the system npm
// answers with a marker, while the npm bundled with the seeded managed runtime
// prints its version.
const { chmodSync, writeFileSync } = require('node:fs');
const path = require('node:path');

const npm = path.join('system-bin', 'npm');
writeFileSync(
  npm,
  `#!/usr/bin/env node
const node = process.execPath.includes('system-bin')
  ? 'system'
  : process.execPath.includes('js_runtime')
    ? 'managed'
    : 'other';
if (process.argv[2] !== 'nested') {
  console.log(\`system npm child on \${node} Node.js\`);
  process.exit(0);
}
console.log(\`system npm on \${node} Node.js\`);
const child = require('node:child_process').spawnSync('npm', ['--version'], { stdio: 'inherit' });
process.exit(child.status ?? 1);
`,
);
chmodSync(npm, 0o755);
